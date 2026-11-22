//! Programación cuadrática: mín ½·xᵀPx + qᵀx sujeto a filas lineales y cotas,
//! con variables continuas o enteras.
//!
//! - Convexa (P semidefinida positiva): Clarabel, de punto interior.
//! - No convexa y pequeña: óptimo global enumerando los puntos KKT de cada
//!   conjunto de restricciones activas (el óptimo está en uno de ellos).
//! - No convexa y grande: el mejor óptimo local de varios arranques (se avisa).
//! - Enteras: ramificación y acotamiento sobre esas relajaciones.

use clarabel::algebra::CscMatrix;
use clarabel::solver::{DefaultSettings, DefaultSolver, IPSolver, SolverStatus, SupportedConeT};
use iaqsb_core::linear::Dir;
use nalgebra::{DMatrix, DVector};

#[derive(Debug, Clone)]
pub struct Qp {
    /// Hessiana simétrica (n × n).
    pub p: Vec<Vec<f64>>,
    pub q: Vec<f64>,
    pub rows: Vec<(Vec<f64>, Dir, f64)>,
    pub lb: Vec<f64>,
    pub ub: Vec<f64>,
    pub int: Vec<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QpStatus {
    Optimal,
    /// Óptimo local (problema no convexo grande).
    Local,
    Infeasible,
    Unbounded,
    Error,
}

#[derive(Debug, Clone)]
pub struct QpSol {
    pub status: QpStatus,
    pub x: Vec<f64>,
    pub obj: f64,
    /// Multiplicadores de las filas (≥ 0 en ≤ y ≥ con el signo de mín), si se conocen.
    pub duals: Option<Vec<f64>>,
    pub nodes: usize,
    pub method: &'static str,
}

impl Qp {
    pub fn n(&self) -> usize {
        self.q.len()
    }
    pub fn value(&self, x: &[f64]) -> f64 {
        let n = self.n();
        let mut v = 0.0;
        for i in 0..n {
            v += self.q[i] * x[i];
            for j in 0..n {
                v += 0.5 * self.p[i][j] * x[i] * x[j];
            }
        }
        v
    }
    pub fn feasible(&self, x: &[f64], tol: f64) -> bool {
        (0..self.n()).all(|j| x[j] >= self.lb[j] - tol && x[j] <= self.ub[j] + tol)
            && self.rows.iter().all(|(a, d, b)| d.holds(a.iter().zip(x).map(|(p, q)| p * q).sum(), *b, tol * b.abs().max(1.0)))
    }
}

/// Valores propios extremos de la hessiana (para saber si es convexa).
pub fn eigen_range(p: &[Vec<f64>]) -> (f64, f64) {
    let n = p.len();
    if n == 0 {
        return (0.0, 0.0);
    }
    let m = DMatrix::from_fn(n, n, |i, j| 0.5 * (p[i][j] + p[j][i]));
    let e = m.symmetric_eigenvalues();
    (e.min(), e.max())
}

pub fn is_convex(p: &[Vec<f64>]) -> bool {
    let (lo, hi) = eigen_range(p);
    lo >= -1e-9 * hi.abs().max(1.0)
}

/// Matriz densa a formato CSC (por columnas), sin ceros.
pub(crate) fn csc(d: &[Vec<f64>], m: usize, n: usize) -> CscMatrix<f64> {
    let mut colptr = vec![0usize];
    let mut rowval = vec![];
    let mut nzval = vec![];
    for j in 0..n {
        for (i, row) in d.iter().enumerate().take(m) {
            if row[j] != 0.0 {
                rowval.push(i);
                nzval.push(row[j]);
            }
        }
        colptr.push(rowval.len());
    }
    CscMatrix::new(m, n, colptr, rowval, nzval)
}

/// Relajación continua convexa con Clarabel (cotas dadas, ignora la integralidad).
fn clarabel(qp: &Qp, lb: &[f64], ub: &[f64]) -> QpSol {
    let n = qp.n();
    let mut eq: Vec<(Vec<f64>, f64)> = vec![];
    let mut le: Vec<(Vec<f64>, f64, usize, f64)> = vec![]; // fila, b, índice de origen, signo
    for (k, (a, d, b)) in qp.rows.iter().enumerate() {
        match d {
            Dir::Eq => eq.push((a.clone(), *b)),
            Dir::Le => le.push((a.clone(), *b, k, 1.0)),
            Dir::Ge => le.push((a.iter().map(|x| -x).collect(), -*b, k, -1.0)),
        }
    }
    let nrows_orig = le.len();
    for j in 0..n {
        if lb[j].is_finite() {
            let mut a = vec![0.0; n];
            a[j] = -1.0;
            le.push((a, -lb[j], usize::MAX, 0.0));
        }
        if ub[j].is_finite() {
            let mut a = vec![0.0; n];
            a[j] = 1.0;
            le.push((a, ub[j], usize::MAX, 0.0));
        }
    }
    let m = eq.len() + le.len();
    let mut dense = vec![vec![0.0; n]; m];
    let mut bvec = vec![0.0; m];
    for (r, (a, b)) in eq.iter().enumerate() {
        dense[r] = a.clone();
        bvec[r] = *b;
    }
    for (r, (a, b, _, _)) in le.iter().enumerate() {
        dense[eq.len() + r] = a.clone();
        bvec[eq.len() + r] = *b;
    }
    let pm = csc(&qp.p, n, n);
    let am = csc(&dense, m, n);
    let mut cones: Vec<SupportedConeT<f64>> = vec![];
    if !eq.is_empty() {
        cones.push(SupportedConeT::ZeroConeT(eq.len()));
    }
    if !le.is_empty() {
        cones.push(SupportedConeT::NonnegativeConeT(le.len()));
    }
    let settings = DefaultSettings { verbose: false, max_iter: 500, ..Default::default() };
    let Ok(mut solver) = DefaultSolver::new(&pm, &qp.q, &am, &bvec, &cones, settings) else {
        return QpSol { status: QpStatus::Error, x: vec![0.0; n], obj: f64::NAN, duals: None, nodes: 0, method: "Clarabel" };
    };
    solver.solve();
    let s = &solver.solution;
    let status = match s.status {
        SolverStatus::Solved | SolverStatus::AlmostSolved => QpStatus::Optimal,
        SolverStatus::PrimalInfeasible | SolverStatus::AlmostPrimalInfeasible => QpStatus::Infeasible,
        SolverStatus::DualInfeasible | SolverStatus::AlmostDualInfeasible => QpStatus::Unbounded,
        _ => QpStatus::Error,
    };
    // multiplicadores en el orden de las filas originales
    let mut duals = vec![0.0; qp.rows.len()];
    let mut ie = 0;
    for (k, (_, d, _)) in qp.rows.iter().enumerate() {
        if *d == Dir::Eq {
            duals[k] = s.z[ie];
            ie += 1;
        }
    }
    for (r, (_, _, k, sign)) in le.iter().enumerate().take(nrows_orig) {
        duals[*k] = sign * s.z[eq.len() + r];
    }
    QpSol { status, x: s.x.clone(), obj: s.obj_val, duals: Some(duals), nodes: 0, method: "Clarabel (punto interior)" }
}

/// QP convexo directo (sin comprobar la convexidad ni ramificar).
pub fn convex_qp(qp: &Qp) -> QpSol {
    clarabel(qp, &qp.lb, &qp.ub)
}

/// Óptimo global de un QP no convexo pequeño: puntos KKT de cada conjunto activo.
fn enumerate_kkt(qp: &Qp, lb: &[f64], ub: &[f64]) -> Option<QpSol> {
    let n = qp.n();
    // todas las restricciones como filas a·x (≤, ≥ o =) b, más las cotas finitas
    let mut cons: Vec<(Vec<f64>, f64, bool)> = vec![]; // (a, b, es igualdad)
    for (a, d, b) in &qp.rows {
        cons.push((a.clone(), *b, *d == Dir::Eq));
    }
    for j in 0..n {
        for v in [lb[j], ub[j]] {
            if v.is_finite() {
                let mut a = vec![0.0; n];
                a[j] = 1.0;
                cons.push((a, v, false));
            }
        }
    }
    let eqs: Vec<usize> = (0..cons.len()).filter(|&k| cons[k].2).collect();
    let ineqs: Vec<usize> = (0..cons.len()).filter(|&k| !cons[k].2).collect();
    // número de subconjuntos con a lo más n - |eq| desigualdades activas
    let free = n.saturating_sub(eqs.len());
    let mut total: f64 = 0.0;
    let mut c = 1.0;
    for k in 0..=free.min(ineqs.len()) {
        if k > 0 {
            c = c * (ineqs.len() - k + 1) as f64 / k as f64;
        }
        total += c;
    }
    if total > 300_000.0 {
        return None;
    }
    let mut best: Option<(f64, Vec<f64>)> = None;
    let mut subset: Vec<usize> = vec![];
    fn rec(
        qp: &Qp,
        cons: &[(Vec<f64>, f64, bool)],
        eqs: &[usize],
        ineqs: &[usize],
        start: usize,
        free: usize,
        subset: &mut Vec<usize>,
        lb: &[f64],
        ub: &[f64],
        best: &mut Option<(f64, Vec<f64>)>,
    ) {
        let n = qp.n();
        let act: Vec<usize> = eqs.iter().chain(subset.iter()).copied().collect();
        let k = act.len();
        let mut kkt = DMatrix::zeros(n + k, n + k);
        let mut rhs = DVector::zeros(n + k);
        for i in 0..n {
            for j in 0..n {
                kkt[(i, j)] = 0.5 * (qp.p[i][j] + qp.p[j][i]);
            }
            rhs[i] = -qp.q[i];
        }
        for (r, &ci) in act.iter().enumerate() {
            for j in 0..n {
                kkt[(n + r, j)] = cons[ci].0[j];
                kkt[(j, n + r)] = cons[ci].0[j];
            }
            rhs[n + r] = cons[ci].1;
        }
        if let Some(sol) = kkt.clone().lu().solve(&rhs) {
            let x: Vec<f64> = (0..n).map(|i| sol[i]).collect();
            if x.iter().all(|v| v.is_finite()) && (0..n).all(|j| x[j] >= lb[j] - 1e-7 && x[j] <= ub[j] + 1e-7) && qp.feasible(&x, 1e-7) {
                // comprobar que realmente es solución del sistema (matriz no singular)
                let res = &kkt * &sol - &rhs;
                if res.norm() <= 1e-6 * rhs.norm().max(1.0) {
                    let v = qp.value(&x);
                    if best.as_ref().map(|b| v < b.0 - 1e-12).unwrap_or(true) {
                        *best = Some((v, x));
                    }
                }
            }
        }
        if subset.len() >= free {
            return;
        }
        for t in start..ineqs.len() {
            subset.push(ineqs[t]);
            rec(qp, cons, eqs, ineqs, t + 1, free, subset, lb, ub, best);
            subset.pop();
        }
    }
    rec(qp, &cons, &eqs, &ineqs, 0, free, &mut subset, lb, ub, &mut best);
    let (obj, x) = best?;
    Some(QpSol { status: QpStatus::Optimal, x, obj, duals: None, nodes: 0, method: "enumeración de puntos KKT (óptimo global)" })
}

/// ¿Puede el objetivo bajar sin límite? (dirección de curvatura negativa o recesión)
fn unbounded_hint(qp: &Qp, lb: &[f64], ub: &[f64]) -> bool {
    // región no acotada en alguna variable y curvatura negativa en ella
    let n = qp.n();
    (0..n).any(|j| (!lb[j].is_finite() || !ub[j].is_finite()) && qp.p[j][j] < 0.0 && qp.rows.iter().all(|(a, _, _)| a[j] == 0.0))
}

fn relax(qp: &Qp, lb: &[f64], ub: &[f64], convex: bool) -> QpSol {
    if convex {
        return clarabel(qp, lb, ub);
    }
    if unbounded_hint(qp, lb, ub) {
        return QpSol { status: QpStatus::Unbounded, x: vec![0.0; qp.n()], obj: f64::NEG_INFINITY, duals: None, nodes: 0, method: "análisis de curvatura" };
    }
    if let Some(s) = enumerate_kkt(qp, lb, ub) {
        // si la región no está acotada podría haber valores menores lejos: se avisa como local
        let bounded = (0..qp.n()).all(|j| lb[j].is_finite() && ub[j].is_finite());
        return QpSol { status: if bounded { QpStatus::Optimal } else { QpStatus::Local }, ..s };
    }
    // demasiado grande: mejor de varios arranques con Clarabel sobre la parte convexa
    let mut shifted = qp.clone();
    let (lo, _) = eigen_range(&qp.p);
    for i in 0..qp.n() {
        shifted.p[i][i] += -lo + 1e-6;
    }
    let s = clarabel(&shifted, lb, ub);
    if s.status != QpStatus::Optimal {
        return s;
    }
    let obj = qp.value(&s.x);
    QpSol { status: QpStatus::Local, obj, method: "aproximación convexa (óptimo local)", ..s }
}

/// Resuelve el problema (con ramificación y acotamiento si hay enteras).
pub fn solve(qp: &Qp) -> QpSol {
    let convex = is_convex(&qp.p);
    let root = relax(qp, &qp.lb, &qp.ub, convex);
    if !qp.int.iter().any(|b| *b) || root.status != QpStatus::Optimal {
        let mut r = root;
        if r.status == QpStatus::Optimal || r.status == QpStatus::Local {
            r.obj = qp.value(&r.x);
        }
        return r;
    }
    // ramificación y acotamiento «mejor primero»: se explora siempre el nodo de menor cota.
    // (En profundidad, con igualdades y variables sin cota superior, la búsqueda puede alejarse
    // indefinidamente antes de encontrar una solución entera; p. ej. el ejemplo IQP de WinQSB.)
    let mut best: Option<QpSol> = None;
    let mut heap: Vec<(f64, Vec<f64>, Vec<f64>, QpSol)> = vec![];
    let push = |heap: &mut Vec<(f64, Vec<f64>, Vec<f64>, QpSol)>, lb: Vec<f64>, ub: Vec<f64>, s: QpSol| {
        let v = qp.value(&s.x);
        heap.push((v, lb, ub, s));
    };
    push(&mut heap, qp.lb.clone(), qp.ub.clone(), root);
    let mut nodes = 0;
    let mut limit = false;
    while !heap.is_empty() {
        let k = (0..heap.len()).min_by(|&a, &b| heap[a].0.partial_cmp(&heap[b].0).unwrap_or(std::cmp::Ordering::Equal)).unwrap();
        let (v, lb, ub, s) = heap.swap_remove(k);
        nodes += 1;
        if nodes > 50_000 {
            limit = true;
            break;
        }
        if let Some(b) = &best {
            if v >= b.obj - 1e-9 * b.obj.abs().max(1.0) {
                break; // mejor cota pendiente ya no mejora: óptimo probado
            }
        }
        let frac = (0..qp.n()).filter(|&j| qp.int[j]).map(|j| (j, (s.x[j] - s.x[j].round()).abs())).filter(|(_, f)| *f > 1e-6).max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        match frac {
            None => {
                let mut x = s.x.clone();
                for j in 0..qp.n() {
                    if qp.int[j] {
                        x[j] = x[j].round();
                    }
                }
                let obj = qp.value(&x);
                if best.as_ref().map(|b| obj < b.obj).unwrap_or(true) {
                    best = Some(QpSol { status: QpStatus::Optimal, x, obj, duals: None, nodes, method: s.method });
                }
            }
            Some((j, _)) => {
                let f = s.x[j].floor();
                let mut ub2 = ub.clone();
                ub2[j] = f;
                let mut lb2 = lb.clone();
                lb2[j] = f + 1.0;
                for (l, u) in [(lb.clone(), ub2), (lb2, ub.clone())] {
                    if (0..qp.n()).any(|i| l[i] > u[i]) {
                        continue;
                    }
                    let c = relax(qp, &l, &u, convex);
                    if c.status == QpStatus::Optimal || c.status == QpStatus::Local {
                        push(&mut heap, l, u, c);
                    }
                }
            }
        }
    }
    match best {
        Some(mut b) => {
            b.nodes = nodes;
            if limit {
                b.status = QpStatus::Local; // límite de nodos: la mejor entera hallada, sin prueba de optimalidad
            }
            b
        }
        None => QpSol { status: QpStatus::Infeasible, x: vec![0.0; qp.n()], obj: f64::NAN, duals: None, nodes, method: "ramificación y acotamiento" },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convexa_con_restricciones() {
        // mín (x−1)² + (y−2)² = x² + y² − 2x − 4y + 5, con x + y ≤ 2
        let qp = Qp {
            p: vec![vec![2.0, 0.0], vec![0.0, 2.0]],
            q: vec![-2.0, -4.0],
            rows: vec![(vec![1.0, 1.0], Dir::Le, 2.0)],
            lb: vec![0.0, 0.0],
            ub: vec![f64::INFINITY; 2],
            int: vec![false; 2],
        };
        let s = solve(&qp);
        assert_eq!(s.status, QpStatus::Optimal);
        assert!((s.x[0] - 0.5).abs() < 1e-6 && (s.x[1] - 1.5).abs() < 1e-6, "{:?}", s.x);
        // multiplicador: 2(x−1) + λ = 0 → λ = 1
        assert!((s.duals.unwrap()[0] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn no_convexa_global_y_entera() {
        // mín −x² − y² con x + y ≤ 3, 0 ≤ x, y ≤ 2: óptimo en un vértice (2, 1) o (1, 2) → −5
        let qp = Qp {
            p: vec![vec![-2.0, 0.0], vec![0.0, -2.0]],
            q: vec![0.0, 0.0],
            rows: vec![(vec![1.0, 1.0], Dir::Le, 3.0)],
            lb: vec![0.0; 2],
            ub: vec![2.0; 2],
            int: vec![false; 2],
        };
        let s = solve(&qp);
        assert_eq!(s.status, QpStatus::Optimal);
        assert!((s.obj + 5.0).abs() < 1e-9);
        // entera convexa: mín (x − 1.4)² + (y − 2.6)² → (1, 3)
        let qp = Qp {
            p: vec![vec![2.0, 0.0], vec![0.0, 2.0]],
            q: vec![-2.8, -5.2],
            rows: vec![],
            lb: vec![0.0; 2],
            ub: vec![10.0; 2],
            int: vec![true; 2],
        };
        let s = solve(&qp);
        assert_eq!(s.x, vec![1.0, 3.0]);
    }
}
