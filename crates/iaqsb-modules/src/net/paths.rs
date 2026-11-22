//! Ruta más corta (Dijkstra / Bellman-Ford / camino más largo en redes acíclicas),
//! flujo máximo (Edmonds-Karp con corte mínimo) y árbol de expansión mínima
//! (Kruskal o Prim).

use super::layout::node_network;
use super::{NetModel, NetOptions};
use iaqsb_core::report::*;
use iaqsb_core::{cells, ModError};
use serde_json::json;
use std::collections::VecDeque;

fn ends(m: &NetModel) -> Result<(usize, usize), ModError> {
    let n = m.nodes.len();
    let s = m.node_index(m.start.as_deref().unwrap_or(""), 0)?;
    let t = m.node_index(m.end.as_deref().unwrap_or(""), n - 1)?;
    if s == t {
        return Err(ModError::Invalid("el nodo inicial y el final deben ser distintos".into()));
    }
    Ok((s, t))
}

/// Lista de adyacencia (i → j, costo) respetando la simetría.
fn adjacency(m: &NetModel) -> Vec<Vec<(usize, f64)>> {
    let n = m.nodes.len();
    let mut adj = vec![vec![]; n];
    for (i, row) in adj.iter_mut().enumerate() {
        for j in 0..n {
            if i != j {
                if let Some(c) = m.arc(i, j) {
                    row.push((j, c));
                }
            }
        }
    }
    adj
}

fn path_to(pred: &[Option<usize>], t: usize) -> Vec<usize> {
    let mut p = vec![t];
    let mut x = t;
    while let Some(q) = pred[x] {
        p.push(q);
        x = q;
        if p.len() > pred.len() {
            break;
        }
    }
    p.reverse();
    p
}

fn label_table(m: &NetModel, dist: &[f64], pred: &[Option<usize>], perm: &[bool], current: Option<usize>, title: &str) -> Block {
    let mut t = Table::new(Some(title), &["Nodo", "Distancia", "Predecesor", "Etiqueta"]).left(2).left(3);
    for i in 0..m.nodes.len() {
        let st = if perm[i] { "permanente" } else if dist[i].is_finite() { "temporal" } else { "sin etiqueta" };
        t.row(cells![
            &m.nodes[i].name,
            if dist[i].is_finite() { Cell::Num(dist[i]) } else { "∞".into() },
            pred[i].map(|p| m.nodes[p].name.clone()),
            st
        ]);
        if Some(i) == current {
            t.mark(i, 0, Tone::Highlight);
            t.mark(i, 1, Tone::Highlight);
        } else if perm[i] {
            t.mark(i, 1, Tone::Accent);
        }
    }
    Block::Table(t)
}

pub fn shortest(m: &NetModel, o: &NetOptions) -> Result<Report, ModError> {
    let n = m.nodes.len();
    let (s, t) = ends(m)?;
    let adj = adjacency(m);
    let is_max = m.objective.is_max();
    let negative = adj.iter().flatten().any(|&(_, c)| c < 0.0);
    let mut dist = vec![f64::INFINITY; n];
    let mut pred: Vec<Option<usize>> = vec![None; n];
    let mut st_blocks: Vec<Step> = Vec::new();
    let method;

    if is_max {
        // camino más largo: solo en redes acíclicas
        method = "camino más largo en una red acíclica (orden topológico)";
        let mut indeg = vec![0usize; n];
        for row in &adj {
            for &(j, _) in row {
                indeg[j] += 1;
            }
        }
        let mut q: VecDeque<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
        let mut order = vec![];
        while let Some(u) = q.pop_front() {
            order.push(u);
            for &(v, _) in &adj[u] {
                indeg[v] -= 1;
                if indeg[v] == 0 {
                    q.push_back(v);
                }
            }
        }
        if order.len() < n {
            return Err(ModError::Invalid(
                "la red tiene ciclos: el camino más largo solo está definido en redes sin ciclos (dirigidas y acíclicas)".into(),
            ));
        }
        let mut best = vec![f64::NEG_INFINITY; n];
        best[s] = 0.0;
        for &u in &order {
            if best[u] == f64::NEG_INFINITY {
                continue;
            }
            for &(v, c) in &adj[u] {
                if best[u] + c > best[v] {
                    best[v] = best[u] + c;
                    pred[v] = Some(u);
                }
            }
        }
        dist = best.iter().map(|&b| if b == f64::NEG_INFINITY { f64::INFINITY } else { b }).collect();
    } else if negative {
        method = "Bellman-Ford (hay distancias negativas)";
        dist[s] = 0.0;
        for round in 0..n {
            let mut changed = false;
            for u in 0..n {
                if !dist[u].is_finite() {
                    continue;
                }
                for &(v, c) in &adj[u] {
                    if dist[u] + c < dist[v] - 1e-12 {
                        dist[v] = dist[u] + c;
                        pred[v] = Some(u);
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
            if round == n - 1 {
                return Err(ModError::Invalid("hay un ciclo de distancia negativa: la ruta más corta no está definida".into()));
            }
        }
    } else {
        method = "Dijkstra (etiquetas permanentes)";
        dist[s] = 0.0;
        let mut perm = vec![false; n];
        for k in 0..n {
            let u = (0..n).filter(|&i| !perm[i] && dist[i].is_finite()).min_by(|&a, &b| dist[a].partial_cmp(&dist[b]).unwrap().then(a.cmp(&b)));
            let Some(u) = u else { break };
            perm[u] = true;
            let mut updates = vec![];
            for &(v, c) in &adj[u] {
                if !perm[v] && dist[u] + c < dist[v] - 1e-12 {
                    dist[v] = dist[u] + c;
                    pred[v] = Some(u);
                    updates.push(format!("{} = {}", m.nodes[v].name, fmt(dist[v])));
                }
            }
            if o.steps && n <= 60 {
                st_blocks.push(step(
                    format!("Iteración {}: etiqueta permanente en {}", k + 1, m.nodes[u].name),
                    format!(
                        "{} tiene la menor etiqueta temporal ({}), así que pasa a permanente. {}",
                        m.nodes[u].name,
                        fmt(dist[u]),
                        if updates.is_empty() { "No mejora ninguna etiqueta vecina.".into() } else { format!("Se mejoran: {}.", updates.join(", ")) }
                    ),
                    vec![label_table(m, &dist, &pred, &perm, Some(u), "Etiquetas")],
                ));
            }
            if u == t && !o.steps {
                break;
            }
        }
    }

    if !dist[t].is_finite() {
        return Ok(Report::new("net", &m.name, Status::Infeasible, format!("No hay ningún camino de {} a {}.", m.nodes[s].name, m.nodes[t].name))
            .section("solucion", "Solución", vec![text_tone("No existe un camino entre los dos nodos con los arcos de la red.", Tone::Bad)]));
    }
    let path = path_to(&pred, t);
    let what = if is_max { "Distancia máxima" } else { "Distancia mínima" };
    let mut pt = Table::new(Some("Camino"), &["Desde", "Hasta", "Distancia", "Acumulada"]).left(1);
    let mut acc = 0.0;
    let mut hl = vec![];
    for w in path.windows(2) {
        let c = m.arc(w[0], w[1]).unwrap();
        acc += c;
        hl.push((w[0], w[1]));
        pt.row(cells![&m.nodes[w[0]].name, &m.nodes[w[1]].name, c, acc]);
    }
    pt.row_tone(cells!["Total", "", "", dist[t]], Tone::Total);
    let mut all = Table::new(Some(&format!("Desde {} a cada nodo", m.nodes[s].name)), &["Nodo", "Distancia", "Camino"]).left(2);
    for i in 0..n {
        if i == s {
            continue;
        }
        if dist[i].is_finite() {
            let p = path_to(&pred, i).iter().map(|&k| m.nodes[k].name.clone()).collect::<Vec<_>>().join(" → ");
            all.row(cells![&m.nodes[i].name, dist[i], p]);
        } else {
            all.row_tone(cells![&m.nodes[i].name, "inalcanzable", ""], Tone::Muted);
        }
    }
    let names: Vec<String> = path.iter().map(|&k| m.nodes[k].name.clone()).collect();
    let net = node_network(
        m,
        "Ruta en la red",
        s,
        &hl,
        &|_, _, c| Some(fmt(c)),
        &|i| if dist[i].is_finite() { Some(fmt(dist[i])) } else { None },
        false,
    );
    let mut rep = Report::new("net", &m.name, Status::Optimal, format!("{what} de {} a {}: {} ({}).", m.nodes[s].name, m.nodes[t].name, fmt(dist[t]), names.join(" → ")))
        .section("solucion", "Solución", vec![
            metrics(vec![metric_main(what, dist[t]), metric("Arcos en el camino", path.len() - 1)]),
            text(format!("Camino: {}. Método: {method}.", names.join(" → "))),
            pt.block(),
            all.block(),
        ])
        .section("red", "Red", vec![net]);
    if !st_blocks.is_empty() {
        rep = rep.section("pasos", "Paso a paso", vec![steps(st_blocks)]);
    }
    let lpt = super::lpmodel::shortest_path(m, s, t);
    rep = rep.section(
        "modelo",
        "Modelo matemático",
        lpt.blocks(
            &format!("Como programa entero binario: x(i,j) = 1 si la ruta usa el arco de i a j. Del origen sale un vehículo, al destino llega uno y en cada nodo intermedio lo que entra sale (continuidad). No se usan los arcos que entran al origen ni los que salen del destino ({}).", iaqsb_core::biblio::c("sarache2026redes")),
            lpt.vars.len() <= 120,
        ),
    );
    Ok(rep.data(json!({"distance": clean(dist[t]), "path": names, "method": method, "lp_model": lpt.json()})))
}

pub fn max_flow(m: &NetModel, o: &NetOptions) -> Result<Report, ModError> {
    let n = m.nodes.len();
    let (s, t) = ends(m)?;
    let mut cap = vec![vec![0.0; n]; n];
    for (i, row) in cap.iter_mut().enumerate() {
        for (j, c) in row.iter_mut().enumerate() {
            if i != j {
                if let Some(v) = m.arc(i, j) {
                    if v < 0.0 {
                        return Err(ModError::Invalid("las capacidades no pueden ser negativas".into()));
                    }
                    *c = v;
                }
            }
        }
    }
    let mut flow = vec![vec![0.0; n]; n];
    let mut total = 0.0;
    let mut paths_tbl = Table::new(Some("Caminos de aumento"), &["Iteración", "Camino", "Capacidad del camino", "Flujo acumulado"]).left(1);
    let mut st: Vec<Step> = vec![];
    for k in 1..=10_000 {
        // BFS en la red residual
        let mut prev: Vec<Option<(usize, bool)>> = vec![None; n]; // (nodo previo, arco hacia delante)
        let mut seen = vec![false; n];
        seen[s] = true;
        let mut q = VecDeque::from([s]);
        while let Some(u) = q.pop_front() {
            for v in 0..n {
                if seen[v] {
                    continue;
                }
                if cap[u][v] - flow[u][v] > 1e-12 {
                    seen[v] = true;
                    prev[v] = Some((u, true));
                    q.push_back(v);
                } else if flow[v][u] > 1e-12 {
                    seen[v] = true;
                    prev[v] = Some((u, false));
                    q.push_back(v);
                }
            }
        }
        if !seen[t] {
            break;
        }
        let mut b = f64::INFINITY;
        let mut x = t;
        let mut nodes = vec![t];
        while let Some((p, fwd)) = prev[x] {
            b = b.min(if fwd { cap[p][x] - flow[p][x] } else { flow[x][p] });
            x = p;
            nodes.push(p);
        }
        if b.is_infinite() {
            return Err(ModError::Invalid("hay un camino de capacidad ilimitada".into()));
        }
        nodes.reverse();
        let mut x = t;
        let mut hl = vec![];
        while let Some((p, fwd)) = prev[x] {
            if fwd {
                flow[p][x] += b;
            } else {
                flow[x][p] -= b;
            }
            hl.push((p, x));
            x = p;
        }
        total += b;
        let pname = nodes.iter().map(|&i| m.nodes[i].name.clone()).collect::<Vec<_>>().join(" → ");
        paths_tbl.row(cells![k, &pname, b, total]);
        if o.steps && n <= 60 {
            let fl = flow.clone();
            st.push(step(
                format!("Camino de aumento {k}"),
                format!("En la red residual se encuentra el camino {pname} (búsqueda en anchura). Su capacidad es {}: el flujo total sube a {}.", fmt(b), fmt(total)),
                vec![node_network(
                    m,
                    "Flujo / capacidad",
                    s,
                    &hl,
                    &|i, j, c| Some(format!("{}/{}", fmt(fl[i][j].max(if m.symmetric { fl[j][i] } else { 0.0 })), fmt(c))),
                    &|_| None,
                    false,
                )],
            ));
        }
        if k == 10_000 {
            return Err(ModError::Invalid("demasiadas iteraciones de flujo máximo".into()));
        }
    }
    // corte mínimo: nodos alcanzables en la red residual
    let mut reach = vec![false; n];
    reach[s] = true;
    let mut q = VecDeque::from([s]);
    while let Some(u) = q.pop_front() {
        for v in 0..n {
            if !reach[v] && (cap[u][v] - flow[u][v] > 1e-12 || flow[v][u] > 1e-12) {
                reach[v] = true;
                q.push_back(v);
            }
        }
    }
    let mut ft = Table::new(Some("Flujo por arco"), &["Desde", "Hasta", "Capacidad", "Flujo", "En el corte mínimo"]).left(1).left(4);
    let mut used = vec![];
    let mut cut = vec![];
    for i in 0..n {
        for j in 0..n {
            if cap[i][j] > 0.0 && !(m.symmetric && j < i && m.arcs[i][j].is_none()) {
                let f = (flow[i][j] - if m.symmetric { flow[j][i] } else { 0.0 }).max(0.0);
                let in_cut = reach[i] && !reach[j];
                if in_cut {
                    cut.push(format!("{} → {}", m.nodes[i].name, m.nodes[j].name));
                }
                if f > 1e-12 {
                    used.push((i, j));
                }
                let tone = if in_cut { Tone::Highlight } else if f > 1e-12 { Tone::Accent } else { Tone::Normal };
                ft.row_tone(cells![&m.nodes[i].name, &m.nodes[j].name, cap[i][j], f, if in_cut { "sí" } else { "" }], tone);
            }
        }
    }
    let fl = flow.clone();
    let net = node_network(
        m,
        "Flujo máximo (flujo / capacidad)",
        s,
        &used,
        &|i, j, c| Some(format!("{}/{}", fmt((fl[i][j] - if m.symmetric { fl[j][i] } else { 0.0 }).abs()), fmt(c))),
        &|i| if reach[i] { Some("lado de la fuente".into()) } else { None },
        false,
    );
    let mut rep = Report::new("net", &m.name, Status::Optimal, format!("Flujo máximo de {} a {}: {}.", m.nodes[s].name, m.nodes[t].name, fmt(total)))
        .section("solucion", "Solución", vec![
            metrics(vec![metric_main("Flujo máximo", total), metric("Caminos de aumento", paths_tbl.rows.len())]),
            text(format!(
                "El corte mínimo separa {{{}}} del resto y está formado por {}; su capacidad, {}, es igual al flujo máximo.",
                (0..n).filter(|&i| reach[i]).map(|i| m.nodes[i].name.clone()).collect::<Vec<_>>().join(", "),
                cut.join(", "),
                fmt(total)
            )),
            ft.block(),
            paths_tbl.block(),
        ])
        .section("red", "Red", vec![net]);
    if !st.is_empty() {
        rep = rep.section("pasos", "Paso a paso", vec![steps(st)]);
    }
    Ok(rep.data(json!({"max_flow": clean(total), "min_cut": cut})))
}

pub fn mst(m: &NetModel, o: &NetOptions) -> Result<Report, ModError> {
    let n = m.nodes.len();
    let is_max = m.objective.is_max();
    // aristas no dirigidas: la mejor de i→j y j→i
    let mut edges: Vec<(usize, usize, f64)> = vec![];
    for i in 0..n {
        for j in i + 1..n {
            let a = m.arcs[i][j];
            let b = m.arcs[j][i];
            let c = match (a, b) {
                (Some(x), Some(y)) => Some(if is_max { x.max(y) } else { x.min(y) }),
                (x, y) => x.or(y),
            };
            if let Some(c) = c {
                edges.push((i, j, c));
            }
        }
    }
    let better = |a: f64, b: f64| if is_max { a > b } else { a < b };
    let mut chosen: Vec<(usize, usize, f64)> = vec![];
    let mut st: Vec<Step> = vec![];
    let prim = o.mst_method.eq_ignore_ascii_case("prim");
    let algo;
    if prim {
        algo = "Prim";
        let mut in_tree = vec![false; n];
        in_tree[0] = true;
        for k in 1..n {
            let cand = edges
                .iter()
                .filter(|&&(a, b, _)| in_tree[a] != in_tree[b])
                .fold(None::<(usize, usize, f64)>, |acc, &e| match acc {
                    Some(x) if !better(e.2, x.2) => Some(x),
                    _ => Some(e),
                });
            let Some(e) = cand else { break };
            let newn = if in_tree[e.0] { e.1 } else { e.0 };
            in_tree[newn] = true;
            chosen.push(e);
            if o.steps && n <= 60 {
                st.push(step(
                    format!("Paso {k}: se conecta {}", m.nodes[newn].name),
                    format!(
                        "De las aristas que salen del árbol, la {} es {} – {} ({}). Se añade {} al árbol.",
                        if is_max { "mayor" } else { "menor" },
                        m.nodes[e.0].name,
                        m.nodes[e.1].name,
                        fmt(e.2),
                        m.nodes[newn].name
                    ),
                    vec![node_network(m, "Árbol parcial", 0, &chosen.iter().map(|e| (e.0, e.1)).collect::<Vec<_>>(), &|_, _, c| Some(fmt(c)), &|_| None, false)],
                ));
            }
        }
    } else {
        algo = "Kruskal";
        let mut sorted = edges.clone();
        sorted.sort_by(|a, b| if is_max { b.2.partial_cmp(&a.2).unwrap() } else { a.2.partial_cmp(&b.2).unwrap() });
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(p: &mut [usize], x: usize) -> usize {
            let mut r = x;
            while p[r] != r {
                r = p[r];
            }
            r
        }
        let mut log = Table::new(Some("Aristas en orden"), &["Arista", "Distancia", "Decisión"]).left(2);
        for &(a, b, c) in &sorted {
            let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
            let name = format!("{} – {}", m.nodes[a].name, m.nodes[b].name);
            if ra != rb {
                parent[ra] = rb;
                chosen.push((a, b, c));
                log.row_tone(cells![&name, c, "se añade"], Tone::Accent);
                if o.steps && n <= 60 {
                    st.push(step(
                        format!("Se añade {name}"),
                        format!("Es la arista {} que queda ({}) y no forma ciclo con las ya elegidas.", if is_max { "mayor" } else { "menor" }, fmt(c)),
                        vec![node_network(m, "Árbol parcial", 0, &chosen.iter().map(|e| (e.0, e.1)).collect::<Vec<_>>(), &|_, _, c| Some(fmt(c)), &|_| None, false)],
                    ));
                }
                if chosen.len() == n - 1 {
                    break;
                }
            } else {
                log.row_tone(cells![&name, c, "se descarta: formaría un ciclo"], Tone::Muted);
            }
        }
        if o.steps && n <= 60 {
            st.insert(0, step("Orden de las aristas", "Kruskal recorre las aristas de menor a mayor y añade cada una si no forma ciclo.", vec![log.block()]));
        }
    }
    let total: f64 = chosen.iter().map(|e| e.2).sum();
    let connected = chosen.len() == n - 1;
    let mut t = Table::new(None, &["Desde", "Hasta", "Distancia"]).left(1);
    for &(a, b, c) in &chosen {
        t.row(cells![&m.nodes[a].name, &m.nodes[b].name, c]);
    }
    t.row_tone(cells!["Total", "", total], Tone::Total);
    let hl: Vec<(usize, usize)> = chosen.iter().map(|e| (e.0, e.1)).collect();
    let mut sm = m.clone();
    sm.symmetric = true;
    let net = node_network(&sm, "Árbol de expansión", 0, &hl, &|_, _, c| Some(fmt(c)), &|_| None, false);
    let what = if is_max { "máxima" } else { "mínima" };
    let mut rep = Report::new(
        "net",
        &m.name,
        if connected { Status::Optimal } else { Status::Infeasible },
        if connected {
            format!("Longitud {what} del árbol de expansión: {}.", fmt(total))
        } else {
            "La red no es conexa: no existe un árbol que una todos los nodos.".into()
        },
    )
    .section("solucion", "Solución", {
        let mut b = vec![metrics(vec![metric_main(format!("Longitud total {what}"), total), metric("Aristas", chosen.len())])];
        if !connected {
            b.push(text_tone("La red no es conexa: se muestra el bosque de expansión de cada componente.", Tone::Bad));
        }
        b.push(text(format!("Algoritmo: {algo}.")));
        b.push(t.block());
        b
    })
    .section("red", "Red", vec![net]);
    if !st.is_empty() {
        rep = rep.section("pasos", "Paso a paso", vec![steps(st)]);
    }
    Ok(rep.data(json!({
        "total": clean(total),
        "edges": chosen.iter().map(|e| json!({"from": m.nodes[e.0].name, "to": m.nodes[e.1].name, "distance": e.2})).collect::<Vec<_>>(),
        "connected": connected,
    })))
}
