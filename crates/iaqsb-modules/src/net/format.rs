//! Archivos .NET de WinQSB.
//!
//! Línea 1: NET, nombre, tipo (CNF/TP/AP/SPP/MFP/MST/TSP), criterio,
//!          Matrix|Graphic, Symmetry|Asymmetry
//! Línea 2: nº de nodos, nº de orígenes, nº de destinos
//! Después, la hoja "From \ To" con los nombres y los costes (vacío = sin
//! conexión), las ofertas en la última columna y las demandas en la última fila
//! cuando el tipo las usa. El flujo en redes añade las cotas de los arcos y todos
//! terminan con las posiciones del modelo gráfico (0 si no las hay).

use super::{ArcBound, NetModel, NetNode, NetType};
use iaqsb_core::winqsb::{self, check_tag, Reader, Writer};
use iaqsb_core::{ModError, Objective};

fn cell(s: &str, at: &str, zero_is_none: bool) -> Result<Option<f64>, ModError> {
    let v = winqsb::optional(s, at)?;
    Ok(match v {
        Some(x) if x.abs() >= 1e15 => None,
        Some(x) if zero_is_none && x == 0.0 => None,
        other => other,
    })
}

fn named(s: String, default: String) -> String {
    if s.trim().is_empty() {
        default
    } else {
        s.trim().to_string()
    }
}

pub fn read(bytes: &[u8]) -> Result<NetModel, ModError> {
    let mut r = Reader::new(bytes);
    let mut h = r.line("la cabecera")?;
    check_tag(&h, "NET", "NET")?;
    h.next();
    let mut name = h.next();
    if name.trim().is_empty() {
        name = "NET Problem".into();
    }
    let t = match h.next().trim().to_ascii_uppercase().get(..2).unwrap_or("") {
        "CN" | "NF" => NetType::NetworkFlow,
        "TP" | "TR" => NetType::Transportation,
        "AP" | "AS" => NetType::Assignment,
        "SP" | "SR" => NetType::ShortestPath,
        "MF" => NetType::MaxFlow,
        "MS" => NetType::MinSpanningTree,
        "TS" => NetType::Tsp,
        o => return Err(ModError::Format(format!("tipo de red desconocido: {o}"))),
    };
    let objective = Objective::parse(&h.next());
    let _input_mode = h.next();
    let symmetric = h.next().trim().to_ascii_uppercase().starts_with('S');

    let mut d = r.line("las dimensiones")?;
    let n = winqsb::req(&d.next(), "dimensiones")? as usize;
    let ns = winqsb::req(&d.next(), "dimensiones")? as usize;
    let nd = winqsb::req(&d.next(), "dimensiones")? as usize;
    if n < 1 {
        return Err(ModError::Format("el problema no tiene nodos".into()));
    }

    let mut m = NetModel {
        name,
        problem_type: t,
        objective,
        symmetric,
        sources: vec![],
        destinations: vec![],
        costs: vec![],
        nodes: vec![],
        arcs: vec![],
        bounds: vec![],
        start: None,
        end: None,
        positions: None,
    };

    let mut head = r.line("los nombres")?;
    head.next(); // "From \ To"
    if t.bipartite() {
        let (sp, dp) = if t == NetType::Assignment { ("Objeto", "Tarea") } else { ("Origen", "Destino") };
        m.destinations = (1..=nd)
            .map(|j| NetNode { name: named(head.next(), format!("{dp}{j}")), supply: 0.0, demand: 0.0, ..Default::default() })
            .collect();
        for i in 1..=ns {
            let mut row = r.line(&format!("la fila {i}"))?;
            let nm = named(row.next(), format!("{sp}{i}"));
            let at = format!("fila {nm}");
            let mut cs = Vec::with_capacity(nd);
            for _ in 0..nd {
                cs.push(cell(&row.next(), &at, false)?);
            }
            let supply = if t == NetType::Transportation { winqsb::req(&row.next(), &at)? } else { 1.0 };
            m.sources.push(NetNode { name: nm, supply, demand: 0.0, ..Default::default() });
            m.costs.push(cs);
        }
        if t == NetType::Transportation {
            let mut dl = r.line("la fila de demandas")?;
            dl.next();
            for dst in m.destinations.iter_mut() {
                dst.demand = winqsb::req(&dl.next(), "demandas")?;
            }
        } else {
            for dst in m.destinations.iter_mut() {
                dst.demand = 1.0;
            }
        }
    } else {
        m.nodes = (1..=n).map(|j| NetNode { name: named(head.next(), format!("Nodo{j}")), supply: 0.0, demand: 0.0, ..Default::default() }).collect();
        let flow = t == NetType::NetworkFlow;
        let zero_is_none = t == NetType::MaxFlow;
        for i in 0..n {
            let mut row = r.line(&format!("la fila {}", i + 1))?;
            let nm = row.next();
            if !nm.trim().is_empty() {
                m.nodes[i].name = nm.trim().to_string();
            }
            let at = format!("fila {}", m.nodes[i].name);
            let mut cs = Vec::with_capacity(n);
            for _ in 0..n {
                cs.push(cell(&row.next(), &at, zero_is_none)?);
            }
            if flow {
                m.nodes[i].supply = winqsb::req(&row.next(), &at)?;
            }
            m.arcs.push(cs);
        }
        if flow {
            let mut dl = r.line("la fila de demandas")?;
            dl.next();
            for node in m.nodes.iter_mut() {
                node.demand = winqsb::req(&dl.next(), "demandas")?;
            }
            if let Some(mut c) = r.try_line() {
                let k = winqsb::req(&c.next(), "cotas")? as usize;
                for _ in 0..k {
                    let mut b = r.line("una cota de arco")?;
                    let i = winqsb::req(&b.next(), "cotas")? as usize;
                    let j = winqsb::req(&b.next(), "cotas")? as usize;
                    let lo = winqsb::req(&b.next(), "cotas")?;
                    let up = winqsb::optional(&b.next(), "cotas")?;
                    if i >= 1 && i <= n && j >= 1 && j <= n {
                        m.bounds.push(ArcBound {
                            from: m.nodes[i - 1].name.clone(),
                            to: m.nodes[j - 1].name.clone(),
                            lower: lo,
                            upper: up.filter(|u| u.abs() < 1e15),
                        });
                    }
                }
            }
        }
    }
    // posiciones del modelo gráfico
    if let Some(mut g) = r.try_line() {
        let has = winqsb::req(&g.next(), "posiciones").unwrap_or(0.0) != 0.0;
        if has {
            let mut pos = vec![(0i64, 0i64); n];
            for _ in 0..n {
                if let Some(mut p) = r.try_line() {
                    let i = winqsb::req(&p.next(), "posiciones")? as usize;
                    let row = winqsb::req(&p.next(), "posiciones")? as i64;
                    let col = winqsb::req(&p.next(), "posiciones")? as i64;
                    if i >= 1 && i <= n {
                        pos[i - 1] = (row, col);
                    }
                }
            }
            m.positions = Some(pos);
        }
    }
    Ok(m)
}

fn c(x: &Option<f64>) -> String {
    match x {
        Some(v) => winqsb::num(*v),
        None => " ".into(),
    }
}

pub fn write(m: &NetModel) -> Vec<u8> {
    let mut w = Writer::new();
    let t = m.problem_type;
    let (n, ns, nd) = if t.bipartite() {
        (m.sources.len() + m.destinations.len(), m.sources.len(), m.destinations.len())
    } else {
        (m.nodes.len(), 0, 0)
    };
    w.row(&[
        "NET",
        &m.name,
        t.code(),
        m.objective.word(),
        "Matrix",
        if m.symmetric { "Symmetry" } else { "Asymmetry" },
    ]);
    w.row(&[n.to_string(), ns.to_string(), nd.to_string()]);
    match t {
        NetType::Transportation | NetType::Assignment => {
            let mut h = vec!["From \\ To".to_string()];
            h.extend(m.destinations.iter().map(|d| d.name.clone()));
            if t == NetType::Transportation {
                h.push("Supply".into());
            }
            w.row(&h);
            for (i, s) in m.sources.iter().enumerate() {
                let mut r = vec![s.name.clone()];
                r.extend(m.costs[i].iter().map(c));
                if t == NetType::Transportation {
                    r.push(winqsb::num(s.supply));
                }
                w.row(&r);
            }
            if t == NetType::Transportation {
                let mut r = vec!["Demand".to_string()];
                r.extend(m.destinations.iter().map(|d| winqsb::num(d.demand)));
                r.push(" ".into());
                w.row(&r);
            }
        }
        _ => {
            let flow = t == NetType::NetworkFlow;
            let mut h = vec!["From \\ To".to_string()];
            h.extend(m.nodes.iter().map(|d| d.name.clone()));
            if flow {
                h.push("Supply".into());
            }
            w.row(&h);
            for (i, s) in m.nodes.iter().enumerate() {
                let mut r = vec![s.name.clone()];
                r.extend(m.arcs[i].iter().map(c));
                if flow {
                    r.push(winqsb::num(s.supply));
                }
                w.row(&r);
            }
            if flow {
                let mut r = vec!["Demand".to_string()];
                r.extend(m.nodes.iter().map(|d| winqsb::num(d.demand)));
                r.push(" ".into());
                w.row(&r);
                let idx = |name: &str| m.nodes.iter().position(|nd| nd.name.eq_ignore_ascii_case(name));
                let rows: Vec<Vec<String>> = m
                    .bounds
                    .iter()
                    .filter_map(|b| {
                        let (i, j) = (idx(&b.from)?, idx(&b.to)?);
                        Some(vec![
                            (i + 1).to_string(),
                            (j + 1).to_string(),
                            winqsb::num(b.lower),
                            b.upper.map(winqsb::num).unwrap_or_else(|| "1E+30".into()),
                        ])
                    })
                    .collect();
                w.row(&[rows.len().to_string()]);
                for r in rows {
                    w.row(&r);
                }
            }
        }
    }
    match &m.positions {
        Some(p) if p.len() == n => {
            w.row(&["1"]);
            for (i, (r, c)) in p.iter().enumerate() {
                w.row(&[(i + 1).to_string(), r.to_string(), c.to_string()]);
            }
        }
        _ => {
            w.row(&["0"]);
        }
    }
    w.bytes()
}
