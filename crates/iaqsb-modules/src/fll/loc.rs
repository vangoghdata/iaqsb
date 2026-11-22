//! Localización de instalaciones nuevas.

use super::{dist, dist_label, Distance, FacKind, FllModel, FllOptions};
use iaqsb_core::lp::{solve as lp_solve, Lp, LpStatus, Sense};
use iaqsb_core::*;
use nalgebra::{DMatrix, DVector};
use serde_json::json;

struct Prob {
    ex: Vec<usize>,
    nw: Vec<usize>,
    a: Vec<Vec<f64>>,
    /// w[j][i]: peso nueva j – existente i (flujo de ida y vuelta).
    w: Vec<Vec<f64>>,
    /// v[j][k]: peso entre nuevas.
    v: Vec<Vec<f64>>,
    d: usize,
}

fn total(p: &Prob, x: &[Vec<f64>], dm: Distance) -> f64 {
    let mut s = 0.0;
    for j in 0..p.nw.len() {
        for i in 0..p.ex.len() {
            s += p.w[j][i] * dist(dm, &x[j], &p.a[i]);
        }
        for k in j + 1..p.nw.len() {
            s += p.v[j][k] * dist(dm, &x[j], &x[k]);
        }
    }
    s
}

fn rectilinear(p: &Prob) -> Result<Vec<Vec<f64>>, String> {
    let n = p.nw.len();
    let mut x = vec![vec![0.0; p.d]; n];
    for dim in 0..p.d {
        let mut lp = Lp::new(0, false);
        let xs: Vec<usize> = (0..n).map(|_| lp.add_var(0.0, f64::NEG_INFINITY, f64::INFINITY, false)).collect();
        for j in 0..n {
            for i in 0..p.ex.len() {
                if p.w[j][i] != 0.0 {
                    let ep = lp.add_var(p.w[j][i], 0.0, f64::INFINITY, false);
                    let em = lp.add_var(p.w[j][i], 0.0, f64::INFINITY, false);
                    lp.row(vec![(xs[j], 1.0), (ep, -1.0), (em, 1.0)], Sense::Eq, p.a[i][dim]);
                }
            }
            for k in j + 1..n {
                if p.v[j][k] != 0.0 {
                    let ep = lp.add_var(p.v[j][k], 0.0, f64::INFINITY, false);
                    let em = lp.add_var(p.v[j][k], 0.0, f64::INFINITY, false);
                    lp.row(vec![(xs[j], 1.0), (xs[k], -1.0), (ep, -1.0), (em, 1.0)], Sense::Eq, 0.0);
                }
            }
        }
        let s = lp_solve(&lp);
        if s.status != LpStatus::Optimal {
            return Err("no se pudo resolver el programa lineal de la localización (¿hay flujos negativos?)".into());
        }
        for j in 0..n {
            x[j][dim] = tidy(s.x[xs[j]]);
        }
    }
    Ok(x)
}

fn squared(p: &Prob) -> Result<Vec<Vec<f64>>, String> {
    let n = p.nw.len();
    let mut a = DMatrix::<f64>::zeros(n, n);
    for j in 0..n {
        a[(j, j)] += p.w[j].iter().sum::<f64>();
        for k in 0..n {
            if k != j {
                let v = if j < k { p.v[j][k] } else { p.v[k][j] };
                a[(j, j)] += v;
                a[(j, k)] -= v;
            }
        }
    }
    let lu = a.lu();
    let mut x = vec![vec![0.0; p.d]; n];
    for dim in 0..p.d {
        let b = DVector::from_fn(n, |j, _| (0..p.ex.len()).map(|i| p.w[j][i] * p.a[i][dim]).sum::<f64>());
        let s = lu.solve(&b).ok_or("alguna instalación nueva no tiene flujo (directo o indirecto) con las existentes: su ubicación no queda determinada")?;
        for j in 0..n {
            x[j][dim] = s[j];
        }
    }
    Ok(x)
}

/// Distancia euclidiana: programa cónico de segundo orden (exacto) con Clarabel;
/// Weiszfeld como respaldo.
fn euclidean(p: &Prob, start: Vec<Vec<f64>>) -> Vec<Vec<f64>> {
    use clarabel::solver::{DefaultSettings, DefaultSolver, IPSolver, SolverStatus, SupportedConeT};
    let n = p.nw.len();
    let d = p.d;
    let nx = n * d;
    let mut terms: Vec<(usize, Option<usize>, Option<usize>, f64)> = vec![]; // (j, existente, nueva, peso)
    for j in 0..n {
        for i in 0..p.ex.len() {
            if p.w[j][i] > 0.0 {
                terms.push((j, Some(i), None, p.w[j][i]));
            }
        }
        for k in j + 1..n {
            if p.v[j][k] > 0.0 {
                terms.push((j, None, Some(k), p.v[j][k]));
            }
        }
    }
    let nv = nx + terms.len();
    let mut q = vec![0.0; nv];
    let mut rows: Vec<Vec<f64>> = vec![];
    let mut b = vec![];
    let mut cones = vec![];
    for (t, &(j, ei, nk, w)) in terms.iter().enumerate() {
        q[nx + t] = w;
        let mut r = vec![0.0; nv];
        r[nx + t] = -1.0;
        rows.push(r);
        b.push(0.0);
        for k in 0..d {
            let mut r = vec![0.0; nv];
            r[j * d + k] = -1.0;
            let mut bk = 0.0;
            if let Some(i) = ei {
                bk = -p.a[i][k];
            }
            if let Some(o) = nk {
                r[o * d + k] = 1.0;
            }
            rows.push(r);
            b.push(bk);
        }
        cones.push(SupportedConeT::SecondOrderConeT(d + 1));
    }
    let m = rows.len();
    let am = crate::qpsolve::csc(&rows, m, nv);
    let pm = crate::qpsolve::csc(&vec![vec![0.0; nv]; nv], nv, nv);
    let settings = DefaultSettings { verbose: false, max_iter: 500, tol_gap_abs: 1e-10, tol_gap_rel: 1e-10, tol_feas: 1e-10, ..Default::default() };
    if let Ok(mut solver) = DefaultSolver::new(&pm, &q, &am, &b, &cones, settings) {
        solver.solve();
        if matches!(solver.solution.status, SolverStatus::Solved | SolverStatus::AlmostSolved) {
            return (0..n).map(|j| (0..d).map(|k| solver.solution.x[j * d + k]).collect()).collect();
        }
    }
    weiszfeld(p, start)
}

fn weiszfeld(p: &Prob, start: Vec<Vec<f64>>) -> Vec<Vec<f64>> {
    let n = p.nw.len();
    let scale = p.a.iter().flatten().map(|v| v.abs()).fold(1.0, f64::max);
    let mut x = start;
    let mut eps = 1e-2 * scale;
    for _round in 0..8 {
        for _ in 0..4000 {
            let mut moved = 0.0f64;
            for j in 0..n {
                let mut num = vec![0.0; p.d];
                let mut den = 0.0;
                for i in 0..p.ex.len() {
                    if p.w[j][i] != 0.0 {
                        let dd = (dist(Distance::Squared, &x[j], &p.a[i]) + eps * eps).sqrt();
                        den += p.w[j][i] / dd;
                        for k in 0..p.d {
                            num[k] += p.w[j][i] * p.a[i][k] / dd;
                        }
                    }
                }
                for k2 in 0..n {
                    if k2 != j {
                        let v = if j < k2 { p.v[j][k2] } else { p.v[k2][j] };
                        if v != 0.0 {
                            let dd = (dist(Distance::Squared, &x[j], &x[k2]) + eps * eps).sqrt();
                            den += v / dd;
                            for k in 0..p.d {
                                num[k] += v * x[k2][k] / dd;
                            }
                        }
                    }
                }
                if den > 0.0 {
                    for k in 0..p.d {
                        let nv = num[k] / den;
                        moved = moved.max((nv - x[j][k]).abs());
                        x[j][k] = nv;
                    }
                }
            }
            if moved < 1e-12 * scale {
                break;
            }
        }
        eps /= 30.0;
    }
    x
}

/// Maximización dentro de la caja que forman las instalaciones existentes
/// (el máximo de una función convexa está en un vértice).
fn maximize(p: &Prob, dm: Distance) -> Vec<Vec<f64>> {
    let n = p.nw.len();
    let lo: Vec<f64> = (0..p.d).map(|k| p.a.iter().map(|a| a[k]).fold(f64::INFINITY, f64::min)).collect();
    let hi: Vec<f64> = (0..p.d).map(|k| p.a.iter().map(|a| a[k]).fold(f64::NEG_INFINITY, f64::max)).collect();
    let corners: Vec<Vec<f64>> = (0..(1usize << p.d)).map(|mask| (0..p.d).map(|k| if mask >> k & 1 == 1 { hi[k] } else { lo[k] }).collect()).collect();
    let mut best: Option<(f64, Vec<Vec<f64>>)> = None;
    for c0 in &corners {
        let mut x = vec![c0.clone(); n];
        loop {
            let mut improved = false;
            for j in 0..n {
                let cur = total(p, &x, dm);
                let mut bx = x[j].clone();
                let mut bv = cur;
                for c in &corners {
                    let old = std::mem::replace(&mut x[j], c.clone());
                    let v = total(p, &x, dm);
                    if v > bv + 1e-12 {
                        bv = v;
                        bx = c.clone();
                    }
                    x[j] = old;
                }
                if bv > cur + 1e-12 {
                    x[j] = bx;
                    improved = true;
                }
            }
            if !improved {
                break;
            }
        }
        let v = total(p, &x, dm);
        if best.as_ref().map(|b| v > b.0 + 1e-12).unwrap_or(true) {
            best = Some((v, x));
        }
    }
    best.unwrap().1
}

pub fn solve(m: &FllModel, o: &FllOptions) -> Result<Report, ModError> {
    let n = m.facilities.len();
    if n == 0 {
        return invalid("no hay instalaciones");
    }
    check_names("instalaciones", &m.facilities.iter().map(|f| &f.name).collect::<Vec<_>>())?;
    let d = if m.dims == 3 { 3 } else { 2 };
    let ex: Vec<usize> = (0..n).filter(|&i| m.facilities[i].kind == FacKind::Existing).collect();
    let nw: Vec<usize> = (0..n).filter(|&i| m.facilities[i].kind == FacKind::New).collect();
    if ex.is_empty() || nw.is_empty() {
        return invalid("hace falta al menos una instalación existente y una nueva");
    }
    let fl = |i: usize, j: usize| m.flows.get(i).and_then(|r| r.get(j)).copied().unwrap_or(0.0);
    if (0..n).any(|i| (0..n).any(|j| fl(i, j) < 0.0)) {
        return invalid("los flujos no pueden ser negativos");
    }
    let coord = |i: usize| {
        let f = &m.facilities[i];
        let mut v = vec![f.x, f.y];
        if d == 3 {
            v.push(f.z);
        }
        v
    };
    let p = Prob {
        a: ex.iter().map(|&i| coord(i)).collect(),
        w: nw.iter().map(|&j| ex.iter().map(|&i| fl(j, i) + fl(i, j)).collect()).collect(),
        v: nw.iter().map(|&j| nw.iter().map(|&k| fl(j, k) + fl(k, j)).collect()).collect(),
        ex: ex.clone(),
        nw: nw.clone(),
        d,
    };
    let dm = o.distance;
    let x = if m.maximize {
        maximize(&p, dm)
    } else {
        match dm {
            Distance::Rectilinear => rectilinear(&p).map_err(ModError::Invalid)?,
            Distance::Squared => squared(&p).map_err(ModError::Invalid)?,
            Distance::Euclidean => {
                let start = squared(&p).map_err(ModError::Invalid)?;
                euclidean(&p, start)
            }
        }
    };
    let x: Vec<Vec<f64>> = x.into_iter().map(|v| v.into_iter().map(tidy).collect()).collect();
    let names: Vec<&str> = m.facilities.iter().map(|f| f.name.as_str()).collect();
    let axes = ["X", "Y", "Z"];
    let mut head = vec!["Instalación nueva".to_string()];
    head.extend(axes[..d].iter().map(|s| s.to_string()));
    let mut ct = Table::new(Some("Ubicación óptima"), &head).left(0);
    for (j, &jj) in nw.iter().enumerate() {
        let mut row = vec![Cell::from(names[jj])];
        row.extend(x[j].iter().map(|v| Cell::Num(*v)));
        ct.row_tone(row, Tone::Highlight);
    }
    // detalle de costos
    let pos = |i: usize| -> Vec<f64> {
        match nw.iter().position(|&k| k == i) {
            Some(j) => x[j].clone(),
            None => coord(i),
        }
    };
    let mut bt = Table::new(Some("Costos por par de instalaciones"), &["Desde", "Hacia", "Flujo × costo", &format!("Distancia ({})", dist_label(dm)), "Costo"]).left(0).left(1);
    let (mut sub_ne, mut sub_nn, mut sub_ee) = (0.0, 0.0, 0.0);
    for i in 0..n {
        for j in 0..n {
            let f = fl(i, j);
            if f == 0.0 || i == j {
                continue;
            }
            let dd = dist(dm, &pos(i), &pos(j));
            let c = f * dd;
            let (ni, nj) = (m.facilities[i].kind == FacKind::New, m.facilities[j].kind == FacKind::New);
            match (ni, nj) {
                (false, false) => sub_ee += c,
                (true, true) => sub_nn += c,
                _ => sub_ne += c,
            }
            bt.row_tone(cells![names[i], names[j], f, tidy(dd), tidy(c)], if ni || nj { Tone::Normal } else { Tone::Muted });
        }
    }
    let tot = sub_ne + sub_nn + sub_ee;
    bt.row_tone(cells!["Nuevas con existentes", Cell::Empty, Cell::Empty, Cell::Empty, tidy(sub_ne)], Tone::Total);
    if sub_nn > 0.0 {
        bt.row_tone(cells!["Entre nuevas", Cell::Empty, Cell::Empty, Cell::Empty, tidy(sub_nn)], Tone::Total);
    }
    if sub_ee > 0.0 {
        bt.row_tone(cells!["Entre existentes", Cell::Empty, Cell::Empty, Cell::Empty, tidy(sub_ee)], Tone::Total);
    }
    bt.row_tone(cells!["Total", Cell::Empty, Cell::Empty, Cell::Empty, tidy(tot)], Tone::Total);
    // mapa
    let all: Vec<Vec<f64>> = (0..n).map(pos).collect();
    let (x0, x1) = all.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| (a.min(p[0]), b.max(p[0])));
    let (y0, y1) = all.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| (a.min(p[1]), b.max(p[1])));
    let sx = |v: f64| if x1 > x0 { (v - x0) / (x1 - x0) } else { 0.5 };
    let sy = |v: f64| if y1 > y0 { 1.0 - (v - y0) / (y1 - y0) } else { 0.5 };
    let nodes = (0..n)
        .map(|i| NetNode {
            id: format!("f{i}"),
            label: names[i].into(),
            x: sx(all[i][0]),
            y: sy(all[i][1]),
            note: Some(format!("({})", all[i].iter().map(|v| fmt(tidy(*v))).collect::<Vec<_>>().join(", "))),
            tone: if m.facilities[i].kind == FacKind::New { Tone::Highlight } else { Tone::Normal },
        })
        .collect();
    let mut edges = vec![];
    for i in 0..n {
        for j in i + 1..n {
            let f = fl(i, j) + fl(j, i);
            if f > 0.0 {
                edges.push(NetEdge { from: format!("f{i}"), to: format!("f{j}"), label: Some(fmt(f)), tone: if m.facilities[i].kind == FacKind::New || m.facilities[j].kind == FacKind::New { Tone::Accent } else { Tone::Muted } });
            }
        }
    }
    let map = Block::Network(Network { title: Some("Ubicaciones (X horizontal, Y vertical)".into()), directed: false, nodes, edges });
    let mut st = vec![];
    if o.steps {
        let mut v = vec![];
        if m.maximize {
            v.push(step("Maximización", "Se busca alejar las instalaciones nuevas dentro de la caja que forman las existentes: el costo es convexo, así que el máximo está en un vértice. Se prueban los vértices para cada instalación nueva hasta que ninguna mejora.", vec![]));
        } else {
            match dm {
                Distance::Rectilinear => {
                    v.push(step("Distancia rectilínea", "|Δx| + |Δy| se separa por coordenadas. En cada eje el problema es lineal: se resuelve con programación lineal (con una sola instalación nueva, la solución es la mediana ponderada).", vec![]));
                    if nw.len() == 1 {
                        let mut tbs = vec![];
                        for k in 0..d {
                            let mut ord: Vec<usize> = (0..ex.len()).collect();
                            ord.sort_by(|&a, &b| p.a[a][k].partial_cmp(&p.a[b][k]).unwrap());
                            let tw: f64 = p.w[0].iter().sum();
                            let mut acc = 0.0;
                            let mut t = Table::new(Some(&format!("Mediana ponderada en {}", axes[k])), &["Instalación", axes[k], "Peso", "Peso acumulado"]).left(0);
                            let mut marked = false;
                            for &i in &ord {
                                acc += p.w[0][i];
                                let hit = !marked && acc >= tw / 2.0 - 1e-12;
                                if hit {
                                    marked = true;
                                }
                                t.row_tone(cells![names[ex[i]], p.a[i][k], p.w[0][i], acc], if hit { Tone::Highlight } else { Tone::Normal });
                            }
                            tbs.push(t.block());
                        }
                        v.push(step("Mediana ponderada", "Se ordenan las existentes por coordenada; la mediana es la primera donde el peso acumulado alcanza la mitad del total.", tbs));
                    }
                }
                Distance::Squared => v.push(step("Distancia euclidiana al cuadrado", "Derivando e igualando a cero, cada coordenada es un promedio ponderado (con varias instalaciones nuevas, un sistema lineal). Con una sola: x* = Σ wᵢ·aᵢ / Σ wᵢ.", vec![])),
                Distance::Euclidean => v.push(step("Distancia euclidiana", "El problema es convexo pero no lineal. La idea clásica es la de Weiszfeld: repetir x = Σ (wᵢ·aᵢ/dᵢ) / Σ (wᵢ/dᵢ), con dᵢ la distancia actual, hasta que deja de cambiar. IAQSB lo resuelve exacto como programa cónico de segundo orden (cada distancia es una restricción ‖x − aᵢ‖ ≤ tᵢ) con Clarabel.", vec![])),
            }
        }
        v.push(step("Costo total", "Σ flujo × costo × distancia sobre todos los pares.", vec![bt.clone().block()]));
        st.push(steps(v));
    }
    let head_txt = nw.iter().enumerate().map(|(j, &jj)| format!("{} en ({})", names[jj], x[j].iter().map(|v| fmt(*v)).collect::<Vec<_>>().join(", "))).collect::<Vec<_>>().join("; ");
    Ok(Report::new("fll", &m.name, if m.maximize { Status::Solved } else { Status::Optimal }, format!("{head_txt}; costo total {}.", fmt(tidy(tot))))
        .section("solucion", "Solución", vec![metrics(vec![metric_main("Costo total", tidy(tot)), metric("Distancia", dist_label(dm))]), ct.block(), bt.block()])
        .section("mapa", "Mapa", vec![map])
        .section("pasos", "Paso a paso", st)
        .data(json!({"locations": nw.iter().enumerate().map(|(j, &jj)| json!({"name": names[jj], "coordinates": x[j]})).collect::<Vec<_>>(), "total_cost": tidy(tot)})))
}
