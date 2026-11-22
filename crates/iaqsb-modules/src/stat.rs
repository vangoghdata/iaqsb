//! Funciones de distribución, cuantiles y funciones de pérdida de las
//! distribuciones de WinQSB (las usan inventarios, colas y control de calidad).

use crate::pert::dist::{Dist, Law};
use statrs::distribution::{Beta, ContinuousCDF, Discrete, Gamma, Hypergeometric, Laplace, LogNormal, Normal, Pareto, Triangular, Weibull};

/// Valores y probabilidades de una distribución discreta (soporte truncado
/// donde la cola es despreciable). None si es continua.
pub fn support(l: &Law) -> Option<Vec<(f64, f64)>> {
    let [a, b, c] = l.p;
    let tail = |f: &dyn Fn(u64) -> f64, mean: f64, sd: f64| -> Vec<(f64, f64)> {
        let hi = (mean + 40.0 * sd + 50.0).ceil() as u64;
        let mut v = vec![];
        let mut acc = 0.0;
        for k in 0..=hi.min(5_000_000) {
            let p = f(k);
            if p > 0.0 {
                v.push((k as f64, p));
            }
            acc += p;
            if acc > 1.0 - 1e-14 && k as f64 > mean {
                break;
            }
        }
        v
    };
    Some(match l.dist {
        Dist::Constant => vec![(a, 1.0)],
        Dist::Discrete => l.values.clone(),
        Dist::Binomial => {
            let d = statrs::distribution::Binomial::new(b, a as u64).ok()?;
            (0..=a as u64).map(|k| (k as f64, d.pmf(k))).collect()
        }
        Dist::Poisson => {
            let d = statrs::distribution::Poisson::new(a).ok()?;
            tail(&|k| d.pmf(k), a, a.sqrt())
        }
        Dist::Geometric => tail(&|k| a * (1.0 - a).powi(k as i32), l.mean(), l.variance().sqrt()),
        Dist::Hypergeometric => {
            let d = Hypergeometric::new(a as u64, b as u64, c as u64).ok()?;
            (0..=c as u64).map(|k| (k as f64, d.pmf(k))).filter(|x| x.1 > 0.0).collect()
        }
        _ => return None,
    })
}

/// F(x) = P(X ≤ x).
pub fn cdf(l: &Law, x: f64) -> f64 {
    if let Some(s) = support(l) {
        return s.iter().filter(|(v, _)| *v <= x + 1e-12).map(|(_, p)| p).sum::<f64>().min(1.0);
    }
    let [a, b, c] = l.p;
    let f = match l.dist {
        Dist::Normal | Dist::Arbitrary => {
            if b <= 0.0 {
                return if x >= a { 1.0 } else { 0.0 };
            }
            Normal::new(a, b).map(|d| d.cdf(x)).unwrap_or(f64::NAN)
        }
        Dist::Uniform => {
            if b <= a {
                return if x >= a { 1.0 } else { 0.0 };
            }
            ((x - a) / (b - a)).clamp(0.0, 1.0)
        }
        Dist::Triangular => Triangular::new(a, c, b).map(|d| d.cdf(x)).unwrap_or(f64::NAN),
        Dist::Exponential => {
            if x <= a {
                0.0
            } else {
                1.0 - (-(x - a) / b).exp()
            }
        }
        Dist::Beta => Beta::new(b, c).map(|d| d.cdf((x / a).clamp(0.0, 1.0))).unwrap_or(f64::NAN),
        Dist::Gamma | Dist::Erlang => {
            if x <= a {
                0.0
            } else {
                Gamma::new(c, 1.0 / b).map(|d| d.cdf(x - a)).unwrap_or(f64::NAN)
            }
        }
        Dist::Weibull => {
            if x <= a {
                0.0
            } else {
                Weibull::new(c, b).map(|d| d.cdf(x - a)).unwrap_or(f64::NAN)
            }
        }
        Dist::Lognormal => {
            if x <= 0.0 {
                0.0
            } else {
                LogNormal::new(a, b).map(|d| d.cdf(x)).unwrap_or(f64::NAN)
            }
        }
        Dist::Laplace => Laplace::new(a, b).map(|d| d.cdf(x)).unwrap_or(f64::NAN),
        Dist::Pareto => {
            if x <= a {
                0.0
            } else {
                Pareto::new(a, b).map(|d| d.cdf(x)).unwrap_or(f64::NAN)
            }
        }
        Dist::PowerFunction => {
            if x <= a {
                0.0
            } else {
                ((x - a) / b).clamp(0.0, 1.0).powf(c)
            }
        }
        Dist::ThreeTime => {
            if c <= a {
                return if x >= a { 1.0 } else { 0.0 };
            }
            let al = 1.0 + 4.0 * (b - a) / (c - a);
            let be = 1.0 + 4.0 * (c - b) / (c - a);
            Beta::new(al, be).map(|d| d.cdf(((x - a) / (c - a)).clamp(0.0, 1.0))).unwrap_or(f64::NAN)
        }
        _ => f64::NAN,
    };
    f.clamp(0.0, 1.0)
}

/// Límites prácticos del soporte (cuantiles 1e-12 y 1 − 1e-12).
fn bounds(l: &Law) -> (f64, f64) {
    let m = l.mean();
    let sd = l.variance().sqrt().max(1e-9);
    let mut lo = if m.is_finite() { m - 10.0 * sd } else { 0.0 };
    let mut hi = if m.is_finite() { m + 10.0 * sd } else { 1.0 };
    let mut k = 0;
    while cdf(l, lo) > 1e-12 && k < 200 {
        lo -= (hi - lo).max(1.0);
        k += 1;
    }
    k = 0;
    while cdf(l, hi) < 1.0 - 1e-12 && k < 200 {
        hi += (hi - lo).max(1.0);
        k += 1;
    }
    (lo, hi)
}

/// Cuantil: el menor x con F(x) ≥ p.
pub fn quantile(l: &Law, p: f64) -> f64 {
    let p = p.clamp(0.0, 1.0);
    if let Some(s) = support(l) {
        let mut acc = 0.0;
        for (v, q) in &s {
            acc += q;
            if acc >= p - 1e-12 {
                return *v;
            }
        }
        return s.last().map(|x| x.0).unwrap_or(0.0);
    }
    if matches!(l.dist, Dist::Normal | Dist::Arbitrary) && l.p[1] > 0.0 && p > 0.0 && p < 1.0 {
        return Normal::new(l.p[0], l.p[1]).map(|d| d.inverse_cdf(p)).unwrap_or(f64::NAN);
    }
    let (mut lo, mut hi) = bounds(l);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if cdf(l, mid) >= p {
            hi = mid;
        } else {
            lo = mid;
        }
        if hi - lo <= 1e-12 * (1.0 + hi.abs()) {
            break;
        }
    }
    hi
}

/// Función de pérdida E[(X − s)⁺] (unidades que faltan si se tienen s).
pub fn loss(l: &Law, s: f64) -> f64 {
    if let Some(sup) = support(l) {
        return sup.iter().map(|(v, p)| p * (v - s).max(0.0)).sum();
    }
    if matches!(l.dist, Dist::Normal | Dist::Arbitrary) {
        let (m, sd) = (l.p[0], l.p[1]);
        if sd <= 0.0 {
            return (m - s).max(0.0);
        }
        return sd * unit_normal_loss((s - m) / sd);
    }
    // ∫ₛ^∞ (1 − F(x)) dx con Simpson adaptativo en [max(s, lo), hi]
    let (lo, mut hi) = bounds(l);
    // recorta la cola donde F ya vale 1 (soporte acotado)
    let mut a0 = lo;
    for _ in 0..200 {
        let mid = 0.5 * (a0 + hi);
        if cdf(l, mid) >= 1.0 - 1e-15 {
            hi = mid;
        } else {
            a0 = mid;
        }
    }
    if s >= hi {
        return 0.0;
    }
    let start = s.max(lo);
    let extra = if s < lo { lo - s } else { 0.0 };
    let g = |x: f64| 1.0 - cdf(l, x);
    fn simpson(g: &dyn Fn(f64) -> f64, a: f64, b: f64, fa: f64, fm: f64, fb: f64, whole: f64, eps: f64, depth: u32) -> f64 {
        let m = 0.5 * (a + b);
        let (lm, rm) = (0.5 * (a + m), 0.5 * (m + b));
        let (flm, frm) = (g(lm), g(rm));
        let left = (m - a) / 6.0 * (fa + 4.0 * flm + fm);
        let right = (b - m) / 6.0 * (fm + 4.0 * frm + fb);
        if depth == 0 || (left + right - whole).abs() <= 15.0 * eps {
            return left + right + (left + right - whole) / 15.0;
        }
        simpson(g, a, m, fa, flm, fm, left, eps / 2.0, depth - 1) + simpson(g, m, b, fm, frm, fb, right, eps / 2.0, depth - 1)
    }
    // en trozos para no saltarse detalles
    let pieces = 64;
    let w = (hi - start) / pieces as f64;
    let mut acc = 0.0;
    for k in 0..pieces {
        let (a, b) = (start + k as f64 * w, start + (k + 1) as f64 * w);
        let (fa, fb, fm) = (g(a), g(b), g(0.5 * (a + b)));
        let whole = (b - a) / 6.0 * (fa + 4.0 * fm + fb);
        acc += simpson(&g, a, b, fa, fm, fb, whole, 1e-11 * (1.0 + l.mean().abs()) / pieces as f64, 30);
    }
    extra + acc
}

/// Pérdida normal estándar G(z) = φ(z) − z·(1 − Φ(z)).
pub fn unit_normal_loss(z: f64) -> f64 {
    let pdf = (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt();
    pdf - z * (1.0 - crate::pert::dist::phi(z))
}

/// Normal con media y desviación dadas, como `Law`.
pub fn normal_law(mean: f64, sd: f64) -> Law {
    Law { dist: Dist::Normal, p: [mean, sd.max(0.0), 0.0], values: vec![] }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn law(d: Dist, p: [f64; 3]) -> Law {
        Law::new(d, [Some(p[0]), Some(p[1]), Some(p[2])], None, "x").unwrap()
    }

    #[test]
    fn cuantil_y_perdida() {
        for (d, p) in [
            (Dist::Normal, [100.0, 20.0, 0.0]),
            (Dist::Uniform, [10.0, 30.0, 0.0]),
            (Dist::Exponential, [0.0, 5.0, 0.0]),
            (Dist::Gamma, [0.0, 2.0, 3.0]),
            (Dist::Poisson, [7.0, 0.0, 0.0]),
            (Dist::Triangular, [1.0, 3.0, 8.0]),
            (Dist::Weibull, [0.0, 4.0, 2.0]),
        ] {
            let l = law(d, p);
            for q in [0.1, 0.5, 0.9] {
                let x = quantile(&l, q);
                assert!(cdf(&l, x) >= q - 1e-9, "{d:?} {q}");
            }
            // E[(X − s)⁺] con s muy bajo es la media menos s
            let lo = -5.0;
            assert!((loss(&l, lo) - (l.mean() - lo)).abs() < 1e-3 * l.mean().max(1.0), "{d:?}: {} vs {}", loss(&l, lo), l.mean() - lo);
        }
        let u = law(Dist::Uniform, [0.0, 10.0, 0.0]);
        // E[(U − 6)⁺] = ∫₆¹⁰ (x − 6)/10 dx = 0.8
        assert!((loss(&u, 6.0) - 0.8).abs() < 1e-6);
    }
}
