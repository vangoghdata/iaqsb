//! Problema de transporte.
//!
//! Resultado: método de transporte (MODI / u-v) con una solución inicial a
//! elegir (esquina noroeste, costo mínimo, Vogel o Russell), comprobado con HiGHS.
//! El paso a paso muestra la solución inicial y cada iteración con u(i), v(j),
//! los costes reducidos, el ciclo, θ y la celda que sale.

use super::layout::bipartite_network;
use super::{InitialMethod, NetModel, NetOptions};
use iaqsb_core::lp::{self, Lp, LpStatus, Sense};
use iaqsb_core::report::*;
use iaqsb_core::{cells, ModError};
use serde_json::json;
use std::collections::VecDeque;

const EPS: f64 = 1e-9;

pub(crate) struct Tp {
    pub m: usize,
    pub n: usize,
    /// Costes internos (minimización); las celdas prohibidas valen `big`.
    pub cost: Vec<Vec<f64>>,
    pub prohibited: Vec<Vec<bool>>,
    pub supply: Vec<f64>,
    pub demand: Vec<f64>,
    pub rows: Vec<String>,
    pub cols: Vec<String>,
    pub dummy_row: bool,
    pub dummy_col: bool,
    /// -1 si el problema original es de maximizar.
    pub sign: f64,
    pub big: f64,
    /// Costes originales para mostrar (None = prohibida).
    pub shown: Vec<Vec<Option<f64>>>,
}

pub(crate) fn build(md: &NetModel) -> Tp {
    let sign = if md.objective.is_max() { -1.0 } else { 1.0 };
    let mut supply: Vec<f64> = md.sources.iter().map(|s| s.supply).collect();
    let mut demand: Vec<f64> = md.destinations.iter().map(|d| d.demand).collect();
    let mut rows: Vec<String> = md.sources.iter().map(|s| s.name.clone()).collect();
    let mut cols: Vec<String> = md.destinations.iter().map(|d| d.name.clone()).collect();
    let mut shown = md.costs.clone();
    let maxc = md.costs.iter().flatten().flatten().fold(1.0f64, |a, b| a.max(b.abs()));
    let big = 1e6 * maxc;
    let (ts, td): (f64, f64) = (supply.iter().sum(), demand.iter().sum());
    let mut dummy_row = false;
    let mut dummy_col = false;
    if ts > td + EPS {
        demand.push(ts - td);
        cols.push("Ficticio".into());
        for r in shown.iter_mut() {
            r.push(Some(0.0));
        }
        dummy_col = true;
    } else if td > ts + EPS {
        supply.push(td - ts);
        rows.push("Ficticio".into());
        shown.push(vec![Some(0.0); cols.len()]);
        dummy_row = true;
    }
    let m = rows.len();
    let n = cols.len();
    let mut cost = vec![vec![0.0; n]; m];
    let mut prohibited = vec![vec![false; n]; m];
    for i in 0..m {
        for j in 0..n {
            match shown[i][j] {
                Some(c) => cost[i][j] = sign * c,
                None => {
                    cost[i][j] = big;
                    prohibited[i][j] = true;
                }
            }
        }
    }
    Tp { m, n, cost, prohibited, supply, demand, rows, cols, dummy_row, dummy_col, sign, big, shown }
}

pub(crate) struct Sol {
    pub x: Vec<Vec<f64>>,
    pub basic: Vec<Vec<bool>>,
}

impl Sol {
    fn new(m: usize, n: usize) -> Sol {
        Sol { x: vec![vec![0.0; n]; m], basic: vec![vec![false; n]; m] }
    }
    fn count(&self) -> usize {
        self.basic.iter().flatten().filter(|&&b| b).count()
    }
}

struct Alloc {
    cell: (usize, usize),
    qty: f64,
    why: String,
}

/// Solución inicial con el método elegido. Devuelve también la explicación de cada asignación.
fn initial(tp: &Tp, method: InitialMethod) -> (Sol, Vec<Alloc>) {
    let (m, n) = (tp.m, tp.n);
    let mut s = tp.supply.clone();
    let mut d = tp.demand.clone();
    let mut row_done = vec![false; m];
    let mut col_done = vec![false; n];
    let mut sol = Sol::new(m, n);
    let mut log = Vec::new();
    let mut remaining = m + n;
    let mut ni = 0usize; // esquina noroeste
    let mut nj = 0usize;
    while remaining > 1 {
        let open_rows: Vec<usize> = (0..m).filter(|&i| !row_done[i]).collect();
        let open_cols: Vec<usize> = (0..n).filter(|&j| !col_done[j]).collect();
        if open_rows.is_empty() || open_cols.is_empty() {
            break;
        }
        let (cell, why) = match method {
            InitialMethod::NorthwestCorner => {
                while row_done[ni] {
                    ni += 1;
                }
                while col_done[nj] {
                    nj += 1;
                }
                ((ni, nj), "esquina noroeste de lo que queda".to_string())
            }
            InitialMethod::MinimumCost => {
                let mut best = (open_rows[0], open_cols[0]);
                for &i in &open_rows {
                    for &j in &open_cols {
                        let (bi, bj) = best;
                        let better = tp.cost[i][j] < tp.cost[bi][bj] - EPS
                            || ((tp.cost[i][j] - tp.cost[bi][bj]).abs() <= EPS && s[i].min(d[j]) > s[bi].min(d[bj]) + EPS);
                        if better {
                            best = (i, j);
                        }
                    }
                }
                (best, format!("menor costo disponible ({})", show_cost(tp, best)))
            }
            InitialMethod::Vogel => {
                let pen = |costs: Vec<f64>| -> f64 {
                    let mut c = costs;
                    c.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    if c.len() >= 2 {
                        c[1] - c[0]
                    } else {
                        c.first().copied().unwrap_or(0.0)
                    }
                };
                let mut best: Option<(f64, f64, bool, usize)> = None; // (penalización, costo mín, es fila, índice)
                for &i in &open_rows {
                    let p = pen(open_cols.iter().map(|&j| tp.cost[i][j]).collect());
                    let mc = open_cols.iter().map(|&j| tp.cost[i][j]).fold(f64::INFINITY, f64::min);
                    if best.map_or(true, |b| p > b.0 + EPS || ((p - b.0).abs() <= EPS && mc < b.1 - EPS)) {
                        best = Some((p, mc, true, i));
                    }
                }
                for &j in &open_cols {
                    let p = pen(open_rows.iter().map(|&i| tp.cost[i][j]).collect());
                    let mc = open_rows.iter().map(|&i| tp.cost[i][j]).fold(f64::INFINITY, f64::min);
                    if best.map_or(true, |b| p > b.0 + EPS || ((p - b.0).abs() <= EPS && mc < b.1 - EPS)) {
                        best = Some((p, mc, false, j));
                    }
                }
                let (p, _, is_row, k) = best.unwrap();
                let cell = if is_row {
                    let j = *open_cols.iter().min_by(|&&a, &&b| tp.cost[k][a].partial_cmp(&tp.cost[k][b]).unwrap()).unwrap();
                    (k, j)
                } else {
                    let i = *open_rows.iter().min_by(|&&a, &&b| tp.cost[a][k].partial_cmp(&tp.cost[b][k]).unwrap()).unwrap();
                    (i, k)
                };
                let line = if is_row { format!("fila {}", tp.rows[k]) } else { format!("columna {}", tp.cols[k]) };
                (cell, format!("mayor penalización: {} en la {line}", fmt_cost(tp, p)))
            }
            InitialMethod::Russell => {
                let ubar: Vec<f64> = (0..m).map(|i| open_cols.iter().map(|&j| tp.cost[i][j]).fold(f64::NEG_INFINITY, f64::max)).collect();
                let vbar: Vec<f64> = (0..n).map(|j| open_rows.iter().map(|&i| tp.cost[i][j]).fold(f64::NEG_INFINITY, f64::max)).collect();
                let mut best = (open_rows[0], open_cols[0]);
                let delta = |i: usize, j: usize| tp.cost[i][j] - ubar[i] - vbar[j];
                for &i in &open_rows {
                    for &j in &open_cols {
                        if delta(i, j) < delta(best.0, best.1) - EPS {
                            best = (i, j);
                        }
                    }
                }
                (best, format!("Δ más negativo ({})", fmt_cost(tp, delta(best.0, best.1))))
            }
        };
        let (i, j) = cell;
        let q = s[i].min(d[j]);
        sol.x[i][j] += q;
        sol.basic[i][j] = true;
        s[i] -= q;
        d[j] -= q;
        log.push(Alloc { cell, qty: q, why });
        // se tacha una sola línea; si se agotan ambas, la fila (la columna queda con 0)
        if s[i] <= EPS && (d[j] > EPS || remaining > 2) && !(d[j] <= EPS && open_rows.len() == 1) {
            row_done[i] = true;
        } else {
            col_done[j] = true;
        }
        remaining -= 1;
    }
    complete_basis(tp, &mut sol);
    (sol, log)
}

/// Añade celdas básicas con valor 0 hasta tener m + n − 1 (degeneración).
fn complete_basis(tp: &Tp, sol: &mut Sol) {
    let (m, n) = (tp.m, tp.n);
    let mut parent: Vec<usize> = (0..m + n).collect();
    fn find(p: &mut Vec<usize>, x: usize) -> usize {
        let mut r = x;
        while p[r] != r {
            r = p[r];
        }
        let mut y = x;
        while p[y] != r {
            let nx = p[y];
            p[y] = r;
            y = nx;
        }
        r
    }
    // quitar celdas que formen ciclo (no debería ocurrir) y unir componentes
    for i in 0..m {
        for j in 0..n {
            if sol.basic[i][j] {
                let (a, b) = (find(&mut parent, i), find(&mut parent, m + j));
                if a == b {
                    sol.basic[i][j] = sol.x[i][j] > EPS;
                } else {
                    parent[a] = b;
                }
            }
        }
    }
    let mut cand: Vec<(usize, usize)> = (0..m).flat_map(|i| (0..n).map(move |j| (i, j))).filter(|&(i, j)| !sol.basic[i][j]).collect();
    cand.sort_by(|a, b| tp.cost[a.0][a.1].partial_cmp(&tp.cost[b.0][b.1]).unwrap());
    for (i, j) in cand {
        if sol.count() >= m + n - 1 {
            break;
        }
        let (a, b) = (find(&mut parent, i), find(&mut parent, m + j));
        if a != b {
            parent[a] = b;
            sol.basic[i][j] = true;
        }
    }
}

/// Potenciales u(i), v(j) con u(0) = 0.
fn potentials(tp: &Tp, sol: &Sol) -> (Vec<f64>, Vec<f64>) {
    let (m, n) = (tp.m, tp.n);
    let mut u = vec![f64::NAN; m];
    let mut v = vec![f64::NAN; n];
    u[0] = 0.0;
    let mut q = VecDeque::from([(true, 0usize)]);
    while let Some((is_row, k)) = q.pop_front() {
        if is_row {
            for j in 0..n {
                if sol.basic[k][j] && v[j].is_nan() {
                    v[j] = tp.cost[k][j] - u[k];
                    q.push_back((false, j));
                }
            }
        } else {
            for i in 0..m {
                if sol.basic[i][k] && u[i].is_nan() {
                    u[i] = tp.cost[i][k] - v[k];
                    q.push_back((true, i));
                }
            }
        }
    }
    // componentes sueltas (no debería ocurrir con una base completa)
    for x in u.iter_mut().chain(v.iter_mut()) {
        if x.is_nan() {
            *x = 0.0;
        }
    }
    (u, v)
}

/// Celdas del camino en el árbol básico desde la columna j hasta la fila i.
fn tree_path(tp: &Tp, sol: &Sol, i: usize, j: usize) -> Vec<(usize, usize)> {
    let (m, n) = (tp.m, tp.n);
    let total = m + n;
    let mut prev: Vec<Option<usize>> = vec![None; total];
    let start = m + j;
    let goal = i;
    let mut seen = vec![false; total];
    seen[start] = true;
    let mut q = VecDeque::from([start]);
    while let Some(x) = q.pop_front() {
        if x == goal {
            break;
        }
        let nbrs: Vec<usize> = if x < m {
            (0..n).filter(|&c| sol.basic[x][c]).map(|c| m + c).collect()
        } else {
            (0..m).filter(|&r| sol.basic[r][x - m]).collect()
        };
        for y in nbrs {
            if !seen[y] {
                seen[y] = true;
                prev[y] = Some(x);
                q.push_back(y);
            }
        }
    }
    let mut cells = Vec::new();
    let mut x = goal;
    while let Some(p) = prev[x] {
        let (r, c) = if x < m { (x, p - m) } else { (p, x - m) };
        cells.push((r, c));
        x = p;
    }
    cells.reverse(); // desde la columna j hacia la fila i
    cells
}

fn show_cost(tp: &Tp, (i, j): (usize, usize)) -> String {
    match tp.shown[i][j] {
        Some(c) => fmt(c),
        None => "M".into(),
    }
}

/// Muestra un valor interno de coste en el sentido original, con M si procede.
fn fmt_cost(tp: &Tp, x: f64) -> String {
    let v = tp.sign * x;
    let mpart = (v / tp.big).round();
    if mpart != 0.0 && (v - mpart * tp.big).abs() < tp.big * 1e-3 {
        let rest = v - mpart * tp.big;
        let mp = if mpart == 1.0 { "M".into() } else if mpart == -1.0 { "−M".into() } else { format!("{}M", fmt(mpart)) };
        if rest.abs() < 1e-9 {
            mp
        } else {
            format!("{mp} {} {}", if rest > 0.0 { "+" } else { "−" }, fmt(rest.abs()))
        }
    } else {
        fmt(v)
    }
}

struct Iter {
    sol_x: Vec<Vec<f64>>,
    basic: Vec<Vec<bool>>,
    u: Vec<f64>,
    v: Vec<f64>,
    d: Vec<Vec<f64>>,
    entering: Option<(usize, usize)>,
    cycle: Vec<((usize, usize), bool)>,
    theta: f64,
    leaving: Option<(usize, usize)>,
}

fn total_cost(tp: &Tp, x: &[Vec<f64>]) -> f64 {
    let mut s = 0.0;
    for i in 0..tp.m {
        for j in 0..tp.n {
            if let Some(c) = tp.shown[i][j] {
                s += c * x[i][j];
            }
        }
    }
    s
}

/// Método MODI. Devuelve las iteraciones y si terminó (no se agotó el límite).
fn modi(tp: &Tp, mut sol: Sol, max_iter: usize) -> (Vec<Iter>, Sol, bool) {
    let (m, n) = (tp.m, tp.n);
    let mut iters = Vec::new();
    for _ in 0..max_iter {
        let (u, v) = potentials(tp, &sol);
        let mut d = vec![vec![0.0; n]; m];
        let mut ent: Option<(usize, usize)> = None;
        let tol = 1e-12 * tp.big + 1e-9;
        for i in 0..m {
            for j in 0..n {
                if !sol.basic[i][j] {
                    d[i][j] = tp.cost[i][j] - u[i] - v[j];
                    if d[i][j] < -tol && ent.map_or(true, |(a, b)| d[i][j] < d[a][b] - tol) {
                        ent = Some((i, j));
                    }
                }
            }
        }
        let Some((ei, ej)) = ent else {
            iters.push(Iter { sol_x: sol.x.clone(), basic: sol.basic.clone(), u, v, d, entering: None, cycle: vec![], theta: 0.0, leaving: None });
            return (iters, sol, true);
        };
        let path = tree_path(tp, &sol, ei, ej);
        let mut cycle = vec![((ei, ej), true)];
        for (k, &c) in path.iter().enumerate() {
            cycle.push((c, k % 2 == 1)); // la primera celda del camino resta
        }
        let minus: Vec<(usize, usize)> = cycle.iter().filter(|c| !c.1).map(|c| c.0).collect();
        let theta = minus.iter().map(|&(a, b)| sol.x[a][b]).fold(f64::INFINITY, f64::min);
        // sale la primera celda con el mínimo (en el orden del ciclo)
        let leaving = *minus.iter().find(|&&(a, b)| (sol.x[a][b] - theta).abs() <= EPS).unwrap();
        iters.push(Iter {
            sol_x: sol.x.clone(),
            basic: sol.basic.clone(),
            u,
            v,
            d,
            entering: Some((ei, ej)),
            cycle: cycle.clone(),
            theta,
            leaving: Some(leaving),
        });
        for ((a, b), plus) in &cycle {
            if *plus {
                sol.x[*a][*b] += theta;
            } else {
                sol.x[*a][*b] -= theta;
                if sol.x[*a][*b].abs() < EPS {
                    sol.x[*a][*b] = 0.0;
                }
            }
        }
        sol.basic[ei][ej] = true;
        sol.basic[leaving.0][leaving.1] = false;
    }
    (iters, sol, false)
}

fn tableau(tp: &Tp, it: &Iter, title: &str) -> Block {
    let mut cols: Vec<String> = vec!["".into()];
    cols.extend(tp.cols.iter().cloned());
    cols.push("Oferta".into());
    cols.push("u(i)".into());
    let mut t = Table::new(Some(title), &cols);
    for i in 0..tp.m {
        let mut r: Vec<Cell> = vec![tp.rows[i].clone().into()];
        for j in 0..tp.n {
            let c = show_cost(tp, (i, j));
            let mut txt = if it.basic[i][j] {
                format!("{c} │ {}", fmt(it.sol_x[i][j]))
            } else if it.u.is_empty() {
                format!("{c} │")
            } else {
                format!("{c} │ ({})", fmt_cost(tp, it.d[i][j]))
            };
            if let Some((_, plus)) = it.cycle.iter().find(|x| x.0 == (i, j)) {
                txt.push_str(if *plus { " +θ" } else { " −θ" });
            }
            r.push(txt.into());
        }
        r.push(fmt(tp.supply[i]).into());
        r.push(it.u.get(i).map(|u| Cell::from(fmt_cost(tp, *u))).unwrap_or(Cell::Empty));
        t.row(r);
        for j in 0..tp.n {
            if Some((i, j)) == it.entering {
                t.mark(i, j + 1, Tone::Highlight);
            } else if Some((i, j)) == it.leaving {
                t.mark(i, j + 1, Tone::Bad);
            } else if it.basic[i][j] {
                t.mark(i, j + 1, Tone::Accent);
            }
        }
    }
    let mut dr: Vec<Cell> = vec!["Demanda".into()];
    dr.extend(tp.demand.iter().map(|x| Cell::from(fmt(*x))));
    dr.push(Cell::Empty);
    dr.push(Cell::Empty);
    t.row_tone(dr, Tone::Total);
    if !it.v.is_empty() {
        let mut vr: Vec<Cell> = vec!["v(j)".into()];
        vr.extend(it.v.iter().map(|x| Cell::from(fmt_cost(tp, *x))));
        vr.push(Cell::Empty);
        vr.push(Cell::Empty);
        t.row_tone(vr, Tone::Muted);
    }
    t.note = Some("Cada celda muestra «costo │ envío» en las celdas básicas y «costo │ (costo reducido)» en las demás.".into());
    Block::Table(t)
}

/// Rango de optimalidad del coste de cada celda (en el sentido original).
fn ranges(tp: &Tp, sol: &Sol, d: &[Vec<f64>]) -> Vec<Vec<(f64, f64)>> {
    let (m, n) = (tp.m, tp.n);
    let mut out = vec![vec![(f64::NEG_INFINITY, f64::INFINITY); n]; m];
    for i in 0..m {
        for j in 0..n {
            if tp.prohibited[i][j] {
                continue;
            }
            let (lo, hi) = if !sol.basic[i][j] {
                (-d[i][j], f64::INFINITY)
            } else {
                // quitar la celda parte el árbol en dos componentes
                let mut comp = vec![usize::MAX; m + n];
                let mut q = VecDeque::from([i]);
                comp[i] = 0;
                while let Some(x) = q.pop_front() {
                    let nb: Vec<usize> = if x < m {
                        (0..n).filter(|&c| sol.basic[x][c] && (x, c) != (i, j)).map(|c| m + c).collect()
                    } else {
                        (0..m).filter(|&r| sol.basic[r][x - m] && (r, x - m) != (i, j)).collect()
                    };
                    for y in nb {
                        if comp[y] == usize::MAX {
                            comp[y] = 0;
                            q.push_back(y);
                        }
                    }
                }
                let mut lo = f64::NEG_INFINITY;
                let mut hi = f64::INFINITY;
                for k in 0..m {
                    for l in 0..n {
                        if sol.basic[k][l] || tp.prohibited[k][l] {
                            continue;
                        }
                        let row_a = comp[k] == 0;
                        let col_a = comp[m + l] == 0;
                        if row_a && !col_a {
                            hi = hi.min(d[k][l]);
                        } else if !row_a && col_a {
                            lo = lo.max(-d[k][l]);
                        }
                    }
                }
                (lo, hi)
            };
            let c = tp.cost[i][j];
            let (a, b) = (c + lo, c + hi);
            out[i][j] = if tp.sign > 0.0 { (a, b) } else { (-b, -a) };
        }
    }
    out
}

fn verify_lp(tp: &Tp) -> (LpStatus, f64) {
    let (m, n) = (tp.m, tp.n);
    let mut lp = Lp::new(0, false);
    let mut idx = vec![vec![None; n]; m];
    for i in 0..m {
        for j in 0..n {
            if !tp.prohibited[i][j] {
                idx[i][j] = Some(lp.add_var(tp.cost[i][j], 0.0, f64::INFINITY, false));
            }
        }
    }
    for i in 0..m {
        let c: Vec<(usize, f64)> = (0..n).filter_map(|j| idx[i][j].map(|k| (k, 1.0))).collect();
        lp.row(c, Sense::Eq, tp.supply[i]);
    }
    for j in 0..n {
        let c: Vec<(usize, f64)> = (0..m).filter_map(|i| idx[i][j].map(|k| (k, 1.0))).collect();
        lp.row(c, Sense::Eq, tp.demand[j]);
    }
    let s = lp::solve(&lp);
    (s.status, tp.sign * s.objective)
}

pub fn solve(md: &NetModel, o: &NetOptions) -> Result<Report, ModError> {
    let tp = build(md);
    let (lp_status, lp_obj) = verify_lp(&tp);
    let what = if md.objective.is_max() { "Beneficio" } else { "Costo" };
    if lp_status == LpStatus::Infeasible {
        return Ok(Report::new("net", &md.name, Status::Infeasible, "No hay forma de cubrir la demanda con las conexiones disponibles.")
            .section("solucion", "Solución", vec![text_tone(
                "El problema no es factible: alguna demanda solo se podría cubrir usando celdas sin conexión. Revisa las celdas vacías de la matriz de costes.",
                Tone::Bad,
            )]));
    }
    let (sol0, log) = initial(&tp, o.initial);
    let init_cost = total_cost(&tp, &sol0.x);
    // la solución inicial puede usar celdas prohibidas (costo M): entonces su costo es «M»
    let init_m = (0..tp.m).any(|i| (0..tp.n).any(|j| tp.prohibited[i][j] && sol0.x[i][j] > 1e-9));
    let init_txt = if init_m { format!("{} + M (usa celdas prohibidas)", fmt(init_cost)) } else { fmt(init_cost) };
    let init_snapshot = Iter {
        sol_x: sol0.x.clone(),
        basic: sol0.basic.clone(),
        u: vec![],
        v: vec![],
        d: vec![vec![0.0; tp.n]; tp.m],
        entering: None,
        cycle: vec![],
        theta: 0.0,
        leaving: None,
    };
    let (iters, sol, finished) = modi(&tp, sol0, 10_000);
    let z = total_cost(&tp, &sol.x);
    let uses_prohibited = (0..tp.m).any(|i| (0..tp.n).any(|j| tp.prohibited[i][j] && sol.x[i][j] > EPS));
    let agrees = finished && !uses_prohibited && (z - lp_obj).abs() <= 1e-6 * (1.0 + lp_obj.abs());
    if !agrees {
        return Err(ModError::Invalid(format!(
            "el método de transporte no coincidió con HiGHS ({} frente a {}); por favor, informa de este caso",
            fmt(z),
            fmt(lp_obj)
        )));
    }
    let last = iters.last().unwrap();
    let rng = ranges(&tp, &sol, &last.d);

    // --- solución ---
    let mut t = Table::new(None, &["Desde", "Hasta", "Envío", "Costo unitario", "Total", "Costo reducido", "Mínimo", "Máximo"]).left(1);
    let mut flows = Vec::new();
    for i in 0..tp.m {
        for j in 0..tp.n {
            let show = sol.x[i][j] > EPS;
            if (tp.dummy_row && i == tp.m - 1) || (tp.dummy_col && j == tp.n - 1) {
                continue;
            }
            if tp.prohibited[i][j] {
                continue;
            }
            let c = tp.shown[i][j].unwrap();
            let (lo, hi) = rng[i][j];
            let red = if sol.basic[i][j] { 0.0 } else { tp.sign * last.d[i][j] };
            if show {
                flows.push((i, j, sol.x[i][j]));
                t.row_tone(cells![&tp.rows[i], &tp.cols[j], sol.x[i][j], c, c * sol.x[i][j], red, lo, hi], Tone::Accent);
            } else {
                t.row(cells![&tp.rows[i], &tp.cols[j], 0.0, c, 0.0, red, lo, hi]);
            }
        }
    }
    t.row_tone(cells!["Total", "", "", "", z, "", "", ""], Tone::Total);
    let t = t.note("Mínimo y máximo: rango del costo unitario en el que la solución sigue siendo óptima.");

    let mut unmet = Vec::new();
    if tp.dummy_col {
        for i in 0..tp.m {
            let q = sol.x[i][tp.n - 1];
            if q > EPS {
                unmet.push(format!("{} se queda con {} unidades sin enviar", tp.rows[i], fmt(q)));
            }
        }
    }
    if tp.dummy_row {
        for j in 0..tp.n {
            let q = sol.x[tp.m - 1][j];
            if q > EPS {
                unmet.push(format!("a {} le faltan {} unidades", tp.cols[j], fmt(q)));
            }
        }
    }
    let alt = (0..tp.m).any(|i| (0..tp.n).any(|j| !sol.basic[i][j] && !tp.prohibited[i][j] && last.d[i][j].abs() < 1e-9));

    let mut sol_blocks = vec![metrics(vec![
        metric_main(format!("{what} total"), z),
        metric("Iteraciones MODI", iters.len() - 1),
        metric(format!("{what} de la solución inicial"), if init_m { Cell::from(init_txt.clone()) } else { Cell::Num(init_cost) }),
    ])];
    if !unmet.is_empty() {
        sol_blocks.push(text(format!(
            "La oferta y la demanda no están equilibradas, así que se añadió un {} ficticio con costo 0: {}.",
            if tp.dummy_col { "destino" } else { "origen" },
            unmet.join("; ")
        )));
    }
    if alt {
        sol_blocks.push(text("Hay soluciones óptimas alternativas: alguna celda sin envío tiene costo reducido 0."));
    }
    sol_blocks.push(t.block());
    let mut duals = Table::new(Some("Valores duales"), &["Fila o columna", "u(i) / v(j)"]);
    for i in 0..tp.m {
        duals.row(cells![format!("u: {}", tp.rows[i]), tp.sign * last.u[i]]);
    }
    for j in 0..tp.n {
        duals.row(cells![format!("v: {}", tp.cols[j]), tp.sign * last.v[j]]);
    }

    let mut report = Report::new(
        "net",
        &md.name,
        Status::Optimal,
        format!("{what} total óptimo: {}.", fmt(z)),
    )
    .section("solucion", "Solución", sol_blocks)
    .section("red", "Red", vec![bipartite_network(md, "Envíos de la solución óptima", &flows, false)])
    .section("analisis", "Análisis", vec![duals.block()]);

    if o.steps && tp.m * tp.n <= 400 {
        let method = match o.initial {
            InitialMethod::NorthwestCorner => "esquina noroeste",
            InitialMethod::MinimumCost => "costo mínimo",
            InitialMethod::Vogel => "aproximación de Vogel",
            InitialMethod::Russell => "aproximación de Russell",
        };
        let mut a = Table::new(Some("Asignaciones en orden"), &["Paso", "Celda", "Cantidad", "Motivo"]).left(1).left(3);
        for (k, al) in log.iter().enumerate() {
            a.row(cells![k + 1, format!("{} → {}", tp.rows[al.cell.0], tp.cols[al.cell.1]), al.qty, &al.why]);
        }
        let mut st = vec![step(
            format!("Solución inicial: {method}"),
            format!(
                "Se asigna en cada paso todo lo posible a la celda elegida y se tacha la fila o la columna que se agota. {} total inicial: {}.{}",
                what,
                init_txt,
                if init_m { " Las celdas prohibidas tienen un costo M muy grande, así que el método MODI las vaciará." } else { "" }
            ),
            vec![a.block(), tableau(&tp, &init_snapshot, "Solución inicial")],
        )];
        for (k, it) in iters.iter().enumerate() {
            let title = if it.entering.is_none() { "Solución óptima".to_string() } else { format!("Iteración {}", k + 1) };
            let desc = match (it.entering, it.leaving) {
                (Some(e), Some(l)) => format!(
                    "Se calculan u(i) y v(j) con u({}) = 0 y u(i) + v(j) = costo en las celdas básicas. La celda {} → {} tiene el costo reducido más negativo ({}), así que entra. En el ciclo, θ = {}: sale {} → {}. {} total: {} → {}.",
                    tp.rows[0],
                    tp.rows[e.0],
                    tp.cols[e.1],
                    fmt_cost(&tp, it.d[e.0][e.1]),
                    fmt(it.theta),
                    tp.rows[l.0],
                    tp.cols[l.1],
                    what,
                    fmt(total_cost(&tp, &it.sol_x)),
                    fmt(total_cost(&tp, &it.sol_x) + tp.sign * it.theta * it.d[e.0][e.1]),
                ),
                _ => format!(
                    "Todos los costos reducidos son {}: la solución es óptima. {} total: {}.",
                    if md.objective.is_max() { "≤ 0 en beneficio" } else { "≥ 0" },
                    what,
                    fmt(z)
                ),
            };
            st.push(step(title.clone(), desc, vec![tableau(&tp, it, &title)]));
        }
        report = report.section("pasos", "Paso a paso", vec![steps(st)]);
    }

    let shipments: Vec<_> = flows
        .iter()
        .map(|&(i, j, q)| json!({"from": tp.rows[i], "to": tp.cols[j], "quantity": clean(q), "unit_cost": tp.shown[i][j]}))
        .collect();
    Ok(report.data(json!({
        "objective_value": clean(z),
        "shipments": shipments,
        "u": tp.rows.iter().zip(&last.u).map(|(n, u)| json!({"name": n, "value": clean(tp.sign * u)})).collect::<Vec<_>>(),
        "v": tp.cols.iter().zip(&last.v).map(|(n, v)| json!({"name": n, "value": clean(tp.sign * v)})).collect::<Vec<_>>(),
        "dummy": if tp.dummy_col { Some("destination") } else if tp.dummy_row { Some("source") } else { None },
        "alternate_optima": alt,
        "iterations": iters.len() - 1,
    })))
}
