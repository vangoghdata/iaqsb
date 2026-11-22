//! Flujo en redes con costes (transbordo), resuelto con HiGHS. El paso a paso
//! muestra el modelo lineal equivalente: conservación del flujo en cada nodo.

use super::layout::node_network;
use super::{NetModel, NetOptions};
use iaqsb_core::lp::{self, Lp, LpStatus, Sense};
use iaqsb_core::report::*;
use iaqsb_core::{cells, ModError};
use serde_json::json;

pub fn solve(m: &NetModel, o: &NetOptions) -> Result<Report, ModError> {
    if super::flowx::any_extended(m) {
        return super::flowx::solve(m, o);
    }
    let n = m.nodes.len();
    let net: Vec<f64> = m.nodes.iter().map(|x| x.supply - x.demand).collect();
    let total: f64 = net.iter().sum();
    let mut lpm = Lp::new(0, m.objective.is_max());
    let mut arcs = vec![];
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            let Some(c) = m.arc(i, j) else { continue };
            let b = m.bounds.iter().find(|b| b.from.eq_ignore_ascii_case(&m.nodes[i].name) && b.to.eq_ignore_ascii_case(&m.nodes[j].name));
            let (lo, up) = b.map(|b| (b.lower, b.upper.unwrap_or(f64::INFINITY))).unwrap_or((0.0, f64::INFINITY));
            if lo > up {
                return Err(ModError::Invalid(format!("la cota mínima de {} → {} supera la máxima", m.nodes[i].name, m.nodes[j].name)));
            }
            let k = lpm.add_var(c, lo, up, false);
            arcs.push((i, j, c, k, lo, up));
        }
    }
    // conservación: sale − entra = oferta − demanda (con exceso de oferta, ≤ en los nodos con oferta)
    let mut forms = vec![];
    for v in 0..n {
        let mut coefs = vec![];
        for &(i, j, _, k, _, _) in &arcs {
            if i == v {
                coefs.push((k, 1.0));
            }
            if j == v {
                coefs.push((k, -1.0));
            }
        }
        let sense = if total > 1e-9 && net[v] > 0.0 {
            Sense::Le
        } else if total < -1e-9 && net[v] < 0.0 {
            Sense::Ge
        } else {
            Sense::Eq
        };
        forms.push((v, sense));
        lpm.row(coefs, sense, net[v]);
    }
    let s = lp::solve(&lpm);
    let what = if m.objective.is_max() { "Beneficio" } else { "Costo" };
    let formulation = || {
        let mut t = Table::new(Some("Conservación del flujo"), &["Nodo", "Sale − entra", "Relación", "Oferta − demanda"]).left(1).left(2);
        for &(v, sense) in &forms {
            let outs: Vec<String> = arcs.iter().filter(|a| a.0 == v).map(|a| format!("x({},{})", m.nodes[a.0].name, m.nodes[a.1].name)).collect();
            let ins: Vec<String> = arcs.iter().filter(|a| a.1 == v).map(|a| format!("x({},{})", m.nodes[a.0].name, m.nodes[a.1].name)).collect();
            let expr = format!(
                "{}{}",
                if outs.is_empty() { "0".to_string() } else { outs.join(" + ") },
                if ins.is_empty() { String::new() } else { format!(" − ({})", ins.join(" + ")) }
            );
            let rel = match sense {
                Sense::Le => "≤",
                Sense::Ge => "≥",
                Sense::Eq => "=",
            };
            t.row(cells![&m.nodes[v].name, expr, rel, net[v]]);
        }
        t.block()
    };
    match s.status {
        LpStatus::Optimal => {}
        LpStatus::Infeasible => {
            // sin una solución que lo envíe todo: se entrega lo máximo posible al menor costo
            return super::flowx::solve(m, o);
        }
        LpStatus::Unbounded => {
            return Ok(Report::new("net", &m.name, Status::Unbounded, "El problema no está acotado: hay un ciclo que mejora el objetivo sin límite.")
                .section("solucion", "Solución", vec![text_tone("Algún ciclo de arcos sin cota superior mejora el objetivo indefinidamente.", Tone::Bad)]));
        }
        LpStatus::Error(e) => return Err(ModError::Invalid(e)),
    }
    let mut t = Table::new(None, &["Desde", "Hasta", "Flujo", "Costo unitario", "Total", "Mínimo", "Máximo"]).left(1);
    let mut used = vec![];
    let mut flows = vec![];
    for &(i, j, c, k, lo, up) in &arcs {
        let f = clean(s.x[k]);
        if f.abs() > 1e-9 {
            used.push((i, j));
            flows.push(json!({"from": m.nodes[i].name, "to": m.nodes[j].name, "flow": f}));
            t.row_tone(cells![&m.nodes[i].name, &m.nodes[j].name, f, c, f * c, lo, up], Tone::Accent);
        }
    }
    t.row_tone(cells!["Total", "", "", "", s.objective, "", ""], Tone::Total);
    let mut potentials = Table::new(Some("Valor dual de cada nodo"), &["Nodo", "Oferta − demanda", "Valor dual"]).left(0);
    if let Some(d) = &s.duals {
        for v in 0..n {
            potentials.row(cells![&m.nodes[v].name, net[v], clean(d[v])]);
        }
    }
    let xs = s.x.clone();
    let arcs2 = arcs.clone();
    let netb = node_network(
        m,
        "Flujo óptimo",
        0,
        &used,
        &|i, j, c| {
            let f = arcs2.iter().find(|a| a.0 == i && a.1 == j).map(|a| xs[a.3]).unwrap_or(0.0);
            Some(if f.abs() > 1e-9 { format!("{} (c {})", fmt(f), fmt(c)) } else { format!("c {}", fmt(c)) })
        },
        &|v| {
            let nv = net[v];
            if nv > 0.0 {
                Some(format!("oferta {}", fmt(nv)))
            } else if nv < 0.0 {
                Some(format!("demanda {}", fmt(-nv)))
            } else {
                None
            }
        },
        false,
    );
    let mut rep = Report::new("net", &m.name, Status::Optimal, format!("{what} total óptimo del flujo: {}.", fmt(s.objective)))
        .section("solucion", "Solución", vec![metrics(vec![metric_main(format!("{what} total"), s.objective), metric("Arcos con flujo", used.len())]), t.block()])
        .section("red", "Red", vec![netb]);
    if s.duals.is_some() {
        rep = rep.section("analisis", "Análisis", vec![potentials.block()]);
    }
    if o.steps {
        rep = rep.section(
            "pasos",
            "Modelo",
            vec![
                text("El flujo en redes es un programa lineal: una variable x(i,j) por arco, el costo como objetivo y una ecuación de conservación por nodo. Se resuelve con HiGHS."),
                formulation(),
            ],
        );
    }
    Ok(rep.data(json!({"objective_value": clean(s.objective), "flows": flows, "lp_model": super::flowx::lp_model(m)})))
}
