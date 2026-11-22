//! Dibujo de las redes del módulo NET.

use super::{NetModel, NetType};
use iaqsb_core::layout;
use iaqsb_core::report::{fmt, Block, NetEdge, NetNode as GNode, Network, Tone};

/// Arcos existentes (i, j, valor). En redes simétricas, cada par una sola vez.
pub fn arcs(m: &NetModel) -> Vec<(usize, usize, f64)> {
    let n = m.nodes.len();
    let mut v = Vec::new();
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            if m.symmetric {
                if i < j {
                    if let Some(c) = m.arc(i, j) {
                        v.push((i, j, c));
                    }
                }
            } else if let Some(c) = m.arcs[i][j] {
                v.push((i, j, c));
            }
        }
    }
    v
}

/// Posiciones del editor visual (x, y de cada nodo), llevadas a [0.05, 0.95];
/// None si a algún nodo le falta la posición o todas coinciden.
pub fn editor_positions(nodes: &[super::NetNode]) -> Option<Vec<(f64, f64)>> {
    let p: Vec<(f64, f64)> = nodes.iter().map(|n| Some((n.x?, n.y?))).collect::<Option<_>>()?;
    if p.is_empty() || p.iter().any(|q| !q.0.is_finite() || !q.1.is_finite()) {
        return None;
    }
    let (x0, x1) = p.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, q| (a.0.min(q.0), a.1.max(q.0)));
    let (y0, y1) = p.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, q| (a.0.min(q.1), a.1.max(q.1)));
    if x1 - x0 < 1e-9 && y1 - y0 < 1e-9 {
        return None;
    }
    let f = |v: f64, lo: f64, hi: f64| if hi - lo < 1e-9 { 0.5 } else { 0.05 + 0.9 * (v - lo) / (hi - lo) };
    Some(p.iter().map(|&(x, y)| (f(x, x0, x1), f(y, y0, y1))).collect())
}

pub fn positions(m: &NetModel, start: usize) -> Vec<(f64, f64)> {
    let n = m.nodes.len();
    // primero, donde el usuario dejó los nodos en el editor visual
    if let Some(p) = editor_positions(&m.nodes) {
        return p;
    }
    if let Some(p) = &m.positions {
        if p.len() == n && p.iter().any(|x| *x != p[0]) {
            return layout::from_grid(p);
        }
    }
    match m.problem_type {
        NetType::MinSpanningTree | NetType::Tsp => layout::circle(n),
        _ => {
            let e: Vec<(usize, usize)> = arcs(m).iter().map(|a| (a.0, a.1)).collect();
            layout::layered(n, &e, start)
        }
    }
}

/// Red de nodos con arcos resaltados. `label(i, j, valor)` da el texto de cada arco;
/// `only_used` dibuja solo los arcos resaltados (p. ej. el recorrido del agente viajero).
pub fn node_network(
    m: &NetModel,
    title: &str,
    start: usize,
    highlight: &[(usize, usize)],
    label: &dyn Fn(usize, usize, f64) -> Option<String>,
    node_note: &dyn Fn(usize) -> Option<String>,
    only_used: bool,
) -> Block {
    let pos = positions(m, start);
    let hl = |i: usize, j: usize| highlight.iter().any(|&(a, b)| (a, b) == (i, j) || (m.symmetric && (b, a) == (i, j)));
    let mut edges = Vec::new();
    for (i, j, c) in arcs(m) {
        let used = hl(i, j) || (!m.symmetric && false);
        if only_used && !used {
            continue;
        }
        // en redes dirigidas, si el resaltado va en sentido contrario no se marca
        edges.push(NetEdge {
            from: format!("n{i}"),
            to: format!("n{j}"),
            label: label(i, j, c),
            tone: if used { Tone::Highlight } else { Tone::Muted },
        });
    }
    // arcos resaltados que no están en la matriz (p. ej. simetría implícita)
    for &(a, b) in highlight {
        let exists = edges.iter().any(|e| (e.from == format!("n{a}") && e.to == format!("n{b}")) || (m.symmetric && e.from == format!("n{b}") && e.to == format!("n{a}")));
        if !exists {
            edges.push(NetEdge {
                from: format!("n{a}"),
                to: format!("n{b}"),
                label: m.arc(a, b).and_then(|c| label(a, b, c)),
                tone: Tone::Highlight,
            });
        }
    }
    Block::Network(Network {
        title: Some(title.into()),
        directed: !m.symmetric,
        nodes: m
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| GNode {
                id: format!("n{i}"),
                label: n.name.clone(),
                x: pos[i].0,
                y: pos[i].1,
                note: node_note(i),
                tone: if highlight.iter().any(|&(a, b)| a == i || b == i) { Tone::Accent } else { Tone::Normal },
            })
            .collect(),
        edges,
    })
}

/// Red bipartita (transporte, asignación) con los envíos.
pub fn bipartite_network(m: &NetModel, title: &str, flows: &[(usize, usize, f64)], unit_label: bool) -> Block {
    let ns = m.sources.len();
    let nd = m.destinations.len();
    let all: Vec<super::NetNode> = m.sources.iter().chain(&m.destinations).cloned().collect();
    let pos = editor_positions(&all).unwrap_or_else(|| layout::bipartite(ns, nd));
    let mut nodes = Vec::new();
    for (i, s) in m.sources.iter().enumerate() {
        nodes.push(GNode {
            id: format!("s{i}"),
            label: s.name.clone(),
            x: pos[i].0,
            y: pos[i].1,
            note: if m.problem_type == NetType::Transportation { Some(format!("oferta {}", fmt(s.supply))) } else { None },
            tone: Tone::Normal,
        });
    }
    for (j, d) in m.destinations.iter().enumerate() {
        nodes.push(GNode {
            id: format!("d{j}"),
            label: d.name.clone(),
            x: pos[ns + j].0,
            y: pos[ns + j].1,
            note: if m.problem_type == NetType::Transportation { Some(format!("demanda {}", fmt(d.demand))) } else { None },
            tone: Tone::Normal,
        });
    }
    let edges = flows
        .iter()
        .filter(|f| f.2 > 1e-9)
        .map(|&(i, j, q)| NetEdge {
            from: format!("s{i}"),
            to: format!("d{j}"),
            label: if unit_label { None } else { Some(fmt(q)) },
            tone: Tone::Highlight,
        })
        .collect();
    Block::Network(Network { title: Some(title.into()), directed: true, nodes, edges })
}
