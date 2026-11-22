//! Agente viajero: óptimo con HiGHS y cortes de subciclos (Dantzig-Fulkerson-
//! Johnson), o heurísticas de clase: vecino más cercano, inserción más barata y
//! mejora 2-opt.

use super::layout::node_network;
use super::{NetModel, NetOptions, TspMethod};
use iaqsb_core::lp::{self, Lp, LpStatus, Sense};
use iaqsb_core::report::*;
use iaqsb_core::{cells, ModError};
use serde_json::json;

fn tour_cost(d: &[Vec<Option<f64>>], tour: &[usize]) -> Option<f64> {
    let mut s = 0.0;
    for k in 0..tour.len() {
        s += d[tour[k]][tour[(k + 1) % tour.len()]]?;
    }
    Some(s)
}

fn cycles(next: &[usize]) -> Vec<Vec<usize>> {
    let n = next.len();
    let mut seen = vec![false; n];
    let mut out = vec![];
    for s in 0..n {
        if seen[s] {
            continue;
        }
        let mut c = vec![];
        let mut x = s;
        while !seen[x] {
            seen[x] = true;
            c.push(x);
            x = next[x];
        }
        out.push(c);
    }
    out
}

pub fn solve(m: &NetModel, o: &NetOptions) -> Result<Report, ModError> {
    let n = m.nodes.len();
    let is_max = m.objective.is_max();
    let d: Vec<Vec<Option<f64>>> = (0..n).map(|i| (0..n).map(|j| if i == j { None } else { m.arc(i, j) }).collect()).collect();
    let better = |a: f64, b: f64| if is_max { a > b + 1e-12 } else { a < b - 1e-12 };
    let name = |i: usize| m.nodes[i].name.clone();
    let mut st: Vec<Step> = vec![];
    let show_steps = o.steps && n <= 40;
    let mut method: String;

    let tour: Option<Vec<usize>> = match o.tsp_method {
        TspMethod::Exact => {
            method = "óptimo: programación entera con HiGHS y cortes de subciclos".into();
            let mut lpm = Lp::new(0, is_max);
            let mut var = vec![vec![None; n]; n];
            for i in 0..n {
                for j in 0..n {
                    if let Some(c) = d[i][j] {
                        var[i][j] = Some(lpm.add_var(c, 0.0, 1.0, true));
                    }
                }
            }
            for i in 0..n {
                lpm.row((0..n).filter_map(|j| var[i][j].map(|k| (k, 1.0))).collect(), Sense::Eq, 1.0);
                lpm.row((0..n).filter_map(|j| var[j][i].map(|k| (k, 1.0))).collect(), Sense::Eq, 1.0);
            }
            lpm.time_limit = Some(120.0);
            let mut result = None;
            for round in 1..=500 {
                let s = lp::solve(&lpm);
                if s.status != LpStatus::Optimal {
                    break;
                }
                let mut next = vec![0usize; n];
                for i in 0..n {
                    for j in 0..n {
                        if let Some(k) = var[i][j] {
                            if s.x[k] > 0.5 {
                                next[i] = j;
                            }
                        }
                    }
                }
                let cs = cycles(&next);
                if show_steps {
                    let desc = if cs.len() == 1 {
                        format!("La solución es un único recorrido: es óptima ({}).", fmt(s.objective))
                    } else {
                        format!(
                            "La solución ({}) tiene {} subciclos: {}. Se añade un corte por cada uno que prohíbe cerrarlo.",
                            fmt(s.objective),
                            cs.len(),
                            cs.iter().map(|c| format!("[{}]", c.iter().map(|&i| name(i)).collect::<Vec<_>>().join("→"))).collect::<Vec<_>>().join(", ")
                        )
                    };
                    let hl: Vec<(usize, usize)> = (0..n).map(|i| (i, next[i])).collect();
                    st.push(step(format!("Ronda {round}"), desc, vec![node_network(m, "Solución de la ronda", 0, &hl, &|_, _, c| Some(fmt(c)), &|_| None, true)]));
                }
                if cs.len() == 1 {
                    result = Some(cs.into_iter().next().unwrap());
                    break;
                }
                for c in cs {
                    let mut coefs = vec![];
                    for &i in &c {
                        for &j in &c {
                            if let Some(k) = var[i][j] {
                                coefs.push((k, 1.0));
                            }
                        }
                    }
                    lpm.row(coefs, Sense::Le, c.len() as f64 - 1.0);
                }
            }
            result
        }
        TspMethod::NearestNeighbor | TspMethod::TwoOpt => {
            method = "vecino más cercano".into();
            let mut tour = vec![0usize];
            let mut used = vec![false; n];
            used[0] = true;
            let mut ok = true;
            let word = if is_max { "lejano" } else { "cercano" };
            let mut summary = Table::new(Some("Resumen del vecino más cercano"), &["Paso", "Desde", "Va a", "Distancia", "Acumulada"]).left(1).left(2);
            let mut acc = 0.0;
            if show_steps {
                st.push(step(
                    format!("Inicio en {}", name(0)),
                    format!("Se sale del nodo {} (el depósito) y, en cada paso, se va al nodo sin visitar más {}. Al final se regresa al inicio (Rosenkrantz et al., 1977).", name(0), word),
                    vec![node_network(m, "Red completa", 0, &[], &|_, _, c| Some(fmt(c)), &|_| None, false)],
                ));
            }
            for k in 1..n {
                let last = *tour.last().unwrap();
                let cand = (0..n).filter(|&j| !used[j] && d[last][j].is_some()).fold(None::<usize>, |acc, j| match acc {
                    Some(k) if !better(d[last][j].unwrap(), d[last][k].unwrap()) => Some(k),
                    _ => Some(j),
                });
                let Some(j) = cand else {
                    ok = false;
                    break;
                };
                used[j] = true;
                tour.push(j);
                acc += d[last][j].unwrap();
                summary.row(cells![k, name(last), name(j), d[last][j].unwrap(), acc]);
                if show_steps {
                    // todas las opciones que se compararon
                    let mut ct = Table::new(Some(&format!("Nodos sin visitar desde {}", name(last))), &["Nodo", "Distancia", ""]).left(0).left(2);
                    let best = d[last][j].unwrap();
                    let mut ties = 0;
                    for c in (0..n).filter(|&c| !tour[..tour.len() - 1].contains(&c) && c != last) {
                        match d[last][c] {
                            Some(v) => {
                                if (v - best).abs() < 1e-12 {
                                    ties += 1;
                                }
                                ct.row_tone(cells![name(c), v, if c == j { "← el más ".to_string() + word } else { String::new() }], if c == j { Tone::Highlight } else { Tone::Normal });
                            }
                            None => {
                                ct.row_tone(cells![name(c), "sin arco", ""], Tone::Muted);
                            }
                        }
                    }
                    let path: Vec<(usize, usize)> = tour.windows(2).map(|w| (w[0], w[1])).collect();
                    st.push(step(
                        format!("Paso {k}: desde {} a {}", name(last), name(j)),
                        format!(
                            "El nodo sin visitar más {word} a {} es {} ({}).{} Distancia acumulada: {}. Recorrido parcial: {}.",
                            name(last),
                            name(j),
                            fmt(best),
                            if ties > 1 { " Hay empate: se toma el primero de la lista." } else { "" },
                            fmt(acc),
                            tour.iter().map(|&i| name(i)).collect::<Vec<_>>().join(" → ")
                        ),
                        vec![ct.block(), node_network(m, &format!("Recorrido parcial después del paso {k}"), 0, &path, &|_, _, c| Some(fmt(c)), &|_| None, false)],
                    ));
                }
            }
            if ok {
                if let Some(back) = d[*tour.last().unwrap()][0] {
                    acc += back;
                    summary.row_tone(cells![n, name(*tour.last().unwrap()), name(0), back, acc], Tone::Total);
                    if show_steps {
                        let mut path: Vec<(usize, usize)> = tour.windows(2).map(|w| (w[0], w[1])).collect();
                        path.push((*tour.last().unwrap(), 0));
                        st.push(step(
                            format!("Regreso a {}", name(0)),
                            format!("Ya se visitaron todos los nodos: se regresa de {} a {} ({}). Distancia total del vecino más cercano: {}.", name(*tour.last().unwrap()), name(0), fmt(back), fmt(acc)),
                            vec![summary.clone().block(), node_network(m, "Recorrido del vecino más cercano", 0, &path, &|_, _, c| Some(fmt(c)), &|_| None, false)],
                        ));
                    }
                }
            }
            if ok && d[*tour.last().unwrap()][0].is_some() {
                if o.tsp_method == TspMethod::TwoOpt {
                    method = "vecino más cercano mejorado con 2-opt".into();
                    let mut improved = true;
                    let mut cur = tour_cost(&d, &tour).unwrap();
                    while improved {
                        improved = false;
                        'outer: for a in 1..n - 1 {
                            for b in a + 1..n {
                                let mut t2 = tour.clone();
                                t2[a..=b].reverse();
                                if let Some(c2) = tour_cost(&d, &t2) {
                                    if better(c2, cur) {
                                        if show_steps {
                                            let path: Vec<(usize, usize)> = (0..n).map(|q| (t2[q], t2[(q + 1) % n])).collect();
                                            st.push(step(
                                                format!("2-opt: invertir de {} a {}", name(tour[a]), name(tour[b])),
                                                format!("Se quitan los arcos {}–{} y {}–{} y se reconecta invirtiendo el tramo: la distancia pasa de {} a {} (Croes, 1958).", name(tour[a - 1]), name(tour[a]), name(tour[b]), name(tour[(b + 1) % n]), fmt(cur), fmt(c2)),
                                                vec![node_network(m, "Recorrido mejorado", 0, &path, &|_, _, c| Some(fmt(c)), &|_| None, false)],
                                            ));
                                        }
                                        tour = t2;
                                        cur = c2;
                                        improved = true;
                                        break 'outer;
                                    }
                                }
                            }
                        }
                    }
                }
                Some(tour)
            } else {
                None
            }
        }
        TspMethod::CheapestInsertion => {
            method = "inserción más barata".into();
            // empezar con el ciclo de dos nodos 0 → k → 0 más barato
            let start = (1..n).filter(|&k| d[0][k].is_some() && d[k][0].is_some()).fold(None::<usize>, |acc, k| {
                let c = d[0][k].unwrap() + d[k][0].unwrap();
                match acc {
                    Some(j) if !better(c, d[0][j].unwrap() + d[j][0].unwrap()) => Some(j),
                    _ => Some(k),
                }
            });
            match start {
                None => None,
                Some(k0) => {
                    let mut tour = vec![0, k0];
                    let mut ok = true;
                    if show_steps {
                        let mut it = Table::new(Some(&format!("Subtour inicial: {} con cada nodo", name(0))), &["Nodo k", &format!("{0} → k → {0}", name(0)), ""]).left(0).left(2);
                        for k in 1..n {
                            match (d[0][k], d[k][0]) {
                                (Some(a), Some(b)) => {
                                    it.row_tone(cells![name(k), a + b, if k == k0 { "← el más barato" } else { "" }], if k == k0 { Tone::Highlight } else { Tone::Normal });
                                }
                                _ => {
                                    it.row_tone(cells![name(k), "sin arco", ""], Tone::Muted);
                                }
                            }
                        }
                        st.push(step(
                            "Subtour inicial",
                            format!(
                                "Se empieza con el ciclo más {} entre {} y otro nodo: {} → {} → {} ({}). Después, en cada paso se inserta el nodo y la posición que menos aumentan la distancia: costo de insertar k entre a y b = d(a,k) + d(k,b) − d(a,b) (Rosenkrantz et al., 1977).",
                                if is_max { "largo" } else { "corto" },
                                name(0),
                                name(0),
                                name(k0),
                                name(0),
                                fmt(d[0][k0].unwrap() + d[k0][0].unwrap())
                            ),
                            vec![it.block(), node_network(m, "Subtour inicial", 0, &[(0, k0), (k0, 0)], &|_, _, c| Some(fmt(c)), &|_| None, false)],
                        ));
                    }
                    let mut summary = Table::new(Some("Resumen de la inserción más barata"), &["Paso", "Nodo insertado", "Entre", "Aumento", "Distancia del subtour"]).left(1).left(2);
                    let mut cur = d[0][k0].unwrap() + d[k0][0].unwrap();
                    let mut stepn = 0;
                    while tour.len() < n {
                        let mut best: Option<(f64, usize, usize)> = None;
                        let edges: Vec<(usize, usize)> = (0..tour.len()).map(|p| (tour[p], tour[(p + 1) % tour.len()])).collect();
                        let mut costs: Vec<(usize, Vec<Option<f64>>)> = vec![];
                        for k in 0..n {
                            if tour.contains(&k) {
                                continue;
                            }
                            let mut row = vec![];
                            for (p, &(a, b)) in edges.iter().enumerate() {
                                if let (Some(x), Some(y), Some(z)) = (d[a][k], d[k][b], d[a][b]) {
                                    let inc = x + y - z;
                                    row.push(Some(inc));
                                    if best.map_or(true, |bb| better(inc, bb.0)) {
                                        best = Some((inc, k, p + 1));
                                    }
                                } else {
                                    row.push(None);
                                }
                            }
                            costs.push((k, row));
                        }
                        let Some((inc, k, pos)) = best else {
                            ok = false;
                            break;
                        };
                        stepn += 1;
                        let a = tour[pos - 1];
                        let b = tour[pos % tour.len()];
                        cur += inc;
                        summary.row(cells![stepn, name(k), format!("{} y {}", name(a), name(b)), inc, cur]);
                        if show_steps {
                            let mut head = vec!["Nodo k".to_string()];
                            head.extend(edges.iter().map(|&(x, y)| format!("{}–{}", name(x), name(y))));
                            let mut ct = Table::new(Some("Costo de insertar cada nodo en cada arco del subtour"), &head).left(0);
                            for (kk, row) in &costs {
                                let mut r = vec![Cell::from(name(*kk))];
                                for (p, v) in row.iter().enumerate() {
                                    r.push(match v {
                                        Some(x) if *kk == k && p + 1 == pos => Cell::from(format!("{} ✓", fmt(*x))),
                                        Some(x) => Cell::Num(*x),
                                        None => Cell::from("—"),
                                    });
                                }
                                ct.row_tone(r, if *kk == k { Tone::Highlight } else { Tone::Normal });
                            }
                            let mut t2 = tour.clone();
                            t2.insert(pos, k);
                            let path: Vec<(usize, usize)> = (0..t2.len()).map(|q| (t2[q], t2[(q + 1) % t2.len()])).collect();
                            st.push(step(
                                format!("Paso {stepn}: insertar {}", name(k)),
                                format!(
                                    "El menor aumento es insertar {} entre {} y {}: {} + {} − {} = {}.{} El subtour queda {} con distancia {}.",
                                    name(k),
                                    name(a),
                                    name(b),
                                    fmt(d[a][k].unwrap()),
                                    fmt(d[k][b].unwrap()),
                                    fmt(d[a][b].unwrap()),
                                    fmt(inc),
                                    {
                                        let ties = costs.iter().flat_map(|(_, r)| r.iter()).filter(|v| v.is_some_and(|x| (x - inc).abs() < 1e-12)).count();
                                        if ties > 1 { format!(" Hay {ties} opciones con el mismo aumento: se toma la primera (el nodo de menor número y el primer arco del subtour); con otra elección el resultado puede cambiar.") } else { String::new() }
                                    },
                                    t2.iter().chain(std::iter::once(&t2[0])).map(|&i| name(i)).collect::<Vec<_>>().join(" → "),
                                    fmt(cur)
                                ),
                                vec![ct.block(), node_network(m, &format!("Subtour después del paso {stepn}"), 0, &path, &|_, _, c| Some(fmt(c)), &|_| None, false)],
                            ));
                        }
                        tour.insert(pos, k);
                    }
                    if ok && show_steps {
                        st.push(step("Resumen", format!("Distancia total de la inserción más barata: {}.", fmt(cur)), vec![summary.block()]));
                    }
                    if ok {
                        Some(tour)
                    } else {
                        None
                    }
                }
            }
        }
    };

    let Some(tour) = tour else {
        return Ok(Report::new("net", &m.name, Status::Infeasible, "No se encontró ningún recorrido que pase por todos los nodos con los arcos existentes.")
            .section("solucion", "Solución", vec![text_tone("Con las conexiones de la red no hay un ciclo que visite todos los nodos (o la heurística no lo encontró).", Tone::Bad)]));
    };
    let total = tour_cost(&d, &tour).unwrap();
    let mut t = Table::new(None, &["Desde", "Hasta", "Distancia", "Acumulada"]).left(1);
    let mut acc = 0.0;
    let mut hl = vec![];
    for k in 0..n {
        let (a, b) = (tour[k], tour[(k + 1) % n]);
        let c = d[a][b].unwrap();
        acc += c;
        hl.push((a, b));
        t.row(cells![name(a), name(b), c, acc]);
    }
    t.row_tone(cells!["Total", "", total, ""], Tone::Total);
    let route: Vec<String> = tour.iter().chain(std::iter::once(&tour[0])).map(|&i| name(i)).collect();
    let exact = o.tsp_method == TspMethod::Exact;
    let mut rep = Report::new(
        "net",
        &m.name,
        if exact { Status::Optimal } else { Status::Feasible },
        format!("Recorrido {}: {} ({}).", if exact { "óptimo" } else { "heurístico" }, fmt(total), route.join(" → ")),
    )
    .section("solucion", "Solución", vec![
        metrics(vec![metric_main("Distancia total", total), metric("Nodos", n)]),
        text(format!(
            "Recorrido: {}. Método: {method}.{}",
            route.join(" → "),
            if exact { "" } else { " Las heurísticas no garantizan el óptimo; usa el método óptimo para comprobarlo." }
        )),
        t.block(),
    ])
    .section("red", "Red", vec![node_network(m, "Recorrido", 0, &hl, &|_, _, c| Some(fmt(c)), &|_| None, true)]);
    if !st.is_empty() {
        rep = rep.section("pasos", "Paso a paso", vec![steps(st)]);
    }
    let libro = o.mtz_form.trim().eq_ignore_ascii_case("libro");
    let lpt = super::lpmodel::tsp(m, libro);
    let k = m.node_index(&o.subtour_node, 0)?;
    let (sub, nsub) = super::lpmodel::subtours_from(m, k);
    let forma = if libro {
        format!(
            "U(i) − U(j) + n·x(i,j) ≤ n − 1, con 1 ≤ U(i) ≤ n − 1 y n = {n}: la forma que traen muchos libros de texto. Es válida, pero un poco más floja que la de la clase"
        )
    } else {
        format!("U(i) − U(j) + 1 ≤ (n − 1)(1 − x(i,j)), es decir U(i) − U(j) + (n − 1)·x(i,j) ≤ n − 2, con 2 ≤ U(i) ≤ n: la forma de las diapositivas de la clase ({}), que es la restricción original de {} escrita con n = todos los nodos", iaqsb_core::biblio::c("sarache2026ruteo"), iaqsb_core::biblio::cn("miller1960"))
    };
    let mut model = lpt.blocks(
        &format!(
            "x(i,j) = 1 si el recorrido va de i a j: de cada nodo sale un solo vehículo y a cada nodo llega uno solo ({}). Para que no queden subtours se usa la relajación de Miller-Tucker-Zemlin: {forma}. Se escribe para cada par de nodos distintos del primero (no entran la fila ni la columna del nodo {}): (n − 1)(n − 2) = {} restricciones en lugar de una por cada subtour posible. IAQSB calcula el óptimo agregando solo los cortes de los subtours que aparecen, como {} (ver Paso a paso); todos estos modelos dan la misma distancia.",
            iaqsb_core::biblio::c("sarache2026ruteo"),
            m.nodes[0].name,
            (n - 1) * (n.saturating_sub(2)),
            iaqsb_core::biblio::cn("dantzig1954")
        ),
        n <= 10,
    );
    model.push(text(format!(
        "Sin relajación, el rompimiento de subtours se escribe con cada subtour que pasa por un nodo ({}). Desde el nodo {} hay {} ecuaciones: una por cada par de nodos (a, b) y cada sentido del recorrido {}-a-b-{}.",
        iaqsb_core::biblio::c("sarache2026ruteo"),
        m.nodes[k].name,
        nsub,
        m.nodes[k].name,
        m.nodes[k].name
    )));
    if n <= 16 {
        model.push(sub);
    }
    rep = rep.section("modelo", "Modelo matemático", model);
    Ok(rep.data(json!({"distance": clean(total), "tour": route, "method": method, "optimal": exact, "lp_model": lpt.json()})))
}
