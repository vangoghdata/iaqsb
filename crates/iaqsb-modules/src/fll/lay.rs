//! Distribución funcional por intercambio de departamentos (al estilo de CRAFT).

use super::{dist, dist_label, Exchange, FllModel, FllOptions};
use iaqsb_core::*;
use serde_json::json;

type Cellp = (usize, usize);

/// Lee "(3,5), (1,1)-(2,4)": celdas sueltas y rectángulos (fila, columna, desde 1).
pub fn parse_cells(s: &str, rows: usize, cols: usize, who: &str) -> Result<Vec<Cellp>, ModError> {
    let mut items: Vec<Option<Cellp>> = vec![]; // None = guion
    let ch: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < ch.len() {
        match ch[i] {
            '(' => {
                let close = ch[i..].iter().position(|c| *c == ')').map(|p| p + i).ok_or_else(|| ModError::Invalid(format!("{who}: falta «)» en las celdas")))?;
                let inner: String = ch[i + 1..close].iter().collect();
                let (a, b) = inner.split_once(',').ok_or_else(|| ModError::Invalid(format!("{who}: «({inner})» debe ser (fila,columna)")))?;
                let r: usize = a.trim().parse().map_err(|_| ModError::Invalid(format!("{who}: «{a}» no es una fila")))?;
                let c: usize = b.trim().parse().map_err(|_| ModError::Invalid(format!("{who}: «{b}» no es una columna")))?;
                if r < 1 || r > rows || c < 1 || c > cols {
                    return invalid(format!("{who}: la celda ({r},{c}) está fuera del área de {rows} × {cols}"));
                }
                items.push(Some((r - 1, c - 1)));
                i = close + 1;
            }
            '-' => {
                items.push(None);
                i += 1;
            }
            _ => i += 1,
        }
    }
    let mut out = vec![];
    let mut k = 0;
    while k < items.len() {
        match items[k] {
            Some(a) => {
                if k + 2 < items.len() && items[k + 1].is_none() {
                    if let Some(b) = items[k + 2] {
                        for r in a.0.min(b.0)..=a.0.max(b.0) {
                            for c in a.1.min(b.1)..=a.1.max(b.1) {
                                out.push((r, c));
                            }
                        }
                        k += 3;
                        continue;
                    }
                }
                out.push(a);
                k += 1;
            }
            None => k += 1,
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

pub fn cells_text(v: &[Cellp]) -> String {
    v.iter().map(|(r, c)| format!("({},{})", r + 1, c + 1)).collect::<Vec<_>>().join(", ")
}

fn centroid(v: &[Cellp]) -> Vec<f64> {
    let n = v.len().max(1) as f64;
    vec![v.iter().map(|x| x.1 as f64 + 0.5).sum::<f64>() / n, v.iter().map(|x| x.0 as f64 + 0.5).sum::<f64>() / n]
}

struct Lay<'a> {
    c: &'a [Vec<f64>],
    dm: super::Distance,
}

impl Lay<'_> {
    fn cost(&self, cells: &[Vec<Cellp>]) -> f64 {
        let cen: Vec<Vec<f64>> = cells.iter().map(|v| centroid(v)).collect();
        let n = cells.len();
        let mut s = 0.0;
        for i in 0..n {
            for j in 0..n {
                if i != j && self.c[i][j] != 0.0 {
                    s += self.c[i][j] * dist(self.dm, &cen[i], &cen[j]);
                }
            }
        }
        s
    }
}

fn adjacent(a: &[Cellp], b: &[Cellp]) -> bool {
    a.iter().any(|x| b.iter().any(|y| (x.0 == y.0 && x.1.abs_diff(y.1) == 1) || (x.1 == y.1 && x.0.abs_diff(y.0) == 1)))
}

/// Intercambia dos departamentos: iguales en área se cambian las celdas; si son
/// adyacentes y distintos, se reparte su unión según la dirección entre centros.
fn swap2(cells: &[Vec<Cellp>], i: usize, j: usize) -> Option<Vec<Vec<Cellp>>> {
    let (a, b) = (&cells[i], &cells[j]);
    let mut out = cells.to_vec();
    if a.len() == b.len() {
        out[i] = b.clone();
        out[j] = a.clone();
        return Some(out);
    }
    if !adjacent(a, b) {
        return None;
    }
    let ca = centroid(a);
    let cb = centroid(b);
    let dir = [cb[0] - ca[0], cb[1] - ca[1]];
    let mut u: Vec<Cellp> = a.iter().chain(b.iter()).copied().collect();
    u.sort_by(|p, q| {
        let pp = (p.1 as f64 + 0.5) * dir[0] + (p.0 as f64 + 0.5) * dir[1];
        let qq = (q.1 as f64 + 0.5) * dir[0] + (q.0 as f64 + 0.5) * dir[1];
        pp.partial_cmp(&qq).unwrap().then(p.cmp(q))
    });
    let mut nb: Vec<Cellp> = u[..b.len()].to_vec();
    let mut na: Vec<Cellp> = u[b.len()..].to_vec();
    nb.sort();
    na.sort();
    out[j] = nb;
    out[i] = na;
    Some(out)
}

pub fn solve(m: &FllModel, o: &FllOptions) -> Result<Report, ModError> {
    let n = m.departments.len();
    let (rows, cols) = (m.rows as usize, m.cols as usize);
    if n == 0 || rows == 0 || cols == 0 {
        return invalid("hacen falta departamentos y un área de al menos una celda");
    }
    check_names("departamentos", &m.departments.iter().map(|d| &d.name).collect::<Vec<_>>())?;
    let names: Vec<&str> = m.departments.iter().map(|d| d.name.as_str()).collect();
    let mut cells: Vec<Vec<Cellp>> = vec![];
    let mut owner = vec![vec![usize::MAX; cols]; rows];
    // Si dos departamentos reclaman la misma celda, gana el que aparece después (como en WinQSB:
    // en el ejemplo del centro comercial el pasillo fijo «H» se superpone a parte del departamento 6).
    let mut overlaps: Vec<String> = vec![];
    for (k, d) in m.departments.iter().enumerate() {
        let v = parse_cells(&d.cells, rows, cols, &d.name)?;
        if v.is_empty() {
            return invalid(format!("{}: indica sus celdas en la distribución inicial", d.name));
        }
        for &(r, c) in &v {
            let prev = owner[r][c];
            if prev != usize::MAX && prev != k {
                cells[prev].retain(|&x| x != (r, c));
                overlaps.push(format!("({},{}) de {} pasa a {}", r + 1, c + 1, names[prev], d.name));
            }
            owner[r][c] = k;
        }
        cells.push(v);
    }
    if let Some(k) = (0..n).find(|&k| cells[k].is_empty()) {
        return invalid(format!("{}: todas sus celdas las ocupa otro departamento", names[k]));
    }
    let c: Vec<Vec<f64>> = (0..n).map(|i| (0..n).map(|j| m.dept_flows.get(i).and_then(|r| r.get(j)).copied().unwrap_or(0.0)).collect()).collect();
    let ly = Lay { c: &c, dm: o.distance };
    let better = |new: f64, old: f64| if m.maximize { new > old + 1e-9 } else { new < old - 1e-9 };
    let init = cells.clone();
    let init_cost = ly.cost(&cells);
    let mut cur = init_cost;
    let mut log = Table::new(Some("Intercambios"), &["Iteración", "Intercambio", "Costo"]).left(1);
    log.row(cells![0, "Distribución inicial", tidy(cur)]);
    let mut it = 0;
    let phases: Vec<u8> = match o.exchange {
        Exchange::TwoWay => vec![2],
        Exchange::ThreeWay => vec![3],
        Exchange::TwoThenThree => vec![2, 3],
        Exchange::ThreeThenTwo => vec![3, 2],
    };
    for ph in phases {
        loop {
            let mut best: Option<(f64, Vec<Vec<Cellp>>, String)> = None;
            let free: Vec<usize> = (0..n).filter(|&k| !m.departments[k].fixed).collect();
            if ph == 2 {
                for (a, &i) in free.iter().enumerate() {
                    for &j in &free[a + 1..] {
                        if let Some(nc) = swap2(&cells, i, j) {
                            let v = ly.cost(&nc);
                            if better(v, best.as_ref().map(|b| b.0).unwrap_or(cur)) {
                                best = Some((v, nc, format!("{} ↔ {}", names[i], names[j])));
                            }
                        }
                    }
                }
            } else {
                for (a, &i) in free.iter().enumerate() {
                    for (b, &j) in free.iter().enumerate().skip(a + 1) {
                        for &k in &free[b + 1..] {
                            if cells[i].len() != cells[j].len() || cells[j].len() != cells[k].len() {
                                continue;
                            }
                            for rot in 0..2 {
                                let mut nc = cells.clone();
                                if rot == 0 {
                                    nc[i] = cells[j].clone();
                                    nc[j] = cells[k].clone();
                                    nc[k] = cells[i].clone();
                                } else {
                                    nc[i] = cells[k].clone();
                                    nc[j] = cells[i].clone();
                                    nc[k] = cells[j].clone();
                                }
                                let v = ly.cost(&nc);
                                if better(v, best.as_ref().map(|b| b.0).unwrap_or(cur)) {
                                    best = Some((v, nc, format!("{} → {} → {} (ciclo)", names[i], names[j], names[k])));
                                }
                            }
                        }
                    }
                }
            }
            match best {
                Some((v, nc, what)) => {
                    it += 1;
                    cells = nc;
                    cur = v;
                    log.row(cells![it, what, tidy(v)]);
                    if it > 10_000 {
                        break;
                    }
                }
                None => break,
            }
        }
    }
    let grid = |cells: &[Vec<Cellp>], title: &str| -> Block {
        let mut head = vec!["Fila \\ Col".to_string()];
        head.extend((1..=cols).map(|c| c.to_string()));
        let mut t = Table::new(Some(title), &head).left(0);
        for i in 1..=cols {
            t = t.left(i);
        }
        let mut own = vec![vec![usize::MAX; cols]; rows];
        for (k, v) in cells.iter().enumerate() {
            for &(r, c) in v {
                own[r][c] = k;
            }
        }
        let tones = [Tone::Accent, Tone::Highlight, Tone::Good, Tone::Bad, Tone::Muted, Tone::Total];
        for r in 0..rows {
            let mut row = vec![Cell::from((r + 1).to_string())];
            row.extend((0..cols).map(|c| if own[r][c] == usize::MAX { Cell::Empty } else { Cell::from(names[own[r][c]]) }));
            t.row(row);
            for c in 0..cols {
                if own[r][c] != usize::MAX {
                    t.mark(r, c + 1, tones[own[r][c] % tones.len()]);
                }
            }
        }
        t.block()
    };
    let mut dt = Table::new(Some("Departamentos"), &["Departamento", "Área (celdas)", "Centro (columna, fila)", "Celdas finales", "Fijo"]).left(0).left(2).left(3).left(4);
    for k in 0..n {
        let ce = centroid(&cells[k]);
        dt.row(cells![names[k], cells[k].len(), format!("({}, {})", fmt(tidy(ce[0])), fmt(tidy(ce[1]))), cells_text(&cells[k]), if m.departments[k].fixed { "sí" } else { "no" }]);
    }
    let mut st = vec![];
    if o.steps {
        st.push(steps(vec![
            step("Costo de una distribución", format!("Para cada par de departamentos: flujo × costo unitario × distancia {} entre sus centros. La suma es el costo de la distribución.", dist_label(o.distance)), vec![grid(&init, "Distribución inicial")]),
            step(
                "Intercambios",
                "En cada iteración se prueban todos los intercambios permitidos (dos departamentos de igual área o vecinos; tres departamentos de igual área en ciclo) y se aplica el que más mejora. Se para cuando ninguno mejora: es un óptimo local.",
                vec![log.clone().block()],
            ),
        ]));
    }
    let improve = if init_cost != 0.0 { 100.0 * (init_cost - cur).abs() / init_cost.abs() } else { 0.0 };
    Ok(Report::new("fll", &m.name, Status::Feasible, format!("Costo {} → {} tras {} intercambios ({} % {}).", fmt(tidy(init_cost)), fmt(tidy(cur)), it, fmt(tidy(improve)), if m.maximize { "más" } else { "menos" }))
        .section("solucion", "Solución", {
            let mut b = vec![metrics(vec![metric_main("Costo final", tidy(cur)), metric("Costo inicial", tidy(init_cost)), metric("Intercambios", it)]), grid(&cells, "Distribución final"), dt.block()];
            if !overlaps.is_empty() {
                b.insert(0, text_tone(format!("Celdas superpuestas en la distribución inicial (gana el departamento posterior): {}.", overlaps.join("; ")), Tone::Accent));
            }
            b
        })
        .section("iteraciones", "Iteraciones", vec![grid(&init, "Distribución inicial"), log.block()])
        .section("pasos", "Paso a paso", st)
        .data(json!({"initial_cost": tidy(init_cost), "final_cost": tidy(cur), "exchanges": it, "layout": (0..n).map(|k| json!({"department": names[k], "cells": cells_text(&cells[k])})).collect::<Vec<_>>()})))
}
