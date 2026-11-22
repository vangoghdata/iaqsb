//! Constantes de los gráficos de control para muestras normales de tamaño n,
//! calculadas por integración numérica (no por tablas): d2 y d3 (media y
//! desviación del rango relativo), d4 (mediana del rango relativo), c4 y c5
//! (de la desviación estándar), e3 (desviación del rango medio) y e4
//! (desviación de la mediana).

use statrs::function::gamma::ln_gamma;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone, Copy, Debug)]
pub struct Consts {
    pub d2: f64,
    pub d3: f64,
    pub d4: f64,
    pub c4: f64,
    pub c5: f64,
    pub e3: f64,
    pub e4: f64,
}

static CACHE: Mutex<Option<HashMap<usize, Consts>>> = Mutex::new(None);

const H: f64 = 0.01;
const LO: f64 = -8.5;
const NX: usize = 1701;

fn grids() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let xs: Vec<f64> = (0..NX).map(|i| LO + i as f64 * H).collect();
    let phi: Vec<f64> = xs.iter().map(|x| (-x * x / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt()).collect();
    let cdf: Vec<f64> = xs.iter().map(|x| 0.5 * libm_erfc(-x / std::f64::consts::SQRT_2)).collect();
    (xs, phi, cdf)
}

fn libm_erfc(x: f64) -> f64 {
    statrs::function::erf::erfc(x)
}

fn trap(v: &[f64], h: f64) -> f64 {
    if v.len() < 2 {
        return 0.0;
    }
    h * (v.iter().sum::<f64>() - 0.5 * (v[0] + v[v.len() - 1]))
}

fn compute(n: usize) -> Consts {
    let n = n.max(2);
    let nf = n as f64;
    let c4 = ((2.0 / (nf - 1.0)).ln() / 2.0 + ln_gamma(nf / 2.0) - ln_gamma((nf - 1.0) / 2.0)).exp();
    let c5 = (1.0 - c4 * c4).max(0.0).sqrt();
    let (xs, phi, cdf) = grids();
    // rango: F_W(w) = n ∫ φ(x)[Φ(x + w) − Φ(x)]^{n−1} dx
    let nw = 1201; // w de 0 a 12
    let mut surv = vec![0.0; nw];
    for (k, s) in surv.iter_mut().enumerate() {
        let f: Vec<f64> = (0..NX)
            .map(|i| {
                let j = i + k;
                let up = if j < NX { cdf[j] } else { 1.0 };
                phi[i] * (up - cdf[i]).max(0.0).powi(n as i32 - 1)
            })
            .collect();
        *s = 1.0 - (nf * trap(&f, H)).min(1.0);
    }
    let ws: Vec<f64> = (0..nw).map(|k| k as f64 * H).collect();
    let d2 = trap(&surv, H);
    let m2 = trap(&surv.iter().zip(&ws).map(|(s, w)| 2.0 * w * s).collect::<Vec<_>>(), H);
    let d3 = (m2 - d2 * d2).max(0.0).sqrt();
    let d4 = {
        let k = surv.iter().position(|s| *s <= 0.5).unwrap_or(nw - 1).max(1);
        let (s0, s1) = (surv[k - 1], surv[k]);
        ws[k - 1] + H * (s0 - 0.5) / (s0 - s1).max(1e-300)
    };
    // estadísticos de orden
    let lnf = |k: f64| ln_gamma(k + 1.0);
    let dens = |k: usize| -> Vec<f64> {
        let kf = k as f64;
        let c = lnf(nf) - lnf(kf - 1.0) - lnf(nf - kf);
        (0..NX)
            .map(|i| {
                let (p, q) = (cdf[i], 1.0 - cdf[i]);
                if phi[i] <= 0.0 {
                    return 0.0;
                }
                let lp = if k > 1 { (kf - 1.0) * p.max(1e-300).ln() } else { 0.0 };
                let lq = if k < n { (nf - kf) * q.max(1e-300).ln() } else { 0.0 };
                (c + lp + lq + phi[i].ln()).exp()
            })
            .collect()
    };
    let second = |k: usize| trap(&dens(k).iter().zip(&xs).map(|(f, x)| f * x * x).collect::<Vec<_>>(), H);
    // E[X(i)·X(j)] con i < j
    let cross = |i: usize, j: usize| -> f64 {
        let (fi, fj) = (i as f64, j as f64);
        let c = lnf(nf) - lnf(fi - 1.0) - lnf(fj - fi - 1.0) - lnf(nf - fj);
        let step = 2; // rejilla más gruesa para la integral doble
        let idx: Vec<usize> = (0..NX).step_by(step).collect();
        let hh = H * step as f64;
        let mut tot = 0.0;
        for (a, &ia) in idx.iter().enumerate() {
            let (x, px) = (xs[ia], cdf[ia]);
            if phi[ia] < 1e-300 {
                continue;
            }
            let mut row = 0.0;
            for &ib in idx.iter().skip(a + 1) {
                let (y, py) = (xs[ib], cdf[ib]);
                let mid = (py - px).max(0.0);
                if phi[ib] < 1e-300 || (j - i > 1 && mid <= 0.0) {
                    continue;
                }
                let l = c + if i > 1 { (fi - 1.0) * px.max(1e-300).ln() } else { 0.0 } + if j - i > 1 { (fj - fi - 1.0) * mid.ln() } else { 0.0 } + if j < n { (nf - fj) * (1.0 - py).max(1e-300).ln() } else { 0.0 } + phi[ia].ln() + phi[ib].ln();
                row += x * y * l.exp();
            }
            tot += row;
        }
        tot * hh * hh
    };
    let e4 = if n % 2 == 1 {
        second(n.div_ceil(2)).sqrt()
    } else {
        let k = n / 2;
        ((second(k) + second(k + 1) + 2.0 * cross(k, k + 1)) / 4.0).max(0.0).sqrt()
    };
    let e3 = ((second(1) + second(n) + 2.0 * cross(1, n)) / 4.0).max(0.0).sqrt();
    Consts { d2, d3, d4, c4, c5, e3, e4 }
}

pub fn consts(n: usize) -> Consts {
    let n = n.clamp(2, 200);
    {
        let g = CACHE.lock().unwrap();
        if let Some(c) = g.as_ref().and_then(|m| m.get(&n)) {
            return *c;
        }
    }
    let c = compute(n);
    let mut g = CACHE.lock().unwrap();
    g.get_or_insert_with(HashMap::new).insert(n, c);
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tablas() {
        // valores de las tablas clásicas (Montgomery, apéndice VI)
        for (n, d2, d3, c4) in [(2, 1.128, 0.853, 0.7979), (5, 2.326, 0.864, 0.9400), (10, 3.078, 0.797, 0.9727), (25, 3.931, 0.708, 0.9896)] {
            let k = consts(n);
            assert!((k.d2 - d2).abs() < 2e-3, "d2({n}) = {}", k.d2);
            assert!((k.d3 - d3).abs() < 2e-3, "d3({n}) = {}", k.d3);
            assert!((k.c4 - c4).abs() < 1e-4, "c4({n}) = {}", k.c4);
        }
        // mediana de 3: desviación 0.670σ; rango medio de 2 = media: 1/√2
        assert!((consts(3).e4 - 0.6698).abs() < 2e-3, "{}", consts(3).e4);
        assert!((consts(2).e3 - std::f64::consts::FRAC_1_SQRT_2).abs() < 2e-3, "{}", consts(2).e3);
        assert!((consts(2).e4 - std::f64::consts::FRAC_1_SQRT_2).abs() < 2e-3);
        // mediana del rango para n = 2: √2·Φ⁻¹(0.75) = 0.954
        assert!((consts(2).d4 - 0.9539).abs() < 2e-3, "{}", consts(2).d4);
    }
}
