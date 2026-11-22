//! Símplex de metas (método de las filas Cj − Zj por prioridad), el que se
//! enseña en clase para la programación por metas preferente. Cada prioridad
//! tiene su fila de costos reducidos; una variable entra solo si mejora una
//! prioridad sin empeorar ninguna superior. La fase uno (variables artificiales)
//! se trata como la prioridad más alta.
//!
//! Solo para modelos continuos con variables ≥ 0; las cotas superiores se
//! añaden como restricciones. El resultado se compara con HiGHS.

use iaqsb_core::linear::Dir;
use iaqsb_core::report::*;

pub struct Problem {
    /// Nombres de las variables originales.
    pub names: Vec<String>,
    /// Restricciones: coeficientes, sentido y lado derecho.
    pub rows: Vec<(String, Vec<f64>, Dir, f64)>,
    /// Metas en orden de prioridad, ya en forma de minimizar.
    pub goals: Vec<(String, Vec<f64>)>,
}

pub struct Outcome {
    pub steps: Vec<Step>,
    /// Valores de las variables originales (None si no factible / no acotado).
    pub x: Option<Vec<f64>>,
    pub status: &'static str,
}

const EPS: f64 = 1e-9;

pub fn run(p: &Problem, goal_labels: &[String]) -> Outcome {
    let n = p.names.len();
    let m = p.rows.len();
    // columnas: originales, holguras/excesos, artificiales
    let mut cols: Vec<String> = p.names.clone();
    let mut a = vec![vec![0.0; 0]; m];
    let mut b = vec![0.0; m];
    let mut basis = vec![0usize; m];
    let mut art = vec![];
    let mut dirs = vec![];
    for (i, (_, c, d, r)) in p.rows.iter().enumerate() {
        let (mut c, mut d, mut r) = (c.clone(), *d, *r);
        if r < 0.0 {
            c.iter_mut().for_each(|x| *x = -*x);
            r = -r;
            d = match d {
                Dir::Le => Dir::Ge,
                Dir::Ge => Dir::Le,
                Dir::Eq => Dir::Eq,
            };
        }
        a[i] = c;
        b[i] = r;
        dirs.push(d);
    }
    let push_col = |a: &mut Vec<Vec<f64>>, i: usize, v: f64| {
        for (k, row) in a.iter_mut().enumerate() {
            row.push(if k == i { v } else { 0.0 });
        }
    };
    for i in 0..m {
        match dirs[i] {
            Dir::Le => {
                push_col(&mut a, i, 1.0);
                cols.push(format!("S{}", i + 1));
                basis[i] = cols.len() - 1;
            }
            Dir::Ge => {
                push_col(&mut a, i, -1.0);
                cols.push(format!("E{}", i + 1));
            }
            Dir::Eq => {}
        }
    }
    // columnas unitarias de las variables originales (p. ej. las desviaciones d⁻):
    // sirven de base inicial y ahorran variables artificiales, como en el libro
    let mut used = vec![false; n];
    let mut has_basis = vec![false; m];
    for i in 0..m {
        if dirs[i] == Dir::Le {
            has_basis[i] = true;
            continue;
        }
        if let Some(j) = (0..n).find(|&j| !used[j] && (a[i][j] - 1.0).abs() < 1e-12 && (0..m).all(|r| r == i || a[r][j] == 0.0)) {
            used[j] = true;
            basis[i] = j;
            has_basis[i] = true;
        }
    }
    for i in 0..m {
        if !has_basis[i] {
            push_col(&mut a, i, 1.0);
            cols.push(format!("A{}", i + 1));
            basis[i] = cols.len() - 1;
            art.push(cols.len() - 1);
        }
    }
    let nc = cols.len();
    // filas de costo: fase uno (si hay artificiales) y las metas
    let mut costs: Vec<(String, Vec<f64>)> = vec![];
    if !art.is_empty() {
        let mut c = vec![0.0; nc];
        for &j in &art {
            c[j] = 1.0;
        }
        costs.push(("Fase 1 (artificiales)".into(), c));
    }
    for (k, (_, g)) in p.goals.iter().enumerate() {
        let mut c = vec![0.0; nc];
        c[..n].copy_from_slice(&g[..n]);
        costs.push((format!("P{}: {}", k + 1, goal_labels[k]), c));
    }
    let nk = costs.len();
    let is_art = |j: usize| art.contains(&j);

    let mut steps = vec![];
    let reduced = |a: &Vec<Vec<f64>>, b: &Vec<f64>, basis: &Vec<usize>| -> (Vec<Vec<f64>>, Vec<f64>) {
        let mut r = vec![vec![0.0; nc]; nk];
        let mut z = vec![0.0; nk];
        for k in 0..nk {
            let c = &costs[k].1;
            for j in 0..nc {
                let zj: f64 = (0..m).map(|i| c[basis[i]] * a[i][j]).sum();
                r[k][j] = c[j] - zj;
            }
            z[k] = (0..m).map(|i| c[basis[i]] * b[i]).sum();
        }
        (r, z)
    };
    let tableau = |a: &Vec<Vec<f64>>, b: &Vec<f64>, basis: &Vec<usize>, r: &Vec<Vec<f64>>, z: &Vec<f64>, piv: Option<(usize, usize)>, ratios: Option<&Vec<Option<f64>>>| -> Block {
        let mut head = vec!["Base".to_string()];
        head.extend(cols.iter().cloned());
        head.push("LD".into());
        if ratios.is_some() {
            head.push("Razón".into());
        }
        let mut t = Table::new(None, &head).left(0);
        for i in 0..m {
            let mut row = vec![Cell::from(&cols[basis[i]])];
            row.extend(a[i].iter().map(|x| Cell::Num(clean(*x))));
            row.push(Cell::Num(clean(b[i])));
            if let Some(rt) = ratios {
                row.push(rt[i].map(|v| Cell::Num(clean(v))).unwrap_or(Cell::from("—")));
            }
            t.row(row);
        }
        for k in 0..nk {
            let mut row = vec![Cell::from(format!("Cj−Zj {}", costs[k].0))];
            row.extend(r[k].iter().map(|x| Cell::Num(clean(*x))));
            row.push(Cell::Num(clean(z[k])));
            if ratios.is_some() {
                row.push(Cell::Empty);
            }
            t.row_tone(row, Tone::Total);
        }
        if let Some((pi, pj)) = piv {
            t.mark(pi, pj + 1, Tone::Highlight);
            t.col_marks.push((pj + 1, Tone::Accent));
        }
        t.block()
    };

    let mut iter = 0;
    let max_iter = 50 * (m + nc) + 100;
    loop {
        let (r, z) = reduced(&a, &b, &basis);
        // prioridad activa y columna que entra
        let mut choice: Option<(usize, usize)> = None;
        for k in 0..nk {
            let allowed: Vec<usize> = (0..nc)
                .filter(|&j| !is_art(j) && !basis.contains(&j))
                .filter(|&j| (0..k).all(|l| r[l][j].abs() <= EPS * 1e3) && r[k][j] < -EPS * 1e3)
                .collect();
            if !allowed.is_empty() {
                let j = if iter > 2 * (m + nc) {
                    allowed[0] // regla de Bland para evitar ciclos
                } else {
                    *allowed.iter().min_by(|&&x, &&y| r[k][x].partial_cmp(&r[k][y]).unwrap()).unwrap()
                };
                choice = Some((k, j));
                break;
            }
        }
        let Some((k, j)) = choice else {
            // óptimo
            let feasible = art.is_empty() || z[0] <= 1e-7;
            let desc = if feasible {
                "Ninguna variable puede mejorar una prioridad sin empeorar otra más importante: la solución es óptima.".to_string()
            } else {
                "Al terminar la fase uno quedan variables artificiales positivas: las restricciones no tienen solución.".to_string()
            };
            steps.push(step(if feasible { "Tabla final" } else { "Sin solución factible" }, desc, vec![tableau(&a, &b, &basis, &r, &z, None, None)]));
            if !feasible {
                return Outcome { steps, x: None, status: "infeasible" };
            }
            let mut x = vec![0.0; n];
            for i in 0..m {
                if basis[i] < n {
                    x[basis[i]] = b[i];
                }
            }
            return Outcome { steps, x: Some(x), status: "optimal" };
        };
        // razón mínima
        let ratios: Vec<Option<f64>> = (0..m).map(|i| if a[i][j] > EPS { Some(b[i] / a[i][j]) } else { None }).collect();
        let leave = (0..m)
            .filter(|&i| ratios[i].is_some())
            .min_by(|&x, &y| ratios[x].unwrap().partial_cmp(&ratios[y].unwrap()).unwrap().then(basis[x].cmp(&basis[y])));
        let Some(i) = leave else {
            steps.push(step(
                "Problema no acotado",
                format!("{} mejora {} pero ninguna restricción la limita.", cols[j], costs[k].0),
                vec![tableau(&a, &b, &basis, &r, &z, None, None)],
            ));
            return Outcome { steps, x: None, status: "unbounded" };
        };
        iter += 1;
        if steps.len() < 60 {
            steps.push(step(
                format!("Iteración {iter}"),
                format!(
                    "Se trabaja la prioridad «{}»: entra {} (Cj−Zj = {}, sin empeorar prioridades superiores) y sale {} (razón mínima {}).",
                    costs[k].0,
                    cols[j],
                    fmt(r[k][j]),
                    cols[basis[i]],
                    fmt(ratios[i].unwrap())
                ),
                vec![tableau(&a, &b, &basis, &r, &z, Some((i, j)), Some(&ratios))],
            ));
        }
        // pivote
        let pv = a[i][j];
        for x in a[i].iter_mut() {
            *x /= pv;
        }
        b[i] /= pv;
        for r2 in 0..m {
            if r2 != i {
                let f = a[r2][j];
                if f.abs() > 0.0 {
                    for c in 0..nc {
                        a[r2][c] -= f * a[i][c];
                    }
                    b[r2] -= f * b[i];
                }
            }
        }
        basis[i] = j;
        if iter > max_iter {
            return Outcome { steps, x: None, status: "iterations" };
        }
    }
}
