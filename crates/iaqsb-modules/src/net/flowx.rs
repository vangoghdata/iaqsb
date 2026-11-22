//! Flujo en redes con los datos de nodo que se ven en clase y que WinQSB no
//! tiene: costo y capacidad en los centros de transbordo, demandas con mínimo
//! y máximo ("compra al menos 700", "compra máximo 800") y ofertas que no
//! alcanzan. Se arma el programa lineal directamente:
//!
//! - x(i,j): unidades por cada arco, con su costo y sus cotas.
//! - T(v): unidades que pasan por un nodo con costo o capacidad. Es el arco
//!   ficticio de la clase ("Paso 1: el nodo se convierte en un flujo").
//! - Conservación en cada nodo: lo que sale de un origen no supera su oferta,
//!   lo que recibe un destino queda entre su mínimo y su máximo y los nodos de
//!   transbordo envían todo lo que reciben.
//!
//! Si la oferta o las capacidades no alcanzan para las demandas, primero se
//! entrega lo máximo posible y, con eso fijo, se busca el menor costo (como en
//! el caso 3 de la clase, donde se reporta el faltante de cada mercado).
//!
//! También da el programa lineal equivalente (para abrirlo en el módulo de
//! programación lineal o en WinQSB) y la red equivalente sin datos de nodo
//! (para guardarla en un archivo .NET que WinQSB pueda abrir).

use super::layout::node_network;
use super::{ArcBound, NetModel, NetNode, NetOptions};
use iaqsb_core::linear::Bnd;
use iaqsb_core::lp::{self, Lp, LpStatus, Sense};
use iaqsb_core::report::*;
use iaqsb_core::{cells, ModError};
use serde_json::{json, Value};

const EPS: f64 = 1e-7;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Source,
    Sink,
    Transit,
}

/// El modelo armado (variables, filas y nombres), con o sin faltantes.
struct Built {
    lp: Lp,
    /// (i, j, costo, variable, mínimo, máximo)
    arcs: Vec<(usize, usize, f64, usize, f64, f64)>,
    /// (nodo, variable, costo, capacidad, "entra" o "sale")
    through: Vec<(usize, usize, f64, f64, &'static str)>,
    /// (nodo, variable) del faltante de cada destino
    short: Vec<(usize, usize)>,
    var_names: Vec<String>,
    /// (nombre, términos, relación, lado derecho)
    rows: Vec<(String, Vec<(usize, f64)>, Sense, f64)>,
}

fn kind(n: &NetNode) -> Kind {
    let net = n.supply - n.demand;
    if net > EPS {
        Kind::Source
    } else if net < -EPS || n.max_demand.is_some() {
        Kind::Sink
    } else {
        Kind::Transit
    }
}

/// Nombre corto sin espacios para las variables (X1A, X35...).
fn short_name(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric() || *c == '_').collect()
}

fn var_names(m: &NetModel) -> Vec<String> {
    m.nodes.iter().map(|n| short_name(&n.name)).collect()
}

fn bound_of(m: &NetModel, i: usize, j: usize) -> (f64, f64) {
    let b = m
        .bounds
        .iter()
        .find(|b: &&ArcBound| b.from.eq_ignore_ascii_case(&m.nodes[i].name) && b.to.eq_ignore_ascii_case(&m.nodes[j].name));
    b.map(|b| (b.lower, b.upper.unwrap_or(f64::INFINITY))).unwrap_or((0.0, f64::INFINITY))
}

fn build(m: &NetModel, with_short: bool) -> Result<Built, ModError> {
    let n = m.nodes.len();
    let nm = var_names(m);
    let mut lpm = Lp::new(0, m.objective.is_max());
    let mut names = vec![];
    let mut arcs = vec![];
    // nombres únicos: si X + origen + destino se repite, se usan los números de nodo
    let mut seen = std::collections::HashSet::new();
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            let Some(c) = m.arc(i, j) else { continue };
            let (lo, up) = bound_of(m, i, j);
            if lo > up {
                return Err(ModError::Invalid(format!("la cota mínima de {} → {} supera la máxima", m.nodes[i].name, m.nodes[j].name)));
            }
            let k = lpm.add_var(c, lo, up, false);
            let long = nm.iter().any(|x| x.chars().count() > 1);
            let mut name = if long { format!("X{}_{}", nm[i], nm[j]) } else { format!("X{}{}", nm[i], nm[j]) };
            if name.chars().count() > 12 || !seen.insert(name.to_uppercase()) {
                name = format!("X{}_{}", i + 1, j + 1);
                seen.insert(name.to_uppercase());
            }
            names.push(name);
            arcs.push((i, j, c, k, lo, up));
        }
    }
    let mut through = vec![];
    for (v, nd) in m.nodes.iter().enumerate() {
        let h = nd.handling_cost.unwrap_or(0.0);
        if nd.capacity.is_none() && h == 0.0 {
            continue;
        }
        let cap = nd.capacity.unwrap_or(f64::INFINITY);
        if cap < 0.0 {
            return Err(ModError::Invalid(format!("la capacidad de {} no puede ser negativa", nd.name)));
        }
        let k = lpm.add_var(h, 0.0, cap, false);
        names.push(format!("T{}", nm[v]));
        through.push((v, k, h, cap, if kind(nd) == Kind::Source { "sale" } else { "entra" }));
    }
    let mut short = vec![];
    if with_short {
        for (v, nd) in m.nodes.iter().enumerate() {
            if kind(nd) == Kind::Sink && nd.demand - nd.supply > EPS {
                let k = lpm.add_var(0.0, 0.0, f64::INFINITY, false);
                names.push(format!("F{}", nm[v]));
                short.push((v, k));
            }
        }
    }

    let mut rows: Vec<(String, Vec<(usize, f64)>, Sense, f64)> = vec![];
    let balance = |v: usize, sign: f64| -> Vec<(usize, f64)> {
        let mut t = vec![];
        for &(i, j, _, k, _, _) in &arcs {
            if i == v {
                t.push((k, sign));
            }
            if j == v {
                t.push((k, -sign));
            }
        }
        t
    };
    for (v, nd) in m.nodes.iter().enumerate() {
        let net = nd.supply - nd.demand;
        let label = &nd.name;
        match kind(nd) {
            Kind::Source => rows.push((format!("Oferta de {label}"), balance(v, 1.0), Sense::Le, net)),
            Kind::Transit => rows.push((format!("Transbordo en {label}"), balance(v, 1.0), Sense::Eq, 0.0)),
            Kind::Sink => {
                let need = -net;
                let mut t = balance(v, -1.0);
                if let Some(&(_, k)) = short.iter().find(|s| s.0 == v) {
                    t.push((k, 1.0));
                }
                let max = match nd.max_demand {
                    None => Some(need),
                    Some(Bnd(x)) if x.is_finite() => Some(x - nd.supply),
                    Some(_) => None,
                };
                if need > EPS || max.is_none() || max.is_some_and(|x| x > need + EPS) {
                    // mínimo (y máximo aparte) o demanda exacta
                    if max.is_some_and(|x| (x - need).abs() <= EPS) {
                        rows.push((format!("Demanda de {label}"), t, Sense::Eq, need));
                    } else {
                        if need > EPS {
                            rows.push((format!("Demanda mínima de {label}"), t, Sense::Ge, need));
                        }
                        if let Some(x) = max {
                            rows.push((format!("Demanda máxima de {label}"), balance(v, -1.0), Sense::Le, x));
                        }
                    }
                } else {
                    rows.push((format!("Demanda de {label}"), t, Sense::Eq, need));
                }
            }
        }
    }
    for &(v, k, _, _, dir) in &through {
        let mut t = vec![(k, 1.0)];
        for &(i, j, _, a, _, _) in &arcs {
            if (dir == "entra" && j == v) || (dir == "sale" && i == v) {
                t.push((a, -1.0));
            }
        }
        rows.push((format!("Paso por {}", m.nodes[v].name), t, Sense::Eq, 0.0));
    }
    for (_, t, s, r) in &rows {
        lpm.row(t.clone(), *s, *r);
    }
    Ok(Built { lp: lpm, arcs, through, short, var_names: names, rows })
}

fn term_text(c: f64, name: &str, first: bool) -> String {
    let (sign, a) = if c < 0.0 { ("−", -c) } else { ("+", c) };
    let coef = if (a - 1.0).abs() < 1e-12 { String::new() } else { format!("{} ", fmt(a)) };
    if first {
        format!("{}{coef}{name}", if c < 0.0 { "−" } else { "" })
    } else {
        format!(" {sign} {coef}{name}")
    }
}

fn expr(terms: &[(usize, f64)], names: &[String]) -> String {
    if terms.is_empty() {
        return "0".into();
    }
    terms.iter().enumerate().map(|(k, (v, c))| term_text(*c, &names[*v], k == 0)).collect()
}

fn rel(s: Sense) -> &'static str {
    match s {
        Sense::Le => "≤",
        Sense::Ge => "≥",
        Sense::Eq => "=",
    }
}

fn model_blocks(m: &NetModel, b: &Built, penalty: Option<f64>) -> Vec<Block> {
    let mut obj: Vec<(usize, f64)> = b.arcs.iter().map(|a| (a.3, a.2)).collect();
    obj.extend(b.through.iter().filter(|t| t.2 != 0.0).map(|t| (t.1, t.2)));
    if let Some(p) = penalty {
        obj.extend(b.short.iter().map(|s| (s.1, p)));
    }
    let mut vars = Table::new(Some("Variables de decisión"), &["Variable", "Significado"]).left(0).left(1);
    for a in &b.arcs {
        vars.row(cells![&b.var_names[a.3], format!("Unidades enviadas de {} a {}", m.nodes[a.0].name, m.nodes[a.1].name)]);
    }
    for t in &b.through {
        vars.row(cells![&b.var_names[t.1], format!("Unidades que pasan por {} (el nodo convertido en arco)", m.nodes[t.0].name)]);
    }
    for s in &b.short {
        vars.row(cells![&b.var_names[s.1], format!("Demanda de {} que no se puede atender", m.nodes[s.0].name)]);
    }
    let mut t = Table::new(Some("Restricciones"), &["#", "Restricción", "Expresión", "Relación", "Lado derecho"]).left(1).left(2).left(3);
    for (k, (name, terms, s, r)) in b.rows.iter().enumerate() {
        t.row(cells![k + 1, name, expr(terms, &b.var_names), rel(*s), *r]);
    }
    let mut bnds = vec![];
    for a in &b.arcs {
        if a.4 > 0.0 {
            bnds.push(format!("{} ≥ {}", b.var_names[a.3], fmt(a.4)));
        }
        if a.5.is_finite() {
            bnds.push(format!("{} ≤ {}", b.var_names[a.3], fmt(a.5)));
        }
    }
    for tr in &b.through {
        if tr.3.is_finite() {
            bnds.push(format!("{} ≤ {} (capacidad del nodo)", b.var_names[tr.1], fmt(tr.3)));
        }
    }
    let mut out = vec![
        text(format!(
            "{} Z = {}",
            if m.objective.is_max() { "Max" } else { "Min" },
            expr(&obj, &b.var_names)
        )),
        vars.block(),
        t.block(),
    ];
    if !bnds.is_empty() {
        out.push(text(format!("Cotas: {}. Todas las variables son no negativas.", bnds.join("; "))));
    } else {
        out.push(text("Todas las variables son no negativas."));
    }
    if let Some(p) = penalty {
        out.push(text(format!(
            "Como la oferta o las capacidades no alcanzan, el modelo tiene variables de faltante F con un costo de penalización de {} por unidad: así primero se atiende toda la demanda posible y después se busca el menor costo.",
            fmt(p)
        )));
    }
    out
}

/// Programa lineal equivalente, en el formato del módulo LP-ILP.
fn lp_json(m: &NetModel, b: &Built, penalty: Option<f64>) -> Value {
    let nv = b.var_names.len();
    let mut cost = vec![0.0; nv];
    for a in &b.arcs {
        cost[a.3] = a.2;
    }
    for t in &b.through {
        cost[t.1] = t.2;
    }
    if let Some(p) = penalty {
        for s in &b.short {
            cost[s.1] = if m.objective.is_max() { -p } else { p };
        }
    }
    let mut lower = vec![0.0; nv];
    let mut upper: Vec<Value> = vec![Value::from("M"); nv];
    for a in &b.arcs {
        lower[a.3] = a.4;
        if a.5.is_finite() {
            upper[a.3] = json!(a.5);
        }
    }
    for t in &b.through {
        if t.3.is_finite() {
            upper[t.1] = json!(t.3);
        }
    }
    let variables: Vec<Value> = (0..nv)
        .map(|k| json!({"name": b.var_names[k], "cost": cost[k], "lower": lower[k], "upper": upper[k], "type": "continuous"}))
        .collect();
    let constraints: Vec<Value> = b
        .rows
        .iter()
        .enumerate()
        .map(|(k, (_, terms, s, r))| {
            let mut coefs = vec![0.0; nv];
            for (v, c) in terms {
                coefs[*v] += c;
            }
            json!({"name": format!("C{}", k + 1), "coefs": coefs, "direction": match s { Sense::Le => "<=", Sense::Ge => ">=", Sense::Eq => "=" }, "rhs": r})
        })
        .collect();
    json!({
        "name": if m.name.is_empty() { "Flujo en redes".to_string() } else { m.name.clone() },
        "objective": if m.objective.is_max() { "max" } else { "min" },
        "variables": variables,
        "constraints": constraints
    })
}

/// Programa lineal equivalente de cualquier red de flujo (para el botón "Abrir como programa lineal").
pub fn lp_model(m: &NetModel) -> Option<Value> {
    let b = build(m, false).ok()?;
    Some(lp_json(m, &b, None))
}

pub fn any_extended(m: &NetModel) -> bool {
    m.nodes.iter().any(|n| n.extended())
}

pub fn solve(m: &NetModel, o: &NetOptions) -> Result<Report, ModError> {
    let n = m.nodes.len();
    // 1) sin faltantes
    let b0 = build(m, false)?;
    let s0 = lp::solve(&b0.lp);
    let (b, s, shortage, penalty) = match s0.status {
        LpStatus::Optimal => (b0, s0, 0.0, None),
        LpStatus::Unbounded => {
            return Ok(Report::new("net", &m.name, Status::Unbounded, "El problema no está acotado: hay un ciclo que mejora el objetivo sin límite.")
                .section("solucion", "Solución", vec![text_tone("Algún ciclo de arcos sin cota superior mejora el objetivo indefinidamente.", Tone::Bad)]));
        }
        LpStatus::Error(e) => return Err(ModError::Invalid(e)),
        LpStatus::Infeasible => {
            // 2) con faltantes: se penaliza cada unidad sin entregar con un costo
            // muy alto, así primero se entrega lo máximo posible y después se busca el menor costo
            let mut b1 = build(m, true)?;
            if b1.short.is_empty() {
                return Ok(infeasible(m, &b0));
            }
            let scale: f64 = b1.arcs.iter().map(|a| a.2.abs()).sum::<f64>() + b1.through.iter().map(|t| t.2.abs()).sum::<f64>() + 1.0;
            let big = 10f64.powi((scale.log10().ceil() as i32 + 3).min(12));
            for &(_, k) in &b1.short {
                b1.lp.c[k] = if m.objective.is_max() { -big } else { big };
            }
            let r2 = lp::solve(&b1.lp);
            if r2.status != LpStatus::Optimal {
                return Ok(infeasible(m, &b0));
            }
            let missing: f64 = b1.short.iter().map(|&(_, k)| clean(r2.x[k])).sum();
            (b1, r2, missing, Some(big))
        }
    };

    let what = if m.objective.is_max() { "Beneficio" } else { "Costo" };
    let x = |k: usize| clean(s.x[k]);
    let transport: f64 = b.arcs.iter().map(|a| a.2 * x(a.3)).sum();
    let handling: f64 = b.through.iter().map(|t| t.2 * x(t.1)).sum();
    let total = transport + handling;

    let mut t = Table::new(Some("Envíos"), &["Desde", "Hasta", "Flujo", "Costo unitario", "Total", "Mínimo", "Máximo"]).left(1);
    let mut used = vec![];
    let mut flows = vec![];
    for &(i, j, c, k, lo, up) in &b.arcs {
        let f = x(k);
        if f.abs() > 1e-9 {
            used.push((i, j));
            flows.push(json!({"from": m.nodes[i].name, "to": m.nodes[j].name, "flow": f}));
            t.row_tone(cells![&m.nodes[i].name, &m.nodes[j].name, f, c, f * c, lo, up], Tone::Accent);
        }
    }
    t.row_tone(cells!["Total del transporte", "", "", "", transport, "", ""], Tone::Total);

    // balance de cada nodo
    let inflow = |v: usize| b.arcs.iter().filter(|a| a.1 == v).map(|a| x(a.3)).sum::<f64>();
    let outflow = |v: usize| b.arcs.iter().filter(|a| a.0 == v).map(|a| x(a.3)).sum::<f64>();
    let mut nt = Table::new(
        Some("Nodos"),
        &["Nodo", "Papel", "Oferta", "Demanda mínima", "Demanda máxima", "Envía", "Recibe", "Pasa por el nodo", "Capacidad", "Costo de manejo", "Faltante"],
    )
    .left(1);
    let mut node_data = vec![];
    for (v, nd) in m.nodes.iter().enumerate() {
        let k = kind(nd);
        let role = match k {
            Kind::Source => "Origen",
            Kind::Sink => "Destino",
            Kind::Transit => "Transbordo",
        };
        let pass = b.through.iter().find(|t| t.0 == v).map(|t| x(t.1));
        let miss = b.short.iter().find(|s| s.0 == v).map(|s| x(s.1)).unwrap_or(0.0);
        let maxd = match nd.max_demand {
            None if k == Kind::Sink => Cell::from(nd.demand),
            None => Cell::from(""),
            Some(Bnd(v)) if v.is_finite() => Cell::from(v),
            Some(_) => Cell::from("Sin límite"),
        };
        let tone = if miss > 1e-9 {
            Tone::Bad
        } else if b.through.iter().any(|t| t.0 == v && t.3.is_finite() && pass.unwrap_or(0.0) >= t.3 - 1e-7) {
            Tone::Highlight
        } else {
            Tone::Normal
        };
        nt.row_tone(
            cells![
                &nd.name,
                role,
                if nd.supply > 0.0 { Cell::from(nd.supply) } else { Cell::from("") },
                if k == Kind::Sink { Cell::from(nd.demand) } else { Cell::from("") },
                maxd,
                outflow(v),
                inflow(v),
                pass.map(Cell::from).unwrap_or(Cell::from("")),
                nd.capacity.map(Cell::from).unwrap_or(Cell::from("")),
                nd.handling_cost.filter(|h| *h != 0.0).map(Cell::from).unwrap_or(Cell::from("")),
                if miss > 1e-9 { Cell::from(miss) } else { Cell::from("") }
            ],
            tone,
        );
        node_data.push(json!({"node": nd.name, "sent": clean(outflow(v)) + 0.0, "received": clean(inflow(v)) + 0.0, "through": pass.map(|p| clean(p) + 0.0), "unmet": clean(miss) + 0.0}));
    }

    let xs: Vec<f64> = s.x.clone();
    let arcs2 = b.arcs.clone();
    let through2 = b.through.clone();
    let short2 = b.short.clone();
    let netb = node_network(
        m,
        "Plan de distribución",
        0,
        &used,
        &|i, j, c| {
            let f = arcs2.iter().find(|a| a.0 == i && a.1 == j).map(|a| xs[a.3]).unwrap_or(0.0);
            Some(if f.abs() > 1e-9 { format!("{} (c {})", fmt(clean(f)), fmt(c)) } else { format!("c {}", fmt(c)) })
        },
        &|v| {
            let nd = &m.nodes[v];
            let mut parts = vec![];
            if nd.supply > 0.0 {
                parts.push(format!("oferta {}", fmt(nd.supply)));
            }
            if kind(nd) == Kind::Sink {
                let got = arcs2.iter().filter(|a| a.1 == v).map(|a| xs[a.3]).sum::<f64>() - arcs2.iter().filter(|a| a.0 == v).map(|a| xs[a.3]).sum::<f64>();
                let miss = short2.iter().find(|s| s.0 == v).map(|s| clean(xs[s.1])).unwrap_or(0.0);
                parts.push(if miss > 1e-9 { format!("recibe {} (−{})", fmt(clean(got)), fmt(miss)) } else { format!("recibe {}", fmt(clean(got))) });
            }
            if let Some(t) = through2.iter().find(|t| t.0 == v) {
                let mut s = format!("pasan {}", fmt(clean(xs[t.1])));
                if t.3.is_finite() {
                    s.push_str(&format!(" de {}", fmt(t.3)));
                }
                if t.2 != 0.0 {
                    s.push_str(&format!(", $ {}", fmt(t.2)));
                }
                parts.push(s);
            }
            if parts.is_empty() {
                None
            } else {
                Some(parts.join("\n"))
            }
        },
        false,
    );

    let status = Status::Optimal;
    let headline = if shortage > 1e-9 {
        format!("No alcanza para toda la demanda: faltan {} unidades. {what} mínimo de lo que sí se entrega: {}.", fmt(shortage), fmt(total))
    } else {
        format!("{what} total óptimo: {} (transporte {} y manejo en los nodos {}).", fmt(total), fmt(transport), fmt(handling))
    };
    let mut mets = vec![metric_main(format!("{what} total"), total), metric("Transporte", transport)];
    if !b.through.is_empty() {
        mets.push(metric("Manejo en los nodos", handling));
    }
    let delivered: f64 = m.nodes.iter().enumerate().filter(|(_, nd)| kind(nd) == Kind::Sink).map(|(v, _)| inflow(v) - outflow(v)).sum();
    mets.push(metric("Unidades entregadas", delivered));
    if shortage > 1e-9 {
        mets.push(metric("Demanda sin atender", shortage));
    }
    let mut sol = vec![metrics(mets)];
    if shortage > 1e-9 {
        sol.push(text_tone(
            "La oferta o la capacidad de los nodos no alcanza para todas las demandas. Primero se entregó lo máximo posible y, con esa cantidad fija, se buscó el menor costo. La columna Faltante dice cuánto le falta a cada destino.",
            Tone::Highlight,
        ));
    }
    sol.push(t.block());
    sol.push(nt.block());

    let lpv = lp_json(m, &b, penalty);
    let mut rep = Report::new("net", &m.name, status, headline)
        .section("solucion", "Solución", sol)
        .section("red", "Red", vec![netb]);
    let mut model = vec![text(
        "Se arma el programa lineal de la red. Los nodos con costo o capacidad se convierten en un arco ficticio (la variable T): por él pasa todo lo que entra al nodo, cuesta lo que cuesta manejar una unidad y no puede superar la capacidad.",
    )];
    model.extend(model_blocks(m, &b, penalty));
    rep = rep.section("modelo", "Modelo matemático", model);
    let _ = (n, o);
    Ok(rep.data(json!({"objective_value": clean(total), "transport_cost": clean(transport), "handling_cost": clean(handling), "unmet": clean(shortage), "flows": flows, "nodes": node_data, "lp_model": lpv})))
}

fn infeasible(m: &NetModel, b: &Built) -> Report {
    let mut blocks = vec![text_tone("No hay forma de mover el flujo con estas ofertas, demandas, capacidades y cotas: revisa que las cotas mínimas de los arcos se puedan cumplir.", Tone::Bad)];
    blocks.extend(model_blocks(m, b, None));
    Report::new("net", &m.name, Status::Infeasible, "No es posible mover el flujo con la red y sus cotas.").section("solucion", "Solución", blocks)
}

/// Red equivalente sin datos de nodo, que WinQSB puede abrir: cada nodo con
/// costo o capacidad se divide en dos (entrada y salida) unidos por un arco con
/// ese costo y esa capacidad, y las demandas con rango se completan con un
/// destino ficticio que recibe la oferta que sobra.
pub fn expand(m: &NetModel) -> NetModel {
    if !any_extended(m) {
        return m.clone();
    }
    let n = m.nodes.len();
    let mut nodes: Vec<NetNode> = vec![];
    let mut idx_in = vec![0; n];
    let mut idx_out = vec![0; n];
    let mut pairs = vec![];
    for (v, nd) in m.nodes.iter().enumerate() {
        let split = nd.capacity.is_some() || nd.handling_cost.is_some_and(|h| h != 0.0);
        idx_in[v] = nodes.len();
        if split {
            let src = kind(nd) == Kind::Source;
            // en un origen, la oferta entra por la primera mitad; en los demás, la demanda queda en la segunda
            nodes.push(NetNode { name: nd.name.clone(), supply: if src { nd.supply } else { 0.0 }, demand: 0.0, ..Default::default() });
            idx_out[v] = nodes.len();
            nodes.push(NetNode { name: format!("{}'", nd.name), supply: if src { 0.0 } else { nd.supply }, demand: nd.demand, ..Default::default() });
            pairs.push(v);
        } else {
            idx_out[v] = idx_in[v];
            nodes.push(NetNode { name: nd.name.clone(), supply: nd.supply, demand: nd.demand, ..Default::default() });
        }
    }
    let ranged: Vec<usize> = (0..n).filter(|&v| m.nodes[v].max_demand.is_some()).collect();
    let total_supply: f64 = m.nodes.iter().map(|x| x.supply).sum();
    let total_min: f64 = m.nodes.iter().map(|x| x.demand).sum();
    let dummy = if !ranged.is_empty() && total_supply > total_min + EPS {
        nodes.push(NetNode { name: "Ficticio".into(), supply: 0.0, demand: total_supply - total_min, ..Default::default() });
        Some(nodes.len() - 1)
    } else {
        None
    };
    let nn = nodes.len();
    let mut arcs = vec![vec![None; nn]; nn];
    let mut bounds = vec![];
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            if let Some(c) = m.arc(i, j) {
                let (a, bb) = (idx_out[i], idx_in[j]);
                arcs[a][bb] = Some(c);
                let (lo, up) = bound_of(m, i, j);
                if lo > 0.0 || up.is_finite() {
                    bounds.push(ArcBound { from: nodes[a].name.clone(), to: nodes[bb].name.clone(), lower: lo, upper: up.is_finite().then_some(up) });
                }
            }
        }
    }
    for &v in &pairs {
        let nd = &m.nodes[v];
        let (a, bb) = (idx_in[v], idx_out[v]);
        arcs[a][bb] = Some(nd.handling_cost.unwrap_or(0.0));
        if let Some(c) = nd.capacity {
            bounds.push(ArcBound { from: nodes[a].name.clone(), to: nodes[bb].name.clone(), lower: 0.0, upper: Some(c) });
        }
    }
    if let Some(d) = dummy {
        // la oferta que sobra se queda en los orígenes...
        for (v, nd) in m.nodes.iter().enumerate() {
            if nd.supply > EPS {
                arcs[idx_in[v]][d] = Some(0.0);
            }
        }
        // ...o la recibe un destino por encima de su mínimo, hasta su máximo
        for &v in &ranged {
            let nd = &m.nodes[v];
            let extra = match nd.max_demand {
                Some(Bnd(x)) if x.is_finite() => Some((x - nd.demand).max(0.0)),
                _ => None,
            };
            if extra == Some(0.0) {
                continue;
            }
            arcs[idx_out[v]][d] = Some(0.0);
            if let Some(e) = extra {
                bounds.push(ArcBound { from: nodes[idx_out[v]].name.clone(), to: "Ficticio".into(), lower: 0.0, upper: Some(e) });
            }
        }
    }
    NetModel { nodes, arcs, bounds, positions: None, symmetric: false, ..m.clone() }
}
