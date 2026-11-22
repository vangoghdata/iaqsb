//! Método de Wolfe (símplex modificado) para programación cuadrática convexa,
//! el que traen los libros: se escriben las condiciones de Karush-Kuhn-Tucker
//! como un sistema lineal y se resuelve la fase uno del símplex con la regla de
//! entrada restringida (una variable no entra si su complementaria es básica).
//!
//!   P·x + Aᵀu − y = −q        (estacionariedad)
//!   A·x + v = b               (factibilidad)
//!   x, u, y, v ≥ 0,  x·y = 0,  u·v = 0   (holgura complementaria)

use iaqsb_core::report::*;

pub struct WolfeOut {
    pub steps: Vec<Step>,
    pub x: Option<Vec<f64>>,
}

const EPS: f64 = 1e-9;

/// `p`, `q`: objetivo a minimizar ½xᵀPx + qᵀx; `a`, `b`: filas A·x ≤ b; x ≥ 0.
pub fn run(names: &[String], row_names: &[String], p: &[Vec<f64>], q: &[f64], a: &[Vec<f64>], b: &[f64]) -> WolfeOut {
    let n = q.len();
    let m = b.len();
    // columnas: x (n), u (m), y (n), v (m), artificiales
    let mut cols: Vec<String> = names.to_vec();
    cols.extend(row_names.iter().map(|r| format!("u{}", short(r))));
    cols.extend(names.iter().map(|x| format!("y{}", short(x))));
    cols.extend(row_names.iter().map(|r| format!("v{}", short(r))));
    let base_cols = cols.len();
    let rows = n + m;
    let mut t = vec![vec![0.0; base_cols]; rows];
    let mut rhs = vec![0.0; rows];
    for j in 0..n {
        for k in 0..n {
            t[j][k] = p[j][k];
        }
        for i in 0..m {
            t[j][n + i] = a[i][j];
        }
        t[j][n + m + j] = -1.0;
        rhs[j] = -q[j];
    }
    for i in 0..m {
        for k in 0..n {
            t[n + i][k] = a[i][k];
        }
        t[n + i][2 * n + m + i] = 1.0;
        rhs[n + i] = b[i];
    }
    // base inicial: artificial en cada fila de estacionariedad; v_i si b ≥ 0; si no, artificial
    let mut basis = vec![0usize; rows];
    let mut arts = vec![];
    for r in 0..rows {
        let natural = if r < n { n + m + r } else { 2 * n + m + (r - n) };
        let coef = t[r][natural];
        // como en el libro: artificiales en las filas de estacionariedad (así x e y empiezan fuera
        // de la base) y la holgura v como básica en las de factibilidad cuando b ≥ 0
        if r >= n && (rhs[r] * coef > EPS || (rhs[r].abs() <= EPS && coef > 0.0)) {
            // la variable natural ya sirve (se ajusta el signo de la fila)
            if coef < 0.0 {
                t[r].iter_mut().for_each(|x| *x = -*x);
                rhs[r] = -rhs[r];
            }
            basis[r] = natural;
        } else {
            if rhs[r] < 0.0 {
                t[r].iter_mut().for_each(|x| *x = -*x);
                rhs[r] = -rhs[r];
            }
            for (k, row) in t.iter_mut().enumerate() {
                row.push(if k == r { 1.0 } else { 0.0 });
            }
            cols.push(format!("Z{}", arts.len() + 1));
            basis[r] = cols.len() - 1;
            arts.push(cols.len() - 1);
        }
    }
    let nc = cols.len();
    let complement = |j: usize| -> Option<usize> {
        if j < n {
            Some(n + m + j)
        } else if j < n + m {
            Some(2 * n + m + (j - n))
        } else if j < 2 * n + m {
            Some(j - n - m)
        } else if j < base_cols {
            Some(j - 2 * n - m + n)
        } else {
            None
        }
    };
    let cost: Vec<f64> = (0..nc).map(|j| if arts.contains(&j) { 1.0 } else { 0.0 }).collect();

    let render = |t: &Vec<Vec<f64>>, rhs: &Vec<f64>, basis: &Vec<usize>, piv: Option<(usize, usize)>, ratios: Option<&Vec<Option<f64>>>| -> Block {
        let mut head = vec!["Base".to_string()];
        head.extend(cols.iter().cloned());
        head.push("LD".into());
        if ratios.is_some() {
            head.push("Razón".into());
        }
        let mut tb = Table::new(None, &head).left(0);
        for r in 0..rows {
            let mut row = vec![Cell::from(&cols[basis[r]])];
            row.extend(t[r].iter().map(|x| Cell::Num(clean(*x))));
            row.push(Cell::Num(clean(rhs[r])));
            if let Some(rt) = ratios {
                row.push(rt[r].map(|v| Cell::Num(clean(v))).unwrap_or(Cell::from("—")));
            }
            tb.row(row);
        }
        let mut red = vec![Cell::from("Cj − Zj")];
        for j in 0..nc {
            let zj: f64 = (0..rows).map(|r| cost[basis[r]] * t[r][j]).sum();
            red.push(Cell::Num(clean(cost[j] - zj)));
        }
        red.push(Cell::Num(clean((0..rows).map(|r| cost[basis[r]] * rhs[r]).sum())));
        if ratios.is_some() {
            red.push(Cell::Empty);
        }
        tb.row_tone(red, Tone::Total);
        if let Some((pr, pc)) = piv {
            tb.mark(pr, pc + 1, Tone::Highlight);
            tb.col_marks.push((pc + 1, Tone::Accent));
        }
        tb.block()
    };

    let mut steps = vec![step(
        "Condiciones de Karush-Kuhn-Tucker",
        "El óptimo de un programa cuadrático convexo cumple un sistema lineal (estacionariedad y factibilidad) más la holgura complementaria. Se buscan con la fase uno del símplex: se minimiza la suma de las artificiales Z, y una variable no puede entrar si su complementaria (x con y, u con v) está en la base.",
        vec![render(&t, &rhs, &basis, None, None)],
    )];
    let mut it = 0;
    loop {
        let red: Vec<f64> = (0..nc).map(|j| cost[j] - (0..rows).map(|r| cost[basis[r]] * t[r][j]).sum::<f64>()).collect();
        let w: f64 = (0..rows).map(|r| cost[basis[r]] * rhs[r]).sum();
        let candidates: Vec<usize> = (0..nc)
            .filter(|&j| !basis.contains(&j) && !arts.contains(&j) && red[j] < -EPS)
            .filter(|&j| complement(j).map(|c| !basis.contains(&c)).unwrap_or(true))
            .collect();
        // también puede entrar una variable cuya complementaria es básica si es justo esa la que sale
        let leaving_row = |j: usize| -> Option<usize> {
            (0..rows)
                .filter(|&r| t[r][j] > EPS)
                .min_by(|&a, &b| (rhs[a] / t[a][j]).partial_cmp(&(rhs[b] / t[b][j])).unwrap().then(basis[a].cmp(&basis[b])))
        };
        let swaps: Vec<usize> = (0..nc)
            .filter(|&j| !basis.contains(&j) && !arts.contains(&j) && red[j] < -EPS && !candidates.contains(&j))
            .filter(|&j| {
                let Some(c) = complement(j) else { return false };
                let Some(r) = leaving_row(j) else { return false };
                // la que sale es la complementaria (o, con empate en la razón, puede serlo)
                let best = rhs[r] / t[r][j];
                (0..rows).any(|k| basis[k] == c && t[k][j] > EPS && (rhs[k] / t[k][j] - best).abs() <= 1e-12)
            })
            .collect();
        let mut candidates = candidates;
        let mut forced_leave: Option<usize> = None;
        if candidates.is_empty() && !swaps.is_empty() {
            let j = *swaps.iter().min_by(|&&a, &&b| red[a].partial_cmp(&red[b]).unwrap()).unwrap();
            let c = complement(j).unwrap();
            forced_leave = (0..rows).find(|&k| basis[k] == c);
            candidates.push(j);
        }
        let entering = if it > 3 * nc { candidates.first().copied() } else { candidates.iter().copied().min_by(|&a, &b| red[a].partial_cmp(&red[b]).unwrap()) };
        let Some(j) = entering else {
            if w > 1e-7 {
                steps.push(step("Sin solución", "La fase uno no llega a cero: no hay punto que cumpla las condiciones (el problema no es factible o no es convexo).", vec![render(&t, &rhs, &basis, None, None)]));
                return WolfeOut { steps, x: None };
            }
            let mut val = vec![0.0; nc];
            for r in 0..rows {
                val[basis[r]] = rhs[r];
            }
            // holgura complementaria
            if (0..base_cols).any(|j| complement(j).map(|c| val[j] > 1e-9 && val[c] > 1e-9).unwrap_or(false)) {
                return WolfeOut { steps, x: None };
            }
            let x: Vec<f64> = val[..n].to_vec();
            steps.push(step(
                "Solución",
                "Las artificiales valen cero y se cumple la holgura complementaria: los valores de x en la base son el óptimo; u son los multiplicadores de las restricciones.",
                vec![render(&t, &rhs, &basis, None, None)],
            ));
            return WolfeOut { steps, x: Some(x) };
        };
        let ratios: Vec<Option<f64>> = (0..rows).map(|r| if t[r][j] > EPS { Some(rhs[r] / t[r][j]) } else { None }).collect();
        let Some(r) = forced_leave.or_else(|| (0..rows).filter(|&r| ratios[r].is_some()).min_by(|&a, &b| ratios[a].unwrap().partial_cmp(&ratios[b].unwrap()).unwrap().then(basis[a].cmp(&basis[b])))) else {
            return WolfeOut { steps, x: None };
        };
        it += 1;
        let blocked = (0..nc).filter(|&k| !basis.contains(&k) && red[k] < -EPS && !candidates.contains(&k) && !arts.contains(&k)).map(|k| cols[k].clone()).collect::<Vec<_>>();
        let extra = if blocked.is_empty() { String::new() } else { format!(" No pueden entrar {} porque su complementaria está en la base.", blocked.join(", ")) };
        if steps.len() < 40 {
            steps.push(step(
                format!("Iteración {it}"),
                format!("Entra {} (Cj − Zj = {}) y sale {} (razón mínima {}).{extra}", cols[j], fmt(red[j]), cols[basis[r]], fmt(ratios[r].unwrap())),
                vec![render(&t, &rhs, &basis, Some((r, j)), Some(&ratios))],
            ));
        }
        let pv = t[r][j];
        t[r].iter_mut().for_each(|x| *x /= pv);
        rhs[r] /= pv;
        for k in 0..rows {
            if k != r && t[k][j] != 0.0 {
                let f = t[k][j];
                for c in 0..nc {
                    t[k][c] -= f * t[r][c];
                }
                rhs[k] -= f * rhs[r];
            }
        }
        basis[r] = j;
        if it > 500 {
            return WolfeOut { steps, x: None };
        }
    }
}

fn short(s: &str) -> String {
    s.chars().take(6).collect()
}
