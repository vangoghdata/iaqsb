//! Programación no lineal: mín f(x) con restricciones gᵢ(x) ≤ 0, hₖ(x) = 0 y
//! cotas. Programación cuadrática sucesiva (SQP) con hessiana BFGS amortiguada
//! (Powell), subproblemas cuadráticos elásticos resueltos con Clarabel y
//! búsqueda lineal sobre la función de mérito L1. Derivadas exactas por
//! diferenciación automática. Varios arranques para buscar el mejor óptimo.
//!
//! También: el método de penalización exterior (el de WinQSB), para el modo
//! didáctico.

use crate::qpsolve::{convex_qp, Qp, QpStatus};
use iaqsb_core::expr::Expr;
use iaqsb_core::linear::Dir;
use rand::{RngExt, SeedableRng};

#[derive(Clone)]
pub struct Nlp {
    pub f: Expr,
    /// Restricciones c(x) = lhs − rhs con su sentido (≤ 0, ≥ 0 o = 0).
    pub cons: Vec<(Expr, Dir)>,
    pub lb: Vec<f64>,
    pub ub: Vec<f64>,
    /// −1 para maximizar (se minimiza −f).
    pub sign: f64,
}

#[derive(Debug, Clone)]
pub struct NlpSol {
    pub x: Vec<f64>,
    /// Valor de f (con su signo original).
    pub f: f64,
    pub viol: f64,
    /// Multiplicadores (convención min: ∇f + Σ λ∇c = 0; λ ≥ 0 en ≤).
    pub lambda: Vec<f64>,
    pub iters: usize,
    pub converged: bool,
    pub unbounded: bool,
    pub history: Vec<(Vec<f64>, f64, f64)>,
}

impl Nlp {
    pub fn n(&self) -> usize {
        self.lb.len()
    }
    /// f en forma de minimizar, con gradiente.
    fn obj(&self, x: &[f64]) -> (f64, Vec<f64>) {
        let (v, g) = self.f.eval_grad(x);
        (self.sign * v, g.iter().map(|d| self.sign * d).collect())
    }
    /// Restricciones en forma c(x) ≤ 0 / = 0 (las ≥ se cambian de signo).
    fn cons_at(&self, x: &[f64]) -> Vec<(f64, Vec<f64>, bool)> {
        self.cons
            .iter()
            .map(|(e, d)| {
                let (v, g) = e.eval_grad(x);
                match d {
                    Dir::Le => (v, g, false),
                    Dir::Ge => (-v, g.iter().map(|t| -t).collect(), false),
                    Dir::Eq => (v, g, true),
                }
            })
            .collect()
    }
    pub fn violation(&self, x: &[f64]) -> f64 {
        let mut v: f64 = 0.0;
        for (c, _, eq) in self.cons_at(x) {
            v = v.max(if eq { c.abs() } else { c.max(0.0) });
        }
        for j in 0..self.n() {
            v = v.max(self.lb[j] - x[j]).max(x[j] - self.ub[j]);
        }
        v
    }
    pub fn value(&self, x: &[f64]) -> f64 {
        self.f.eval(x)
    }
    fn clamp(&self, x: &mut [f64]) {
        for j in 0..x.len() {
            x[j] = x[j].clamp(self.lb[j], self.ub[j]);
        }
    }
}

fn norm_inf(v: &[f64]) -> f64 {
    v.iter().fold(0.0, |a, b| a.max(b.abs()))
}

/// Un arranque de SQP desde x0.
pub fn sqp(p: &Nlp, x0: &[f64], max_iter: usize) -> NlpSol {
    let n = p.n();
    let m = p.cons.len();
    let mut x = x0.to_vec();
    p.clamp(&mut x);
    let mut b = vec![vec![0.0; n]; n];
    for (i, r) in b.iter_mut().enumerate() {
        r[i] = 1.0;
    }
    let mut mu: f64 = 10.0;
    let mut lambda = vec![0.0; m];
    let mut history = vec![];
    let mut converged = false;
    let mut unbounded = false;
    let mut it = 0;
    let merit = |x: &[f64], mu: f64| -> f64 {
        let (f, _) = p.obj(x);
        let pen: f64 = p.cons_at(x).iter().map(|(c, _, eq)| if *eq { c.abs() } else { c.max(0.0) }).sum();
        f + mu * pen
    };
    while it < max_iter {
        it += 1;
        let (f, g) = p.obj(&x);
        if !f.is_finite() || g.iter().any(|v| !v.is_finite()) {
            break;
        }
        history.push((x.clone(), p.sign * f, p.violation(&x)));
        if f < -1e15 || norm_inf(&x) > 1e12 {
            unbounded = true;
            break;
        }
        let cs = p.cons_at(&x);
        // subproblema elástico: variables d (n) y holguras t (m)
        let rho = 1e3 * (1.0 + norm_inf(&g));
        let mut qp = Qp {
            p: vec![vec![0.0; n + m]; n + m],
            q: vec![0.0; n + m],
            rows: vec![],
            lb: vec![0.0; n + m],
            ub: vec![f64::INFINITY; n + m],
            int: vec![false; n + m],
        };
        for i in 0..n {
            for j in 0..n {
                qp.p[i][j] = b[i][j];
            }
            qp.q[i] = g[i];
            // paso acotado (región de confianza amplia) y respetando las cotas
            let tr = 1e3 * (1.0 + x[i].abs());
            qp.lb[i] = (p.lb[i] - x[i]).max(-tr);
            qp.ub[i] = (p.ub[i] - x[i]).min(tr);
        }
        for (k, (c, gc, eq)) in cs.iter().enumerate() {
            qp.q[n + k] = rho;
            let mut a = vec![0.0; n + m];
            a[..n].copy_from_slice(gc);
            if *eq {
                a[n + k] = -1.0;
                qp.rows.push((a.clone(), Dir::Le, -c));
                let mut a2: Vec<f64> = gc.iter().map(|v| -v).collect();
                a2.resize(n + m, 0.0);
                a2[n + k] = -1.0;
                qp.rows.push((a2, Dir::Le, *c));
            } else {
                a[n + k] = -1.0;
                qp.rows.push((a, Dir::Le, -c));
            }
        }
        let s = convex_qp(&qp);
        if s.status != QpStatus::Optimal {
            break;
        }
        let d: Vec<f64> = s.x[..n].to_vec();
        // multiplicadores de las restricciones originales
        if let Some(du) = &s.duals {
            let mut r = 0;
            for (k, (_, _, eq)) in cs.iter().enumerate() {
                if *eq {
                    lambda[k] = du[r] - du[r + 1];
                    r += 2;
                } else {
                    lambda[k] = du[r];
                    r += 1;
                }
            }
        }
        let viol0 = cs.iter().map(|(c, _, eq)| if *eq { c.abs() } else { c.max(0.0) }).sum::<f64>();
        if norm_inf(&d) <= 1e-10 * (1.0 + norm_inf(&x)) {
            converged = viol0 <= 1e-8;
            if converged {
                break;
            }
        }
        mu = mu.max(1.1 * norm_inf(&lambda)).max(1.0);
        // búsqueda lineal (Armijo sobre el mérito L1)
        let phi0 = merit(&x, mu);
        let dd: f64 = g.iter().zip(&d).map(|(a, b)| a * b).sum::<f64>() - mu * viol0;
        let mut alpha = 1.0;
        let mut xn = x.clone();
        let mut accepted = false;
        for _ in 0..40 {
            for j in 0..n {
                xn[j] = x[j] + alpha * d[j];
            }
            p.clamp(&mut xn);
            if merit(&xn, mu) <= phi0 + 1e-4 * alpha * dd.min(0.0) {
                accepted = true;
                break;
            }
            alpha *= 0.5;
        }
        if !accepted {
            // paso muy corto: se toma igualmente si mejora algo
            if merit(&xn, mu) >= phi0 {
                converged = viol0 <= 1e-7 && norm_inf(&d) <= 1e-6 * (1.0 + norm_inf(&x));
                break;
            }
        }
        // BFGS amortiguado (Powell) sobre el gradiente del lagrangiano
        let lag_grad = |x: &[f64]| -> Vec<f64> {
            let (_, g) = p.obj(x);
            let cs = p.cons_at(x);
            (0..n).map(|j| g[j] + cs.iter().enumerate().map(|(k, (_, gc, _))| lambda[k] * gc[j]).sum::<f64>()).collect()
        };
        let sv: Vec<f64> = (0..n).map(|j| xn[j] - x[j]).collect();
        let ga = lag_grad(&x);
        let gb = lag_grad(&xn);
        let yv: Vec<f64> = (0..n).map(|j| gb[j] - ga[j]).collect();
        let bs: Vec<f64> = (0..n).map(|i| (0..n).map(|j| b[i][j] * sv[j]).sum()).collect();
        let sbs: f64 = sv.iter().zip(&bs).map(|(a, c)| a * c).sum();
        let sy: f64 = sv.iter().zip(&yv).map(|(a, c)| a * c).sum();
        if sbs > 1e-14 {
            let theta = if sy >= 0.2 * sbs { 1.0 } else { 0.8 * sbs / (sbs - sy) };
            let r: Vec<f64> = (0..n).map(|j| theta * yv[j] + (1.0 - theta) * bs[j]).collect();
            let sr: f64 = sv.iter().zip(&r).map(|(a, c)| a * c).sum();
            if sr > 1e-14 {
                for i in 0..n {
                    for j in 0..n {
                        b[i][j] += -bs[i] * bs[j] / sbs + r[i] * r[j] / sr;
                    }
                }
            }
        }
        let step = norm_inf(&sv);
        x = xn;
        if step <= 1e-12 * (1.0 + norm_inf(&x)) && p.violation(&x) <= 1e-8 {
            converged = true;
            break;
        }
    }
    let (f, _) = p.obj(&x);
    NlpSol { f: p.sign * f, viol: p.violation(&x), x, lambda, iters: it, converged, unbounded, history }
}

/// Pulido final: Newton sobre las condiciones KKT de las restricciones activas
/// (hessiana del lagrangiano por diferencias de gradientes exactos). Corrige el
/// estancamiento de la búsqueda lineal junto al óptimo (efecto Maratos).
pub fn polish(p: &Nlp, s: &NlpSol) -> Option<NlpSol> {
    use nalgebra::{DMatrix, DVector};
    let n = p.n();
    let mut x = s.x.clone();
    let cs = p.cons_at(&x);
    // activas: igualdades y desigualdades casi saturadas con multiplicador positivo
    let act: Vec<usize> = (0..cs.len()).filter(|&k| cs[k].2 || (cs[k].0 > -1e-5 * (1.0 + cs[k].0.abs()) && s.lambda[k] > 1e-9)).collect();
    // variables fijas en una cota
    let fixed: Vec<bool> = (0..n).map(|j| (p.lb[j].is_finite() && (x[j] - p.lb[j]).abs() <= 1e-7 * (1.0 + p.lb[j].abs())) || (p.ub[j].is_finite() && (x[j] - p.ub[j]).abs() <= 1e-7 * (1.0 + p.ub[j].abs()))).collect();
    for j in 0..n {
        if fixed[j] {
            x[j] = if (x[j] - p.lb[j]).abs() < (x[j] - p.ub[j]).abs() { p.lb[j] } else { p.ub[j] };
        }
    }
    let free: Vec<usize> = (0..n).filter(|&j| !fixed[j]).collect();
    let nf = free.len();
    let k = act.len();
    if nf == 0 || k > nf {
        return None;
    }
    let mut lam: Vec<f64> = act.iter().map(|&a| s.lambda[a]).collect();
    let lag_grad = |x: &[f64], lam: &[f64]| -> Vec<f64> {
        let (_, g) = p.obj(x);
        let cs = p.cons_at(x);
        (0..n).map(|j| g[j] + act.iter().enumerate().map(|(r, &a)| lam[r] * cs[a].1[j]).sum::<f64>()).collect()
    };
    let resid = |x: &[f64], lam: &[f64]| -> f64 {
        let lg = lag_grad(x, lam);
        let cs = p.cons_at(x);
        free.iter().map(|&j| lg[j].abs()).fold(0.0, f64::max).max(act.iter().map(|&a| cs[a].0.abs()).fold(0.0, f64::max))
    };
    let r0 = resid(&x, &lam);
    for _ in 0..15 {
        let lg = lag_grad(&x, &lam);
        let cs = p.cons_at(&x);
        let mut jm = DMatrix::zeros(nf + k, nf + k);
        let mut rhs = DVector::zeros(nf + k);
        for (a, &j) in free.iter().enumerate() {
            let h = 1e-6 * (1.0 + x[j].abs());
            let mut xp = x.clone();
            xp[j] += h;
            let mut xm = x.clone();
            xm[j] -= h;
            let gp = lag_grad(&xp, &lam);
            let gm = lag_grad(&xm, &lam);
            for (b, &i) in free.iter().enumerate() {
                jm[(b, a)] = (gp[i] - gm[i]) / (2.0 * h);
            }
            rhs[a] = -lg[j];
        }
        for (r, &ai) in act.iter().enumerate() {
            for (b, &j) in free.iter().enumerate() {
                jm[(nf + r, b)] = cs[ai].1[j];
                jm[(b, nf + r)] = cs[ai].1[j];
            }
            rhs[nf + r] = -cs[ai].0;
        }
        let d = jm.lu().solve(&rhs)?;
        for (b, &j) in free.iter().enumerate() {
            x[j] += d[b];
        }
        for r in 0..k {
            lam[r] += d[nf + r];
        }
        if d.amax() <= 1e-14 * (1.0 + x.iter().fold(0.0f64, |a, b| a.max(b.abs()))) {
            break;
        }
    }
    let r1 = resid(&x, &lam);
    // válido si mejora, respeta cotas, inactivas y signos de los multiplicadores
    let ok = r1.is_finite()
        && r1 <= r0
        && (0..n).all(|j| x[j] >= p.lb[j] - 1e-12 && x[j] <= p.ub[j] + 1e-12)
        && act.iter().zip(&lam).all(|(&a, l)| cs[a].2 || *l >= -1e-9)
        && p.violation(&x) <= s.viol.max(1e-12);
    if !ok {
        return None;
    }
    let (f, _) = p.obj(&x);
    let mut lambda = vec![0.0; cs.len()];
    for (r, &a) in act.iter().enumerate() {
        lambda[a] = lam[r];
    }
    // f está en forma de minimizar; s.f, con su signo original
    if f > p.sign * s.f + 1e-9 * s.f.abs().max(1.0) && s.viol <= 1e-9 {
        return None;
    }
    Some(NlpSol { x: x.clone(), f: p.sign * f, viol: p.violation(&x), lambda, iters: s.iters, converged: true, unbounded: false, history: s.history.clone() })
}

/// Punto inicial por defecto: dentro de las cotas.
pub fn default_start(lb: &[f64], ub: &[f64]) -> Vec<f64> {
    lb.iter()
        .zip(ub)
        .map(|(&l, &u)| match (l.is_finite(), u.is_finite()) {
            (true, true) => 0.5 * (l + u),
            (true, false) => l + 1.0,
            (false, true) => u - 1.0,
            _ => 0.0,
        })
        .collect()
}

/// Mejor solución de varios arranques (el primero desde x0).
pub fn solve(p: &Nlp, x0: &[f64], starts: usize, seed: u64) -> (NlpSol, usize) {
    let n = p.n();
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
    let mut best: Option<NlpSol> = None;
    let mut feasible_runs = 0;
    for k in 0..starts.max(1) {
        let start: Vec<f64> = if k == 0 {
            x0.to_vec()
        } else {
            (0..n)
                .map(|j| {
                    let (l, u) = (p.lb[j], p.ub[j]);
                    if l.is_finite() && u.is_finite() {
                        rng.random_range(l..=u)
                    } else {
                        let c = x0[j];
                        let w = 2.0 * (1.0 + c.abs()) * (k as f64).sqrt();
                        let lo = if l.is_finite() { l.max(c - w) } else { c - w };
                        let hi = if u.is_finite() { u.min(c + w) } else { c + w };
                        if hi > lo { rng.random_range(lo..=hi) } else { lo }
                    }
                })
                .collect()
        };
        let mut s = sqp(p, &start, 300);
        if !s.unbounded {
            if let Some(q) = polish(p, &s) {
                s = q;
            }
        }
        if s.unbounded {
            return (s, feasible_runs);
        }
        let ok = s.viol <= 1e-6;
        if ok {
            feasible_runs += 1;
        }
        let better = match &best {
            None => true,
            Some(b) => {
                let bok = b.viol <= 1e-6;
                (ok && !bok) || (ok == bok && if ok { p.sign * s.f < p.sign * b.f - 1e-9 * b.f.abs().max(1.0) } else { s.viol < b.viol })
            }
        };
        if better {
            best = Some(s);
        }
    }
    (best.unwrap(), feasible_runs)
}

/// Método de penalización exterior: mín f + r·Σ violación², con r creciente.
pub fn penalty_path(p: &Nlp, x0: &[f64]) -> Vec<(f64, Vec<f64>, f64, f64)> {
    let mut out = vec![];
    let mut x = x0.to_vec();
    let mut r = 1.0;
    for _ in 0..10 {
        // subproblema sin restricciones (solo cotas), por SQP con la función penalizada
        let pen = PenaltyNlp { base: p, r };
        let s = pen.minimize(&x);
        x = s;
        let v = p.violation(&x);
        out.push((r, x.clone(), p.value(&x), v));
        if v <= 1e-6 {
            break;
        }
        r *= 10.0;
    }
    out
}

struct PenaltyNlp<'a> {
    base: &'a Nlp,
    r: f64,
}

impl PenaltyNlp<'_> {
    fn val_grad(&self, x: &[f64]) -> (f64, Vec<f64>) {
        let (f, mut g) = self.base.obj(x);
        let mut v = f;
        for (c, gc, eq) in self.base.cons_at(x) {
            let e = if eq { c } else { c.max(0.0) };
            if e != 0.0 {
                v += self.r * e * e;
                for j in 0..g.len() {
                    g[j] += 2.0 * self.r * e * gc[j];
                }
            }
        }
        (v, g)
    }
    /// BFGS con proyección sobre las cotas.
    fn minimize(&self, x0: &[f64]) -> Vec<f64> {
        let n = x0.len();
        let mut x = x0.to_vec();
        self.base.clamp(&mut x);
        let mut h = vec![vec![0.0; n]; n];
        for (i, r) in h.iter_mut().enumerate() {
            r[i] = 1.0;
        }
        let (mut f, mut g) = self.val_grad(&x);
        for _ in 0..500 {
            let d: Vec<f64> = (0..n).map(|i| -(0..n).map(|j| h[i][j] * g[j]).sum::<f64>()).collect();
            let mut a = 1.0;
            let mut xn = x.clone();
            let mut fnew = f;
            let slope: f64 = g.iter().zip(&d).map(|(p, q)| p * q).sum();
            let mut ok = false;
            for _ in 0..50 {
                for j in 0..n {
                    xn[j] = x[j] + a * d[j];
                }
                self.base.clamp(&mut xn);
                fnew = self.val_grad(&xn).0;
                if fnew <= f + 1e-4 * a * slope.min(0.0) {
                    ok = true;
                    break;
                }
                a *= 0.5;
            }
            if !ok || (f - fnew).abs() <= 1e-15 * f.abs().max(1.0) {
                break;
            }
            let (_, gn) = self.val_grad(&xn);
            let s: Vec<f64> = (0..n).map(|j| xn[j] - x[j]).collect();
            let y: Vec<f64> = (0..n).map(|j| gn[j] - g[j]).collect();
            let sy: f64 = s.iter().zip(&y).map(|(p, q)| p * q).sum();
            if sy > 1e-14 {
                let hy: Vec<f64> = (0..n).map(|i| (0..n).map(|j| h[i][j] * y[j]).sum()).collect();
                let yhy: f64 = y.iter().zip(&hy).map(|(p, q)| p * q).sum();
                for i in 0..n {
                    for j in 0..n {
                        h[i][j] += (sy + yhy) * s[i] * s[j] / (sy * sy) - (hy[i] * s[j] + s[i] * hy[j]) / sy;
                    }
                }
            }
            x = xn;
            f = fnew;
            g = gn;
            if norm_inf(&g) <= 1e-10 {
                break;
            }
        }
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iaqsb_core::expr::Vars;

    fn nlp(f: &str, cons: &[(&str, Dir)], lb: Vec<f64>, ub: Vec<f64>, names: &[&str], sign: f64) -> Nlp {
        let mut v = Vars::fixed(names);
        Nlp {
            f: Expr::parse(f, &mut v).unwrap(),
            cons: cons.iter().map(|(e, d)| (Expr::parse(e, &mut v).unwrap(), *d)).collect(),
            lb,
            ub,
            sign,
        }
    }

    #[test]
    fn rosenbrock_y_restringido() {
        let p = nlp("100*(Y - X^2)^2 + (1 - X)^2", &[], vec![f64::NEG_INFINITY; 2], vec![f64::INFINITY; 2], &["X", "Y"], 1.0);
        let (s, _) = solve(&p, &[-1.2, 1.0], 1, 1);
        assert!((s.x[0] - 1.0).abs() < 1e-6 && (s.x[1] - 1.0).abs() < 1e-6, "{:?}", s.x);
        // máx x·y con x + y ≤ 10 (en forma c ≤ 0): (5, 5), λ = 5
        let p = nlp("X*Y", &[("X + Y - 10", Dir::Le)], vec![0.0; 2], vec![f64::INFINITY; 2], &["X", "Y"], -1.0);
        let (s, _) = solve(&p, &[1.0, 1.0], 3, 1);
        assert!((s.x[0] - 5.0).abs() < 1e-6 && (s.f - 25.0).abs() < 1e-6, "{:?}", s);
        assert!((s.lambda[0] - 5.0).abs() < 1e-4, "{:?}", s.lambda);
        // círculo: mín x + y con x² + y² = 2 → (−1, −1)
        let p = nlp("X + Y", &[("X^2 + Y^2 - 2", Dir::Eq)], vec![f64::NEG_INFINITY; 2], vec![f64::INFINITY; 2], &["X", "Y"], 1.0);
        let (s, _) = solve(&p, &[0.5, 0.2], 4, 1);
        assert!((s.x[0] + 1.0).abs() < 1e-6 && (s.x[1] + 1.0).abs() < 1e-6, "{:?}", s.x);
        let path = penalty_path(&p, &[0.5, 0.2]);
        assert!(path.last().unwrap().3 < 1e-3);
    }
}

