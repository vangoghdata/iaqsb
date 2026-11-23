//! Suavización exponencial ETS: traducción a Rust de `ets()` del paquete forecast de R
//! (Hyndman y Khandakar, 2008), la que usan `ses()`, `holt()` y `hw()`.
//!
//! Se traducen paso a paso las piezas de forecast 9 y de R para dar los mismos números
//! que R sin tener R:
//! - `initstate()` (estados iniciales: `decompose()` o la regresión con Fourier),
//! - `initparam()` (parámetros iniciales), `check.param()` y `admissible()`,
//! - `etscalc()` (el filtro en C) y el objetivo (`lik`, `mse`, `amse`, `sigma`, `mae`),
//! - `nmmin()`, el Nelder-Mead de R (src/appl/optim.c) con los valores de `etsmodel()`,
//! - la selección por AICc entre modelos (ZZZ), y
//! - `forecast.ets()`: medias e intervalos por las fórmulas de las clases 1, 2 y 3.

use statrs::distribution::{ContinuousCDF, Normal};

const TOL: f64 = 1.0e-10;
const HUGEN: f64 = 1.0e10;
const NA: f64 = -99999.0;
const BIG: f64 = 1.0e35;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comp {
    N,
    A,
    M,
}

impl Comp {
    fn letra(self) -> char {
        match self {
            Comp::N => 'N',
            Comp::A => 'A',
            Comp::M => 'M',
        }
    }
    fn desde(c: char) -> Option<Comp> {
        match c.to_ascii_uppercase() {
            'N' => Some(Comp::N),
            'A' => Some(Comp::A),
            'M' => Some(Comp::M),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Criterio {
    Lik,
    Mse,
    Amse,
    Sigma,
    Mae,
}

#[derive(Debug, Clone)]
pub struct EtsFit {
    pub error: Comp,
    pub tendencia: Comp,
    pub estacion: Comp,
    pub amortiguada: bool,
    pub m: usize,
    pub alpha: f64,
    pub beta: f64,
    pub gamma: f64,
    pub phi: f64,
    /// Parámetros como `fit$par` de R: alpha, beta, gamma, phi, l, b, s0, s1…
    pub par: Vec<(String, f64)>,
    pub ajustados: Vec<f64>,
    pub residuos: Vec<f64>,
    /// Estado después del último dato (l, b, s1…sm).
    pub ultimo: Vec<f64>,
    pub lik: f64,
    pub aic: f64,
    pub aicc: f64,
    pub bic: f64,
    pub sigma2: f64,
    pub mse: f64,
}

impl EtsFit {
    /// "ETS(A,Ad,N)", como `fit$method` de R.
    pub fn metodo(&self) -> String {
        format!("ETS({},{}{},{})", self.error.letra(), self.tendencia.letra(), if self.amortiguada { "d" } else { "" }, self.estacion.letra())
    }
}

// ------------------------------------------------------------------ etscalc (C de forecast)

fn pronostico_pasos(l: f64, b: f64, s: &[f64], m: usize, tend: Comp, est: Comp, phi: f64, f: &mut [f64]) {
    let h = f.len();
    let mut phistar = phi;
    for i in 0..h {
        f[i] = match tend {
            Comp::N => l,
            Comp::A => l + phistar * b,
            Comp::M => {
                if b < 0.0 {
                    NA
                } else {
                    l * b.powf(phistar)
                }
            }
        };
        let mut j = m as i64 - 1 - i as i64;
        while j < 0 {
            j += m as i64;
        }
        let j = j as usize;
        match est {
            Comp::A => f[i] += s[j],
            Comp::M => f[i] *= s[j],
            Comp::N => {}
        }
        if i + 1 < h {
            if (phi - 1.0).abs() < TOL {
                phistar += 1.0;
            } else {
                phistar += phi.powi(i as i32 + 1);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn actualizar(oldl: f64, l: &mut f64, oldb: f64, b: &mut f64, olds: &[f64], s: &mut [f64], m: usize, tend: Comp, est: Comp, alpha: f64, beta: f64, gamma: f64, phi: f64, y: f64) {
    let (q, phib) = match tend {
        Comp::N => (oldl, 0.0),
        Comp::A => {
            let pb = phi * oldb;
            (oldl + pb, pb)
        }
        Comp::M => {
            if (phi - 1.0).abs() < TOL {
                (oldl * oldb, oldb)
            } else {
                let pb = oldb.powf(phi);
                (oldl * pb, pb)
            }
        }
    };
    let p = match est {
        Comp::N => y,
        Comp::A => y - olds[m - 1],
        Comp::M => {
            if olds[m - 1].abs() < TOL {
                HUGEN
            } else {
                y / olds[m - 1]
            }
        }
    };
    *l = q + alpha * (p - q);
    if tend != Comp::N {
        let r = if tend == Comp::A {
            *l - oldl
        } else if oldl.abs() < TOL {
            HUGEN
        } else {
            *l / oldl
        };
        *b = phib + (beta / alpha) * (r - phib);
    }
    if est != Comp::N {
        let t = if est == Comp::A {
            y - q
        } else if q.abs() < TOL {
            HUGEN
        } else {
            y / q
        };
        s[0] = olds[m - 1] + gamma * (t - olds[m - 1]);
        for j in 1..m {
            s[j] = olds[j - 1];
        }
    }
}

struct Calculo {
    e: Vec<f64>,
    ajustados: Vec<f64>,
    lik: f64,
    amse: Vec<f64>,
    estados: Vec<f64>,
}

#[allow(clippy::too_many_arguments)]
fn etscalc(y: &[f64], x0: &[f64], m: usize, err: Comp, tend: Comp, est: Comp, alpha: f64, beta: f64, gamma: f64, phi: f64, nmse: usize) -> Calculo {
    let n = y.len();
    let m = if est == Comp::N { 1 } else { m.max(1) };
    let nmse = nmse.min(30);
    let nest = m * (est != Comp::N) as usize + 1 + (tend != Comp::N) as usize;
    let mut x = vec![0.0; nest * (n + 1)];
    x[..nest.min(x0.len())].copy_from_slice(&x0[..nest.min(x0.len())]);
    let mut l = x[0];
    let mut b = if tend != Comp::N { x[1] } else { 0.0 };
    let mut s = vec![0.0; m.max(1)];
    if est != Comp::N {
        for j in 0..m {
            s[j] = x[(tend != Comp::N) as usize + j + 1];
        }
    }
    let mut lik = 0.0;
    let mut lik2 = 0.0;
    let mut amse = vec![0.0; nmse];
    let mut denom = vec![0.0; nmse];
    let mut e = vec![0.0; n];
    let mut aj = vec![0.0; n];
    let mut f = vec![0.0; nmse.max(1)];
    let mut olds = vec![0.0; m.max(1)];
    for i in 0..n {
        let oldl = l;
        let oldb = b;
        olds.copy_from_slice(&s);
        pronostico_pasos(oldl, oldb, &olds, m, tend, est, phi, &mut f);
        if (f[0] - NA).abs() < TOL {
            return Calculo { e, ajustados: aj, lik: NA, amse, estados: x };
        }
        aj[i] = f[0];
        e[i] = if err == Comp::A { y[i] - f[0] } else { (y[i] - f[0]) / f[0] };
        for j in 0..nmse {
            if i + j < n {
                denom[j] += 1.0;
                let tmp = y[i + j] - f[j];
                amse[j] = (amse[j] * (denom[j] - 1.0) + tmp * tmp) / denom[j];
            }
        }
        actualizar(oldl, &mut l, oldb, &mut b, &olds, &mut s, m, tend, est, alpha, beta, gamma, phi, y[i]);
        x[nest * (i + 1)] = l;
        if tend != Comp::N {
            x[nest * (i + 1) + 1] = b;
        }
        if est != Comp::N {
            for j in 0..m {
                x[(tend != Comp::N) as usize + nest * (i + 1) + j + 1] = s[j];
            }
        }
        lik += e[i] * e[i];
        lik2 += f[0].abs().ln();
    }
    lik = n as f64 * lik.ln();
    if err == Comp::M {
        lik += 2.0 * lik2;
    }
    Calculo { e, ajustados: aj, lik, amse, estados: x }
}

// ------------------------------------------------------------------ raíces (admissible)

/// Máximo módulo de las raíces del polinomio mónico con coeficientes en potencias
/// crecientes (como polyroot de R), por el método de Aberth-Ehrlich.
fn max_modulo_raices(c: &[f64]) -> f64 {
    let n = c.len() - 1;
    if n == 0 {
        return 0.0;
    }
    #[derive(Clone, Copy)]
    struct Z(f64, f64);
    let add = |a: Z, b: Z| Z(a.0 + b.0, a.1 + b.1);
    let sub = |a: Z, b: Z| Z(a.0 - b.0, a.1 - b.1);
    let mul = |a: Z, b: Z| Z(a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0);
    let div = |a: Z, b: Z| {
        let d = b.0 * b.0 + b.1 * b.1;
        Z((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d)
    };
    let lead = c[n];
    let a: Vec<f64> = c.iter().map(|x| x / lead).collect();
    let eval = |z: Z| -> (Z, Z) {
        // p(z) y p'(z) por Horner
        let mut p = Z(a[n], 0.0);
        let mut d = Z(0.0, 0.0);
        for k in (0..n).rev() {
            d = add(mul(d, z), p);
            p = add(mul(p, z), Z(a[k], 0.0));
        }
        (p, d)
    };
    let cota = 1.0 + a[..n].iter().fold(0.0f64, |m, x| m.max(x.abs()));
    let mut z: Vec<Z> = (0..n).map(|k| {
        let th = 2.0 * std::f64::consts::PI * k as f64 / n as f64 + 0.4;
        Z(0.5 * cota * th.cos(), 0.5 * cota * th.sin())
    }).collect();
    for _ in 0..500 {
        let mut cambio = 0.0f64;
        for i in 0..n {
            let (p, d) = eval(z[i]);
            if p.0 == 0.0 && p.1 == 0.0 {
                continue;
            }
            let ratio = div(p, d);
            let mut s = Z(0.0, 0.0);
            for j in 0..n {
                if j != i {
                    let dz = sub(z[i], z[j]);
                    s = add(s, div(Z(1.0, 0.0), dz));
                }
            }
            let w = div(ratio, sub(Z(1.0, 0.0), mul(ratio, s)));
            if w.0.is_finite() && w.1.is_finite() {
                z[i] = sub(z[i], w);
                cambio = cambio.max((w.0 * w.0 + w.1 * w.1).sqrt());
            }
        }
        if cambio < 1e-15 {
            break;
        }
    }
    z.iter().map(|w| (w.0 * w.0 + w.1 * w.1).sqrt()).fold(0.0, f64::max)
}

// ------------------------------------------------------------------ objetivo

struct Objetivo<'a> {
    y: &'a [f64],
    nest: usize,
    err: Comp,
    tend: Comp,
    est: Comp,
    amortiguada: bool,
    lower: [f64; 4],
    upper: [f64; 4],
    crit: Criterio,
    nmse: usize,
    m: usize,
}

impl Objetivo<'_> {
    fn admisible(&self, alpha: f64, beta: f64, gamma: f64, phi: f64) -> bool {
        if !(0.0..=1.0 + 1e-8).contains(&phi) {
            return false;
        }
        if self.est == Comp::N {
            if alpha < 1.0 - 1.0 / phi || alpha > 1.0 + 1.0 / phi {
                return false;
            }
            if self.tend != Comp::N && (beta < alpha * (phi - 1.0) || beta > (1.0 + phi) * (2.0 - alpha)) {
                return false;
            }
        } else if self.m > 1 {
            let beta = if self.tend == Comp::N { 0.0 } else { beta };
            let m = self.m as f64;
            let d = 1.0 - 1.0 / phi - alpha;
            if gamma < d.max(0.0) || gamma > 1.0 + 1.0 / phi - alpha {
                return false;
            }
            if alpha < 1.0 - 1.0 / phi - gamma * (1.0 - m + phi + phi * m) / (2.0 * phi * m) {
                return false;
            }
            if beta < -(1.0 - phi) * (gamma / m + alpha) {
                return false;
            }
            // P <- c(phi*(1-alpha-gamma), alpha+beta-alpha*phi+gamma-1, rep(alpha+beta-alpha*phi, m-2), alpha+beta-phi, 1)
            let mut p = vec![phi * (1.0 - alpha - gamma), alpha + beta - alpha * phi + gamma - 1.0];
            for _ in 0..self.m.saturating_sub(2) {
                p.push(alpha + beta - alpha * phi);
            }
            p.push(alpha + beta - phi);
            p.push(1.0);
            if max_modulo_raices(&p) > 1.0 + 1e-10 {
                return false;
            }
        }
        true
    }

    fn parametros(&self, par: &[f64]) -> (f64, f64, f64, f64, usize) {
        let mut k = 0;
        let alpha = par[k];
        k += 1;
        let beta = if self.tend != Comp::N {
            k += 1;
            par[k - 1]
        } else {
            0.0
        };
        let gamma = if self.est != Comp::N {
            k += 1;
            par[k - 1]
        } else {
            0.0
        };
        let phi = if self.amortiguada {
            k += 1;
            par[k - 1]
        } else {
            1.0
        };
        (alpha, beta, gamma, phi, k)
    }

    fn valor(&self, par: &[f64]) -> f64 {
        let (alpha, beta, gamma, phi, _) = self.parametros(par);
        // check.param con bounds = "both"
        let (lo, up) = (self.lower, self.upper);
        if alpha < lo[0] || alpha > up[0] {
            return f64::INFINITY;
        }
        if self.tend != Comp::N && (beta < lo[1] || beta > alpha || beta > up[1]) {
            return f64::INFINITY;
        }
        if self.amortiguada && (phi < lo[3] || phi > up[3]) {
            return f64::INFINITY;
        }
        if self.est != Comp::N && (gamma < lo[2] || gamma > 1.0 - alpha || gamma > up[2]) {
            return f64::INFINITY;
        }
        if !self.admisible(alpha, beta, gamma, phi) {
            return f64::INFINITY;
        }
        let np = par.len();
        let mut estado: Vec<f64> = par[np - self.nest..].to_vec();
        if self.est != Comp::N {
            let ini = 1 + (self.tend != Comp::N) as usize;
            let suma: f64 = estado[ini..].iter().sum();
            estado.push(self.m as f64 * (self.est == Comp::M) as u8 as f64 - suma);
            if self.est == Comp::M && estado[ini..].iter().cloned().fold(f64::INFINITY, f64::min) < 0.0 {
                return f64::INFINITY;
            }
        }
        let c = etscalc(self.y, &estado, self.m, self.err, self.tend, self.est, alpha, beta, gamma, phi, self.nmse);
        let mut lik = c.lik;
        if lik < -1e10 {
            lik = -1e10;
        }
        if lik.is_nan() || (lik + 99999.0).abs() < 1e-7 {
            lik = f64::INFINITY;
        }
        match self.crit {
            Criterio::Lik => lik,
            Criterio::Mse => c.amse[0],
            Criterio::Amse => c.amse.iter().sum::<f64>() / c.amse.len() as f64,
            Criterio::Sigma => c.e.iter().map(|x| x * x).sum::<f64>() / c.e.len() as f64,
            Criterio::Mae => c.e.iter().map(|x| x.abs()).sum::<f64>() / c.e.len() as f64,
        }
    }
}

// ------------------------------------------------------------------ nmmin (Nelder-Mead de R)

/// Traducción de `nmmin()` de R (src/appl/optim.c).
#[allow(clippy::too_many_arguments)]
fn nmmin(bvec: &mut [f64], f_obj: &dyn Fn(&[f64]) -> f64, abstol: f64, intol: f64, alpha: f64, bet: f64, gamm: f64, maxit: usize) -> (Vec<f64>, f64) {
    let n = bvec.len();
    let fmin = |b: &[f64]| {
        let v = f_obj(b);
        if v.is_finite() {
            v
        } else {
            BIG
        }
    };
    let mut f = f_obj(bvec);
    if !f.is_finite() {
        return (bvec.to_vec(), f);
    }
    let mut funcount = 1usize;
    let convtol = intol * (f.abs() + intol);
    let n1 = n + 1;
    let c = n + 2;
    // P: (n+1) filas × (n+2) columnas; la fila n guarda el valor de la función
    let mut p = vec![vec![0.0; n + 2]; n + 1];
    p[n1 - 1][0] = f;
    for i in 0..n {
        p[i][0] = bvec[i];
    }
    let mut l = 1usize;
    let mut size = 0.0;
    let mut step = 0.0f64;
    for i in 0..n {
        if 0.1 * bvec[i].abs() > step {
            step = 0.1 * bvec[i].abs();
        }
    }
    if step == 0.0 {
        step = 0.1;
    }
    for j in 2..=n1 {
        for i in 0..n {
            p[i][j - 1] = bvec[i];
        }
        let mut trystep = step;
        while p[j - 2][j - 1] == bvec[j - 2] {
            p[j - 2][j - 1] = bvec[j - 2] + trystep;
            trystep *= 10.0;
        }
        size += trystep;
    }
    let mut oldsize = size;
    let mut calcvert = true;
    loop {
        if calcvert {
            for j in 0..n1 {
                if j + 1 != l {
                    for i in 0..n {
                        bvec[i] = p[i][j];
                    }
                    f = fmin(bvec);
                    funcount += 1;
                    p[n1 - 1][j] = f;
                }
            }
            calcvert = false;
        }
        let mut vl = p[n1 - 1][l - 1];
        let mut vh = vl;
        let mut h = l;
        for j in 1..=n1 {
            if j != l {
                let fj = p[n1 - 1][j - 1];
                if fj < vl {
                    l = j;
                    vl = fj;
                }
                if fj > vh {
                    h = j;
                    vh = fj;
                }
            }
        }
        if vh <= vl + convtol || vl <= abstol {
            break;
        }
        for i in 0..n {
            let mut temp = -p[i][h - 1];
            for j in 0..n1 {
                temp += p[i][j];
            }
            p[i][c - 1] = temp / n as f64;
        }
        for i in 0..n {
            bvec[i] = (1.0 + alpha) * p[i][c - 1] - alpha * p[i][h - 1];
        }
        f = fmin(bvec);
        funcount += 1;
        let vr = f;
        if vr < vl {
            p[n1 - 1][c - 1] = f;
            for i in 0..n {
                let fi = gamm * bvec[i] + (1.0 - gamm) * p[i][c - 1];
                p[i][c - 1] = bvec[i];
                bvec[i] = fi;
            }
            f = fmin(bvec);
            funcount += 1;
            if f < vr {
                for i in 0..n {
                    p[i][h - 1] = bvec[i];
                }
                p[n1 - 1][h - 1] = f;
            } else {
                for i in 0..n {
                    p[i][h - 1] = p[i][c - 1];
                }
                p[n1 - 1][h - 1] = vr;
            }
        } else {
            if vr < vh {
                for i in 0..n {
                    p[i][h - 1] = bvec[i];
                }
                p[n1 - 1][h - 1] = vr;
            }
            for i in 0..n {
                bvec[i] = (1.0 - bet) * p[i][h - 1] + bet * p[i][c - 1];
            }
            f = fmin(bvec);
            funcount += 1;
            if f < p[n1 - 1][h - 1] {
                for i in 0..n {
                    p[i][h - 1] = bvec[i];
                }
                p[n1 - 1][h - 1] = f;
            } else if vr >= vh {
                calcvert = true;
                size = 0.0;
                for j in 0..n1 {
                    if j + 1 != l {
                        for i in 0..n {
                            p[i][j] = bet * (p[i][j] - p[i][l - 1]) + p[i][l - 1];
                            size += (p[i][j] - p[i][l - 1]).abs();
                        }
                    }
                }
                if size < oldsize {
                    oldsize = size;
                } else {
                    break;
                }
            }
        }
        if funcount > maxit {
            break;
        }
    }
    let x: Vec<f64> = (0..n).map(|i| p[i][l - 1]).collect();
    (x, p[n1 - 1][l - 1])
}

// ------------------------------------------------------------------ estados iniciales

/// Mínimos cuadrados con intercepto (lsfit / lm): coeficientes [intercepto, b1, b2…].
fn mco(x: &[Vec<f64>], y: &[f64]) -> Option<Vec<f64>> {
    let n = y.len();
    let k = x.len() + 1;
    // ecuaciones normales con eliminación de Gauss y pivoteo parcial
    let col = |j: usize, i: usize| if j == 0 { 1.0 } else { x[j - 1][i] };
    let mut a = vec![vec![0.0; k + 1]; k];
    for r in 0..k {
        for s in 0..k {
            a[r][s] = (0..n).map(|i| col(r, i) * col(s, i)).sum();
        }
        a[r][k] = (0..n).map(|i| col(r, i) * y[i]).sum();
    }
    for c in 0..k {
        let piv = (c..k).max_by(|&i, &j| a[i][c].abs().partial_cmp(&a[j][c].abs()).unwrap())?;
        if a[piv][c].abs() < 1e-300 {
            return None;
        }
        a.swap(c, piv);
        for r in 0..k {
            if r != c {
                let f = a[r][c] / a[c][c];
                for s in c..=k {
                    a[r][s] -= f * a[c][s];
                }
            }
        }
    }
    Some((0..k).map(|c| a[c][k] / a[c][c]).collect())
}

fn media(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

/// `decompose()` de R: la figura estacional va por posición desde el primer dato.
fn decompose_estacional(y: &[f64], m: usize, mult: bool) -> Vec<f64> {
    let l = y.len();
    let filtro: Vec<f64> = if m % 2 == 0 {
        let mut f = vec![1.0 / m as f64; m + 1];
        f[0] = 0.5 / m as f64;
        f[m] = 0.5 / m as f64;
        f
    } else {
        vec![1.0 / m as f64; m]
    };
    let k = filtro.len();
    let o = k / 2;
    let mut tend = vec![f64::NAN; l];
    for t in o..l.saturating_sub(k - 1 - o) {
        if t + k - 1 - o < l {
            let mut s = 0.0;
            for (j, w) in filtro.iter().enumerate() {
                // filter(sides = 2): y[t + o - j]
                s += w * y[t + o - j];
            }
            tend[t] = s;
        }
    }
    let season: Vec<f64> = (0..l).map(|t| if mult { y[t] / tend[t] } else { y[t] - tend[t] }).collect();
    let mut figura = vec![0.0; m];
    for i in 0..m {
        let v: Vec<f64> = (0..).map(|c| c * m + i).take_while(|&t| t < l).map(|t| season[t]).filter(|x| x.is_finite()).collect();
        figura[i] = if v.is_empty() { f64::NAN } else { media(&v) };
    }
    let mf = media(&figura);
    for f in figura.iter_mut() {
        *f = if mult { *f / mf } else { *f - mf };
    }
    (0..l).map(|t| figura[t % m]).collect()
}

/// `initstate()` de forecast: l0, b0 y s0…s(m−2).
fn estado_inicial(y: &[f64], m: usize, tend: Comp, est: Comp) -> Result<Vec<f64>, String> {
    let n = y.len();
    let mut init_seas = vec![];
    let y_sa: Vec<f64> = if est != Comp::N {
        if n < 4 {
            return Err("muy pocos datos".into());
        }
        let estacional: Vec<f64> = if n < 3 * m {
            // tslm(y ~ trend + fourier(y, 1))
            let t: Vec<f64> = (1..=n).map(|i| i as f64).collect();
            let p = 1.0 / m as f64;
            let mut cols = vec![t.clone()];
            if (2.0 * p - (2.0 * p).round()).abs() > f64::EPSILON {
                cols.push(t.iter().map(|ti| (std::f64::consts::PI * 2.0 * p * ti).sin()).collect());
            }
            cols.push(t.iter().map(|ti| (std::f64::consts::PI * 2.0 * p * ti).cos()).collect());
            let b = mco(&cols, y).ok_or("no se pudo estimar la estacionalidad inicial")?;
            (0..n).map(|i| if est == Comp::A { y[i] - b[0] - b[1] * (i + 1) as f64 } else { y[i] / (b[0] + b[1] * (i + 1) as f64) }).collect()
        } else {
            decompose_estacional(y, m, est == Comp::M)
        };
        init_seas = estacional[1..m].iter().rev().copied().collect();
        if est == Comp::A {
            (0..n).map(|i| y[i] - estacional[i]).collect()
        } else {
            for s in init_seas.iter_mut() {
                *s = s.max(0.01);
            }
            let suma: f64 = init_seas.iter().sum();
            if suma > m as f64 {
                let d: f64 = init_seas.iter().map(|s| s + 0.01).sum();
                for s in init_seas.iter_mut() {
                    *s /= d;
                }
            }
            (0..n).map(|i| y[i] / estacional[i].max(0.01)).collect()
        }
    } else {
        y.to_vec()
    };
    let mm = if est != Comp::N { m } else { 1 };
    let maxn = 10usize.max(2 * mm).min(y_sa.len());
    let mut v = vec![];
    match tend {
        Comp::N => v.push(media(&y_sa[..maxn])),
        _ => {
            let t: Vec<f64> = (1..=maxn).map(|i| i as f64).collect();
            let b = mco(&[t], &y_sa[..maxn]).ok_or("no se pudo estimar la tendencia inicial")?;
            if tend == Comp::A {
                let (mut l0, mut b0) = (b[0], b[1]);
                if (l0 + b0).abs() < 1e-8 {
                    l0 *= 1.0 + 1e-3;
                    b0 *= 1.0 - 1e-3;
                }
                v.push(l0);
                v.push(b0);
            } else {
                let mut l0 = b[0] + b[1];
                if l0.abs() < 1e-8 {
                    l0 = 1e-7;
                }
                let mut b0 = (b[0] + 2.0 * b[1]) / l0;
                l0 /= b0;
                if b0.abs() > 1e10 {
                    b0 = b0.signum() * 1e10;
                }
                if l0 < 1e-8 || b0 < 1e-8 {
                    l0 = y_sa[0].max(1e-3);
                    b0 = (y_sa[1] / y_sa[0]).max(1e-3);
                }
                v.push(l0);
                v.push(b0);
            }
        }
    }
    v.extend(init_seas);
    Ok(v)
}

// ------------------------------------------------------------------ etsmodel

const LOWER: [f64; 4] = [1e-4, 1e-4, 1e-4, 0.8];
const UPPER: [f64; 4] = [0.9999, 0.9999, 0.9999, 0.98];

fn etsmodel(y: &[f64], m: usize, err: Comp, tend: Comp, est: Comp, amortiguada: bool, crit: Criterio, nmse: usize) -> Result<EtsFit, String> {
    let m = if est != Comp::N { m } else { 1 };
    // initparam (bounds = "both")
    let (lower, mut upper) = (LOWER, UPPER);
    let alpha0 = {
        let a = lower[0] + 0.2 * (upper[0] - lower[0]) / m as f64;
        if !(0.0..=1.0).contains(&a) {
            lower[0] + 2e-3
        } else {
            a
        }
    };
    let mut par = vec![alpha0];
    let mut nombres = vec!["alpha".to_string()];
    if tend != Comp::N {
        upper[1] = upper[1].min(alpha0);
        let mut b = lower[1] + 0.1 * (upper[1] - lower[1]);
        if b < 0.0 || b > alpha0 {
            b = alpha0 - 1e-3;
        }
        par.push(b);
        nombres.push("beta".into());
    }
    if est != Comp::N {
        upper[2] = upper[2].min(1.0 - alpha0);
        let mut g = lower[2] + 0.05 * (upper[2] - lower[2]);
        if g < 0.0 || g > 1.0 - alpha0 {
            g = 1.0 - alpha0 - 1e-3;
        }
        par.push(g);
        nombres.push("gamma".into());
    }
    if amortiguada {
        let mut p = lower[3] + 0.99 * (upper[3] - lower[3]);
        if !(0.0..=1.0).contains(&p) {
            p = upper[3] - 1e-3;
        }
        par.push(p);
        nombres.push("phi".into());
    }
    // los límites de etsmodel son los originales (initparam solo cambió su copia)
    let (lower, upper) = (LOWER, UPPER);
    let ini = estado_inicial(y, m, tend, est)?;
    let nest = ini.len();
    nombres.push("l".into());
    if tend != Comp::N {
        nombres.push("b".into());
    }
    for k in 0..nest - 1 - (tend != Comp::N) as usize {
        nombres.push(format!("s{k}"));
    }
    par.extend(ini);
    let np = par.len();
    if np >= y.len() - 1 {
        return Err("muy pocos datos para este modelo".into());
    }
    let obj = Objetivo { y, nest, err, tend, est, amortiguada, lower, upper, crit, nmse, m };
    let mut b = par.clone();
    let (fit, _) = nmmin(&mut b, &|p: &[f64]| obj.valor(p), f64::NEG_INFINITY, f64::EPSILON.sqrt(), 1.0, 0.5, 2.0, 2000);
    let (alpha, beta, gamma, phi, k) = obj.parametros(&fit);
    let mut estado = fit[k..].to_vec();
    if est != Comp::N {
        let i0 = 1 + (tend != Comp::N) as usize;
        let suma: f64 = estado[i0..].iter().sum();
        estado.push(m as f64 * (est == Comp::M) as u8 as f64 - suma);
    }
    let c = etscalc(y, &estado, m, err, tend, est, alpha, beta, gamma, phi, nmse);
    if !c.lik.is_finite() || (c.lik - NA).abs() < TOL {
        return Err("el modelo no se pudo estimar".into());
    }
    let ny = y.len() as f64;
    let npl = np as f64 + 1.0;
    let aic = c.lik + 2.0 * npl;
    let bic = c.lik + ny.ln() * npl;
    let aicc = aic + 2.0 * npl * (npl + 1.0) / (ny - npl - 1.0);
    let nst = estado.len();
    let ultimo = c.estados[nst * y.len()..nst * (y.len() + 1)].to_vec();
    let sigma2 = c.e.iter().map(|e| e * e).sum::<f64>() / (ny - np as f64);
    Ok(EtsFit {
        error: err,
        tendencia: tend,
        estacion: est,
        amortiguada,
        m,
        alpha,
        beta,
        gamma,
        phi,
        par: nombres.into_iter().zip(fit.iter().copied()).collect(),
        ajustados: c.ajustados,
        residuos: c.e,
        ultimo,
        lik: c.lik,
        aic,
        aicc,
        bic,
        sigma2,
        mse: c.amse[0],
    })
}

/// `ets(y, model, damped, opt.crit)` de forecast. `modelo`: tres letras (E, T, S) con Z = automático;
/// `amortiguada`: None = probar las dos.
pub fn ets(y: &[f64], frecuencia: usize, modelo: &str, amortiguada: Option<bool>, crit: Criterio) -> Result<EtsFit, String> {
    let ny = y.len();
    let mut m = frecuencia.max(1);
    let letras: Vec<char> = modelo.chars().collect();
    if letras.len() != 3 {
        return Err(format!("modelo ETS no válido: «{modelo}»"));
    }
    let lista = |c: char, todos: &[Comp]| -> Result<Vec<Comp>, String> {
        if c.to_ascii_uppercase() == 'Z' {
            Ok(todos.to_vec())
        } else {
            Comp::desde(c).map(|x| vec![x]).ok_or_else(|| format!("modelo ETS no válido: «{modelo}»"))
        }
    };
    let errores = lista(letras[0], &[Comp::A, Comp::M])?;
    let tendencias = lista(letras[1], &[Comp::N, Comp::A])?;
    let mut estaciones = lista(letras[2], &[Comp::N, Comp::A, Comp::M])?;
    if m < 1 || ny <= m {
        estaciones = vec![Comp::N];
    }
    if m == 1 {
        if estaciones.iter().all(|e| *e != Comp::N) && letras[2].to_ascii_uppercase() != 'Z' {
            return Err("la serie no tiene estacionalidad (frecuencia 1)".into());
        }
        estaciones = vec![Comp::N];
    }
    if m > 24 {
        if letras[2].to_ascii_uppercase() == 'Z' {
            estaciones = vec![Comp::N];
        } else if estaciones[0] != Comp::N {
            return Err("frecuencia demasiado alta para ETS (más de 24)".into());
        }
    }
    if estaciones == [Comp::N] {
        m = 1.max(m);
    }
    let positiva = y.iter().cloned().fold(f64::INFINITY, f64::min) > 0.0;
    if letras[0].to_ascii_uppercase() == 'M' && !positiva {
        return Err("los modelos multiplicativos necesitan datos positivos".into());
    }
    if amortiguada == Some(true) && tendencias == [Comp::N] {
        return Err("combinación de modelo no permitida".into());
    }
    // pocos datos: R usa HoltWinters (no se traduce aquí)
    let mut npars = 2usize;
    if tendencias.iter().any(|t| *t != Comp::N) && letras[1].to_ascii_uppercase() != 'Z' {
        npars += 2;
    }
    if estaciones.iter().any(|s| *s != Comp::N) && letras[2].to_ascii_uppercase() != 'Z' {
        npars += m;
    }
    if let Some(d) = amortiguada {
        npars += d as usize;
    }
    if ny <= npars + 4 {
        return Err("muy pocos datos para ETS (R usaría HoltWinters)".into());
    }
    let amortiguadas = match amortiguada {
        Some(d) => vec![d],
        None => vec![true, false],
    };
    let mut mejor: Option<EtsFit> = None;
    for &e in &errores {
        for &t in &tendencias {
            for &s in &estaciones {
                for &d in &amortiguadas {
                    if t == Comp::N && d {
                        continue;
                    }
                    // restrict = TRUE
                    if e == Comp::A && (t == Comp::M || s == Comp::M) {
                        continue;
                    }
                    if e == Comp::M && t == Comp::M && s == Comp::A {
                        continue;
                    }
                    if !positiva && e == Comp::M {
                        continue;
                    }
                    if let Ok(f) = etsmodel(y, m, e, t, s, d, crit, 3) {
                        if f.aicc.is_finite() && mejor.as_ref().is_none_or(|b| f.aicc < b.aicc) {
                            mejor = Some(f);
                        }
                    }
                }
            }
        }
    }
    mejor.ok_or_else(|| "ningún modelo ETS se pudo ajustar".into())
}

// ------------------------------------------------------------------ forecast.ets

type Mat = Vec<Vec<f64>>;

fn mat_mul(a: &Mat, b: &Mat) -> Mat {
    let (n, k, p) = (a.len(), b.len(), b.first().map_or(0, |r| r.len()));
    let mut c = vec![vec![0.0; p]; n];
    for i in 0..n {
        for t in 0..k {
            let x = a[i][t];
            if x != 0.0 {
                for j in 0..p {
                    c[i][j] += x * b[t][j];
                }
            }
        }
    }
    c
}
fn trans(a: &Mat) -> Mat {
    let (n, p) = (a.len(), a.first().map_or(0, |r| r.len()));
    (0..p).map(|j| (0..n).map(|i| a[i][j]).collect()).collect()
}
fn kron(a: &Mat, b: &Mat) -> Mat {
    let (an, ap, bn, bp) = (a.len(), a[0].len(), b.len(), b[0].len());
    let mut c = vec![vec![0.0; ap * bp]; an * bn];
    for i in 0..an {
        for j in 0..ap {
            for k in 0..bn {
                for l in 0..bp {
                    c[i * bn + k][j * bp + l] = a[i][j] * b[k][l];
                }
            }
        }
    }
    c
}
fn suma(a: &Mat, b: &Mat) -> Mat {
    a.iter().zip(b).map(|(x, y)| x.iter().zip(y).map(|(u, v)| u + v).collect()).collect()
}
fn escalar(a: &Mat, s: f64) -> Mat {
    a.iter().map(|r| r.iter().map(|x| x * s).collect()).collect()
}

/// class1(): medias y varianzas de los modelos lineales con error aditivo.
fn clase1(f: &EtsFit, h: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let p = f.ultimo.len();
    let m = f.m;
    let mut hh = vec![0.0; p];
    hh[0] = 1.0;
    if f.estacion == Comp::A {
        hh[p - 1] = 1.0;
    }
    if f.tendencia == Comp::A {
        hh[1] = if f.amortiguada { f.phi } else { 1.0 };
    }
    let mut ff = vec![vec![0.0; p]; p];
    ff[0][0] = 1.0;
    if f.tendencia == Comp::A {
        let v = if f.amortiguada { f.phi } else { 1.0 };
        ff[0][1] = v;
        ff[1][1] = v;
    }
    if f.estacion == Comp::A {
        ff[p - m][p - 1] = 1.0;
        for k in 0..m - 1 {
            ff[p - m + 1 + k][p - m + k] = 1.0;
        }
    }
    let mut g = vec![0.0; p];
    g[0] = f.alpha;
    if f.tendencia == Comp::A {
        g[1] = f.beta;
    }
    if f.estacion == Comp::A {
        g[2] = f.gamma;
    }
    let mut mu = vec![0.0; h];
    let mut cj = vec![0.0; h.saturating_sub(1)];
    let mut fj: Mat = (0..p).map(|i| (0..p).map(|j| if i == j { 1.0 } else { 0.0 }).collect()).collect();
    let hfx = |fj: &Mat, x: &[f64]| -> f64 { (0..p).map(|i| hh[i] * (0..p).map(|j| fj[i][j] * x[j]).sum::<f64>()).sum() };
    for i in 0..h.saturating_sub(1) {
        mu[i] = hfx(&fj, &f.ultimo);
        cj[i] = hfx(&fj, &g);
        fj = mat_mul(&fj, &ff);
    }
    mu[h - 1] = hfx(&fj, &f.ultimo);
    let mut var = vec![f.sigma2; h];
    let mut acum = 0.0;
    for i in 1..h {
        acum += cj[i - 1] * cj[i - 1];
        var[i] = f.sigma2 * (1.0 + acum);
    }
    (mu, var, cj)
}

fn clase2(f: &EtsFit, h: usize) -> (Vec<f64>, Vec<f64>) {
    let (mu, _, cj) = clase1(f, h);
    let mut theta = vec![0.0; h];
    theta[0] = mu[0] * mu[0];
    for j in 1..h {
        theta[j] = mu[j] * mu[j] + f.sigma2 * (0..j).map(|k| cj[k] * cj[k] * theta[j - 1 - k]).sum::<f64>();
    }
    let var = (0..h).map(|j| (1.0 + f.sigma2) * theta[j] - mu[j] * mu[j]).collect();
    (mu, var)
}

fn clase3(f: &EtsFit, h: usize) -> (Vec<f64>, Vec<f64>) {
    let p = f.ultimo.len();
    let m = f.m;
    let tn = if f.tendencia != Comp::N { 2 } else { 1 };
    let h1: Mat = vec![vec![1.0; tn]];
    let mut h2v = vec![0.0; m];
    h2v[m - 1] = 1.0;
    let h2: Mat = vec![h2v];
    let (f1, g1): (Mat, Mat) = if f.tendencia == Comp::N {
        (vec![vec![1.0]], vec![vec![f.alpha]])
    } else {
        (vec![vec![1.0, 1.0], vec![0.0, if f.amortiguada { f.phi } else { 1.0 }]], vec![vec![f.alpha, f.alpha], vec![f.beta, f.beta]])
    };
    let mut f2 = vec![vec![0.0; m]; m];
    f2[0][m - 1] = 1.0;
    for i in 1..m {
        f2[i][i - 1] = 1.0;
    }
    let mut g2 = vec![vec![0.0; m]; m];
    g2[0][m - 1] = f.gamma;
    let a: Mat = f.ultimo[..p - m].iter().map(|x| vec![*x]).collect();
    let b: Mat = vec![f.ultimo[p - m..].to_vec()];
    let mut mh = mat_mul(&a, &b);
    let q = mh.len() * mh[0].len();
    let mut vh = vec![vec![0.0; q]; q];
    let h21 = kron(&h2, &h1);
    let f21 = kron(&f2, &f1);
    let g21 = kron(&g2, &g1);
    let k = suma(&kron(&g2, &f1), &kron(&f2, &g1));
    let s2 = f.sigma2;
    let (mut mu, mut var) = (vec![0.0; h], vec![0.0; h]);
    for i in 0..h {
        mu[i] = mat_mul(&mat_mul(&h1, &mh), &trans(&h2))[0][0];
        var[i] = (1.0 + s2) * mat_mul(&mat_mul(&h21, &vh), &trans(&h21))[0][0] + s2 * mu[i] * mu[i];
        // vec(Mh): por columnas
        let filas = mh.len();
        let cols = mh[0].len();
        let vecmh: Vec<f64> = (0..cols).flat_map(|j| (0..filas).map(move |r| (r, j))).map(|(r, j)| mh[r][j]).collect();
        let vv: Mat = vecmh.iter().map(|x| vecmh.iter().map(|y| x * y).collect()).collect();
        let t1 = mat_mul(&mat_mul(&f21, &vh), &trans(&f21));
        let t2 = mat_mul(&mat_mul(&f21, &vh), &trans(&g21));
        let t3 = mat_mul(&mat_mul(&g21, &vh), &trans(&f21));
        let t4 = mat_mul(&mat_mul(&k, &suma(&vh, &vv)), &trans(&k));
        let t5 = mat_mul(&mat_mul(&g21, &suma(&escalar(&vh, 3.0), &escalar(&vv, 2.0))), &trans(&g21));
        vh = suma(&t1, &escalar(&suma(&suma(&suma(&t2, &t3), &t4), &escalar(&t5, s2)), s2));
        mh = suma(&mat_mul(&mat_mul(&f1, &mh), &trans(&f2)), &escalar(&mat_mul(&mat_mul(&g1, &mh), &trans(&g2)), s2));
    }
    (mu, var)
}

/// Pronóstico de `forecast.ets()`: medias y límites del nivel dado (%).
pub fn pronosticar(f: &EtsFit, h: usize, nivel: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    if h == 0 {
        return (vec![], vec![], vec![]);
    }
    let (mu, var) = match (f.error, f.tendencia, f.estacion) {
        (Comp::A, Comp::A | Comp::N, Comp::N | Comp::A) => {
            let (m, v, _) = clase1(f, h);
            (m, Some(v))
        }
        (Comp::M, Comp::A | Comp::N, Comp::N | Comp::A) => {
            let (m, v) = clase2(f, h);
            (m, Some(v))
        }
        (Comp::M, t, Comp::M) if t != Comp::M => {
            let (m, v) = clase3(f, h);
            (m, Some(v))
        }
        _ => {
            // tendencia multiplicativa: solo la media (R simula los intervalos)
            let mut out = vec![0.0; h];
            let p = f.ultimo.len();
            let tn = (f.tendencia != Comp::N) as usize;
            let s: Vec<f64> = if f.estacion != Comp::N { f.ultimo[1 + tn..p].to_vec() } else { vec![0.0] };
            pronostico_pasos(f.ultimo[0], if tn == 1 { f.ultimo[1] } else { 0.0 }, &s, f.m, f.tendencia, f.estacion, if f.amortiguada { f.phi } else { 1.0 }, &mut out);
            (out, None)
        }
    };
    let z = Normal::new(0.0, 1.0).map(|n| n.inverse_cdf(1.0 - (100.0 - nivel) / 200.0)).unwrap_or(1.96).abs();
    match var {
        Some(v) => {
            let inf = mu.iter().zip(&v).map(|(m, v)| m - v.max(0.0).sqrt() * z).collect();
            let sup = mu.iter().zip(&v).map(|(m, v)| m + v.max(0.0).sqrt() * z).collect();
            (mu, inf, sup)
        }
        None => (mu.clone(), vec![f64::NAN; h], vec![f64::NAN; h]),
    }
}
