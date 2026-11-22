//! Problema de asignación: método húngaro (exacto y didáctico), comprobado con HiGHS.

use super::layout::bipartite_network;
use super::{NetModel, NetOptions};
use iaqsb_core::lp::{self, Lp, Sense};
use iaqsb_core::report::*;
use iaqsb_core::{cells, ModError};
use serde_json::json;
use std::collections::VecDeque;


/// Emparejamiento máximo sobre los ceros (caminos de aumento). Devuelve col → fila.
fn max_matching(z: &[Vec<bool>]) -> Vec<Option<usize>> {
    let n = z.len();
    let mut match_col: Vec<Option<usize>> = vec![None; n];
    fn aug(r: usize, z: &[Vec<bool>], seen: &mut [bool], mc: &mut [Option<usize>]) -> bool {
        for c in 0..z.len() {
            if z[r][c] && !seen[c] {
                seen[c] = true;
                if mc[c].map_or(true, |r2| aug(r2, z, seen, mc)) {
                    mc[c] = Some(r);
                    return true;
                }
            }
        }
        false
    }
    for r in 0..n {
        let mut seen = vec![false; n];
        aug(r, z, &mut seen, &mut match_col);
    }
    match_col
}

/// Cubrimiento mínimo de ceros (teorema de König): (filas cubiertas, columnas cubiertas).
fn min_cover(z: &[Vec<bool>], mc: &[Option<usize>]) -> (Vec<bool>, Vec<bool>) {
    let n = z.len();
    let mut row_matched = vec![false; n];
    for r in mc.iter().flatten() {
        row_matched[*r] = true;
    }
    let mut vr = vec![false; n];
    let mut vc = vec![false; n];
    let mut q: VecDeque<usize> = (0..n).filter(|&r| !row_matched[r]).collect();
    for &r in &q {
        vr[r] = true;
    }
    while let Some(r) = q.pop_front() {
        for c in 0..n {
            if z[r][c] && !vc[c] {
                vc[c] = true;
                if let Some(r2) = mc[c] {
                    if !vr[r2] {
                        vr[r2] = true;
                        q.push_back(r2);
                    }
                }
            }
        }
    }
    ((0..n).map(|r| !vr[r]).collect(), vc)
}

struct HStep {
    title: String,
    desc: String,
    mat: Vec<Vec<f64>>,
    rows_cov: Vec<bool>,
    cols_cov: Vec<bool>,
    assign: Vec<Option<usize>>,
}

pub fn solve(md: &NetModel, o: &NetOptions) -> Result<Report, ModError> {
    let ns = md.sources.len();
    let nd = md.destinations.len();
    let n = ns.max(nd);
    let maxc = md.costs.iter().flatten().flatten().fold(0.0f64, |a, &b| a.max(b.abs()));
    let big = 1e6 * (1.0 + maxc);
    let is_max = md.objective.is_max();
    let maxv = md.costs.iter().flatten().flatten().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    // matriz cuadrada (con filas o columnas ficticias de costo 0) en minimización
    let mut c = vec![vec![0.0; n]; n];
    let mut prohibited = vec![vec![false; n]; n];
    for i in 0..n {
        for j in 0..n {
            if i < ns && j < nd {
                match md.costs[i][j] {
                    Some(v) => c[i][j] = if is_max { maxv - v } else { v },
                    None => {
                        c[i][j] = big;
                        prohibited[i][j] = true;
                    }
                }
            }
        }
    }
    let rname = |i: usize| if i < ns { md.sources[i].name.clone() } else { format!("Ficticio{}", i - ns + 1) };
    let cname = |j: usize| if j < nd { md.destinations[j].name.clone() } else { format!("Ficticio{}", j - nd + 1) };

    // --- método húngaro ---
    let mut a = c.clone();
    let mut hs: Vec<HStep> = Vec::new();
    let snap = |a: &Vec<Vec<f64>>| a.clone();
    hs.push(HStep {
        title: "Matriz de costos".into(),
        desc: if is_max {
            format!("Es de maximizar: se convierte en minimización restando cada valor del mayor ({}).", fmt(maxv))
        } else if ns != nd {
            "La matriz no es cuadrada: se añaden filas o columnas ficticias con costo 0.".into()
        } else {
            "Matriz de partida.".into()
        },
        mat: snap(&a),
        rows_cov: vec![false; n],
        cols_cov: vec![false; n],
        assign: vec![None; n],
    });
    for row in a.iter_mut() {
        let mn = row.iter().cloned().fold(f64::INFINITY, f64::min);
        for x in row.iter_mut() {
            *x -= mn;
        }
    }
    hs.push(HStep {
        title: "Reducción por filas".into(),
        desc: "A cada fila se le resta su menor valor.".into(),
        mat: snap(&a),
        rows_cov: vec![false; n],
        cols_cov: vec![false; n],
        assign: vec![None; n],
    });
    for j in 0..n {
        let mn = (0..n).map(|i| a[i][j]).fold(f64::INFINITY, f64::min);
        for row in a.iter_mut() {
            row[j] -= mn;
        }
    }
    let mut desc = "A cada columna se le resta su menor valor.".to_string();
    let mut iter = 0;
    let assignment: Vec<usize> = loop {
        iter += 1;
        let tol = 1e-12 * big + 1e-9;
        let z: Vec<Vec<bool>> = a.iter().map(|r| r.iter().map(|&x| x.abs() <= tol).collect()).collect();
        let mc = max_matching(&z);
        let matched = mc.iter().filter(|x| x.is_some()).count();
        let (rc, cc) = min_cover(&z, &mc);
        let lines = rc.iter().filter(|&&b| b).count() + cc.iter().filter(|&&b| b).count();
        let mut asg = vec![None; n];
        for (cj, r) in mc.iter().enumerate() {
            if let Some(r) = r {
                asg[*r] = Some(cj);
            }
        }
        let done = matched == n;
        hs.push(HStep {
            title: if iter == 1 { "Reducción por columnas".into() } else { format!("Ajuste {}", iter - 1) },
            desc: format!(
                "{desc} Los ceros se cubren con {lines} líneas{}.",
                if done { format!(", igual al tamaño de la matriz ({n}): ya hay una asignación óptima sobre los ceros") } else { format!(", menos que {n}: hay que ajustar") }
            ),
            mat: snap(&a),
            rows_cov: rc.clone(),
            cols_cov: cc.clone(),
            assign: if done { asg.clone() } else { vec![None; n] },
        });
        if done || iter > n * n + 10 {
            break asg.into_iter().map(|x| x.unwrap_or(0)).collect();
        }
        let mut k = f64::INFINITY;
        for i in 0..n {
            for j in 0..n {
                if !rc[i] && !cc[j] {
                    k = k.min(a[i][j]);
                }
            }
        }
        for i in 0..n {
            for j in 0..n {
                if !rc[i] && !cc[j] {
                    a[i][j] -= k;
                } else if rc[i] && cc[j] {
                    a[i][j] += k;
                }
            }
        }
        desc = format!("El menor valor no cubierto es {}: se resta a los no cubiertos y se suma a las intersecciones de dos líneas.", fmt(k));
    };

    let z: f64 = (0..ns).filter(|&i| assignment[i] < nd).filter_map(|i| md.costs[i][assignment[i]]).sum();
    let uses_prohibited = (0..n).any(|i| prohibited[i][assignment[i]]);
    if uses_prohibited {
        return Ok(Report::new("net", &md.name, Status::Infeasible, "No existe una asignación completa con las conexiones disponibles.")
            .section("solucion", "Solución", vec![text_tone("Algún objeto solo podría asignarse a tareas sin conexión. Revisa las celdas vacías.", Tone::Bad)]));
    }
    // comprobación con HiGHS
    let mut lpm = Lp::new(0, is_max);
    let mut idx = vec![vec![None; nd]; ns];
    for i in 0..ns {
        for j in 0..nd {
            if let Some(v) = md.costs[i][j] {
                idx[i][j] = Some(lpm.add_var(v, 0.0, 1.0, false));
            }
        }
    }
    let (rs, cs) = if ns <= nd { (Sense::Eq, Sense::Le) } else { (Sense::Le, Sense::Eq) };
    for i in 0..ns {
        lpm.row((0..nd).filter_map(|j| idx[i][j].map(|k| (k, 1.0))).collect(), rs, 1.0);
    }
    for j in 0..nd {
        lpm.row((0..ns).filter_map(|i| idx[i][j].map(|k| (k, 1.0))).collect(), cs, 1.0);
    }
    let check = lp::solve(&lpm);
    if check.is_optimal() && (check.objective - z).abs() > 1e-6 * (1.0 + z.abs()) {
        return Err(ModError::Invalid(format!("el método húngaro no coincidió con HiGHS ({} frente a {})", fmt(z), fmt(check.objective))));
    }

    let what = if is_max { "Beneficio" } else { "Costo" };
    let mut t = Table::new(None, &["Objeto", "Tarea", what]).left(1);
    let mut flows = vec![];
    let mut pairs = vec![];
    for i in 0..ns {
        let j = assignment[i];
        if j < nd {
            let v = md.costs[i][j].unwrap();
            t.row_tone(cells![&md.sources[i].name, &md.destinations[j].name, v], Tone::Accent);
            flows.push((i, j, 1.0));
            pairs.push(json!({"object": md.sources[i].name, "task": md.destinations[j].name, "value": v}));
        } else {
            t.row_tone(cells![&md.sources[i].name, "sin tarea (ficticia)", Cell::Empty], Tone::Muted);
        }
    }
    for j in 0..nd {
        if !(0..ns).any(|i| assignment[i] == j) {
            t.row_tone(cells!["sin objeto (ficticio)", &md.destinations[j].name, Cell::Empty], Tone::Muted);
        }
    }
    t.row_tone(cells!["Total", "", z], Tone::Total);

    let mut report = Report::new("net", &md.name, Status::Optimal, format!("{what} total óptimo de la asignación: {}.", fmt(z)))
        .section("solucion", "Solución", vec![metrics(vec![metric_main(format!("{what} total"), z), metric("Asignaciones", flows.len())]), t.block()])
        .section("red", "Red", vec![bipartite_network(md, "Asignación óptima", &flows, true)]);

    if o.steps && n <= 30 {
        let st = hs
            .iter()
            .map(|h| {
                let mut cols = vec!["".to_string()];
                cols.extend((0..n).map(&cname));
                let mut t = Table::new(Some(&h.title), &cols);
                for i in 0..n {
                    let mut r: Vec<Cell> = vec![format!("{}{}", rname(i), if h.rows_cov[i] { "  ━" } else { "" }).into()];
                    for j in 0..n {
                        r.push(if prohibited[i][j] && h.mat[i][j] > big / 2.0 { "M".into() } else { Cell::Num(h.mat[i][j]) });
                    }
                    t.row(r);
                    for j in 0..n {
                        if h.assign[i] == Some(j) {
                            t.mark(i, j + 1, Tone::Highlight);
                        } else if h.rows_cov[i] || h.cols_cov[j] {
                            t.mark(i, j + 1, if h.rows_cov[i] && h.cols_cov[j] { Tone::Bad } else { Tone::Accent });
                        }
                    }
                }
                t.col_marks = (0..n).filter(|&j| h.cols_cov[j]).map(|j| (j + 1, Tone::Accent)).collect();
                if h.rows_cov.iter().any(|&b| b) || h.cols_cov.iter().any(|&b| b) {
                    t.note = Some("Celdas azules: cubiertas por una línea; rojas: por dos. Las filas marcadas con ━ están cubiertas.".into());
                }
                step(h.title.clone(), h.desc.clone(), vec![t.block()])
            })
            .collect();
        report = report.section("pasos", "Paso a paso", vec![steps(st)]);
    }
    Ok(report.data(json!({"objective_value": clean(z), "assignments": pairs})))
}
