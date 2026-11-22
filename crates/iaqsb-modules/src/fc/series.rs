//! Pronósticos de series de tiempo: los doce métodos de WinQSB.

use super::{Criterion, FcModel, FcOptions, Method};
use iaqsb_core::*;
use serde_json::json;

#[derive(Clone, Debug)]
pub struct Params {
    pub m: usize,
    pub weights: Vec<f64>,
    pub alpha: f64,
    pub beta: f64,
    pub gamma: f64,
    pub c: usize,
    pub f0: Option<f64>,
    pub t0: Option<f64>,
}

/// Pronóstico de cada período (None donde el método aún no pronostica) y de
/// los períodos futuros, más columnas auxiliares para el paso a paso.
#[derive(Clone, Debug, Default)]
pub struct Fit {
    pub f: Vec<Option<f64>>,
    pub aux: Vec<(String, Vec<Option<f64>>)>,
    pub seasonal: Vec<f64>,
    pub line: Option<(f64, f64)>,
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

/// Recta de mínimos cuadrados y = a + b·x con x = 1..n.
fn ols(v: &[f64]) -> (f64, f64) {
    let n = v.len() as f64;
    let xm = (n + 1.0) / 2.0;
    let ym = mean(v);
    let sxx: f64 = (1..=v.len()).map(|x| (x as f64 - xm).powi(2)).sum();
    let sxy: f64 = v.iter().enumerate().map(|(i, y)| (i as f64 + 1.0 - xm) * (y - ym)).sum();
    let b = if sxx > 0.0 { sxy / sxx } else { 0.0 };
    (ym - b * xm, b)
}

pub fn forecast(method: Method, a: &[f64], p: &Params, h: usize) -> Result<Fit, String> {
    let n = a.len();
    let mut f: Vec<Option<f64>> = vec![None; n + h];
    let mut fit = Fit::default();
    let unit = |x: f64, what: &str| if (0.0..=1.0).contains(&x) { Ok(()) } else { Err(format!("{what} debe estar entre 0 y 1")) };
    match method {
        Method::SimpleAverage => {
            for t in 1..=n {
                let v = mean(&a[..t]);
                if t < n {
                    f[t] = Some(v);
                } else {
                    for k in 0..h {
                        f[n + k] = Some(v);
                    }
                }
            }
        }
        Method::MovingAverage | Method::WeightedMovingAverage => {
            let m = p.m;
            if m == 0 || m > n {
                return Err(format!("los períodos del promedio deben estar entre 1 y {n}"));
            }
            let w: Vec<f64> = if method == Method::WeightedMovingAverage && !p.weights.is_empty() {
                if p.weights.len() != m {
                    return Err(format!("hay {} pesos y el promedio usa {m} períodos", p.weights.len()));
                }
                let s: f64 = p.weights.iter().sum();
                if s <= 0.0 || p.weights.iter().any(|x| *x < 0.0) {
                    return Err("los pesos deben ser no negativos y sumar más que 0".into());
                }
                p.weights.iter().map(|x| x / s).collect()
            } else {
                vec![1.0 / m as f64; m]
            };
            for t in m..=n {
                let v: f64 = (0..m).map(|i| w[i] * a[t - m + i]).sum();
                if t < n {
                    f[t] = Some(v);
                } else {
                    for k in 0..h {
                        f[n + k] = Some(v);
                    }
                }
            }
        }
        Method::MovingAverageTrend => {
            let m = p.m;
            if m < 2 || m > n {
                return Err(format!("el promedio móvil con tendencia necesita entre 2 y {n} períodos"));
            }
            let mut slope = vec![None; n + h];
            for t in m..=n {
                let (a0, b) = ols(&a[t - m..t]);
                if t < n {
                    f[t] = Some(a0 + b * (m as f64 + 1.0));
                    slope[t] = Some(b);
                } else {
                    for k in 0..h {
                        f[n + k] = Some(a0 + b * (m + 1 + k) as f64);
                    }
                }
            }
            fit.aux.push(("Pendiente".into(), slope));
        }
        Method::SingleExponential => {
            unit(p.alpha, "α")?;
            let mut l;
            let start;
            if let Some(f0) = p.f0 {
                l = f0;
                start = 0;
            } else {
                l = a[0];
                start = 1;
            }
            for t in start..n {
                f[t] = Some(l);
                l += p.alpha * (a[t] - l);
            }
            for k in 0..h {
                f[n + k] = Some(l);
            }
        }
        Method::SingleExponentialTrend => {
            unit(p.alpha, "α")?;
            unit(p.beta, "β")?;
            let (mut s, mut tr, start) = match p.f0 {
                Some(f0) => (f0, p.t0.unwrap_or(0.0), 0),
                None => (a[0], p.t0.unwrap_or(0.0), 1),
            };
            let mut sc = vec![None; n + h];
            let mut tc = vec![None; n + h];
            if start == 1 {
                sc[0] = Some(s);
                tc[0] = Some(tr);
            }
            for t in start..n {
                f[t] = Some(s + tr);
                let s2 = p.alpha * a[t] + (1.0 - p.alpha) * (s + tr);
                tr = p.beta * (s2 - s) + (1.0 - p.beta) * tr;
                s = s2;
                sc[t] = Some(s);
                tc[t] = Some(tr);
            }
            for k in 0..h {
                f[n + k] = Some(s + (k + 1) as f64 * tr);
            }
            fit.aux.push(("S(t)".into(), sc));
            fit.aux.push(("T(t)".into(), tc));
        }
        Method::DoubleExponential | Method::DoubleExponentialTrend => {
            unit(p.alpha, "α")?;
            if method == Method::DoubleExponentialTrend && p.alpha >= 1.0 {
                return Err("con tendencia, α debe ser menor que 1".into());
            }
            let (mut s1, start) = match p.f0 {
                Some(f0) => (f0, 0),
                None => (a[0], 1),
            };
            let mut s2 = p.t0.unwrap_or(s1);
            let trend = |s1: f64, s2: f64| if method == Method::DoubleExponentialTrend { p.alpha / (1.0 - p.alpha) * (s1 - s2) } else { 0.0 };
            let mut c1 = vec![None; n + h];
            let mut c2 = vec![None; n + h];
            if start == 1 {
                c1[0] = Some(s1);
                c2[0] = Some(s2);
            }
            for t in start..n {
                f[t] = Some(2.0 * s1 - s2 + trend(s1, s2));
                s1 = p.alpha * a[t] + (1.0 - p.alpha) * s1;
                s2 = p.alpha * s1 + (1.0 - p.alpha) * s2;
                c1[t] = Some(s1);
                c2[t] = Some(s2);
            }
            for k in 0..h {
                f[n + k] = Some(2.0 * s1 - s2 + (k + 1) as f64 * trend(s1, s2));
            }
            fit.aux.push(("F(t)".into(), c1));
            fit.aux.push(("F'(t)".into(), c2));
        }
        Method::AdaptiveExponential => {
            unit(p.alpha, "α")?;
            unit(p.beta, "β")?;
            let (mut l, start) = match p.f0 {
                Some(f0) => (f0, 0),
                None => (a[0], 1),
            };
            let mut al = p.alpha;
            let (mut e_s, mut m_s) = (0.0, 0.0);
            let mut ac = vec![None; n + h];
            for t in start..n {
                f[t] = Some(l);
                ac[t] = Some(al);
                let e = a[t] - l;
                l += al * e;
                e_s = p.beta * e + (1.0 - p.beta) * e_s;
                m_s = p.beta * e.abs() + (1.0 - p.beta) * m_s;
                if m_s > 0.0 {
                    al = (e_s / m_s).abs();
                }
            }
            for k in 0..h {
                f[n + k] = Some(l);
            }
            fit.aux.push(("α usado".into(), ac));
        }
        Method::LinearTrend => {
            let (a0, b0) = ols(a);
            let a1 = p.f0.unwrap_or(a0);
            let b1 = p.t0.unwrap_or(b0);
            for t in 0..n + h {
                f[t] = Some(a1 + b1 * (t + 1) as f64);
            }
            fit.line = Some((a1, b1));
        }
        Method::HoltWintersAdditive | Method::HoltWintersMultiplicative => {
            unit(p.alpha, "α")?;
            unit(p.beta, "β")?;
            unit(p.gamma, "γ")?;
            let c = p.c;
            let mult = method == Method::HoltWintersMultiplicative;
            if c < 2 {
                return Err("la longitud del ciclo debe ser al menos 2".into());
            }
            if n < c + 1 {
                return Err(format!("Holt-Winters necesita más de un ciclo de datos (al menos {})", c + 1));
            }
            if mult && a.iter().any(|x| *x <= 0.0) {
                return Err("el método multiplicativo necesita datos positivos".into());
            }
            let u = mean(&a[..c]);
            let mut lv = p.f0.unwrap_or(u);
            let mut tr = p.t0.unwrap_or(if n >= 2 * c { (mean(&a[c..2 * c]) - u) / c as f64 } else { 0.0 });
            let mut s: Vec<f64> = (0..c).map(|i| if mult { a[i] / u } else { a[i] - u }).collect();
            let mut lc = vec![None; n + h];
            let mut tc = vec![None; n + h];
            let mut sc: Vec<Option<f64>> = vec![None; n + h];
            for i in 0..c {
                sc[i] = Some(s[i]);
            }
            lc[c - 1] = Some(lv);
            tc[c - 1] = Some(tr);
            for t in c..n {
                let si = s[t - c];
                f[t] = Some(if mult { (lv + tr) * si } else { lv + tr + si });
                let l2 = if mult { p.alpha * a[t] / si + (1.0 - p.alpha) * (lv + tr) } else { p.alpha * (a[t] - si) + (1.0 - p.alpha) * (lv + tr) };
                tr = p.beta * (l2 - lv) + (1.0 - p.beta) * tr;
                lv = l2;
                let snew = if mult { p.gamma * a[t] / lv + (1.0 - p.gamma) * si } else { p.gamma * (a[t] - lv) + (1.0 - p.gamma) * si };
                s.push(snew);
                lc[t] = Some(lv);
                tc[t] = Some(tr);
                sc[t] = Some(snew);
            }
            for k in 0..h {
                let si = s[n - c + (k % c)];
                f[n + k] = Some(if mult { (lv + (k + 1) as f64 * tr) * si } else { lv + (k + 1) as f64 * tr + si });
            }
            fit.aux.push(("F(t)".into(), lc));
            fit.aux.push(("T(t)".into(), tc));
            fit.aux.push(("S(t)".into(), sc));
            fit.seasonal = s[s.len() - c..].to_vec();
        }
    }
    fit.f = f;
    Ok(fit)
}

#[derive(Clone, Debug, Default)]
pub struct Errors {
    pub cfe: f64,
    pub mad: f64,
    pub mse: f64,
    pub mape: f64,
    pub signal: f64,
    pub count: usize,
    /// Por período: (error, CFE, MAD, MSE, MAPE, señal) acumulados.
    pub rows: Vec<Option<[f64; 6]>>,
}

pub fn errors(a: &[f64], f: &[Option<f64>]) -> Errors {
    let mut e = Errors::default();
    let (mut sabs, mut ssq, mut spct, mut npct) = (0.0, 0.0, 0.0, 0);
    for t in 0..a.len() {
        match f[t] {
            Some(ft) => {
                let err = a[t] - ft;
                e.count += 1;
                e.cfe += err;
                sabs += err.abs();
                ssq += err * err;
                if a[t] != 0.0 {
                    spct += (err / a[t]).abs();
                    npct += 1;
                }
                let k = e.count as f64;
                let mad = sabs / k;
                let mape = if npct > 0 { 100.0 * spct / npct as f64 } else { 0.0 };
                e.rows.push(Some([err, e.cfe, mad, ssq / k, mape, if mad > 0.0 { e.cfe / mad } else { 0.0 }]));
            }
            None => e.rows.push(None),
        }
    }
    if e.count > 0 {
        let k = e.count as f64;
        e.mad = sabs / k;
        e.mse = ssq / k;
        e.mape = if npct > 0 { 100.0 * spct / npct as f64 } else { 0.0 };
        e.signal = if e.mad > 0.0 { e.cfe / e.mad } else { 0.0 };
    }
    e
}

fn score(e: &Errors, c: Criterion) -> f64 {
    if e.count == 0 {
        return f64::INFINITY;
    }
    match c {
        Criterion::Mse => e.mse,
        Criterion::Mad => e.mad,
        Criterion::Mape => e.mape,
    }
}

/// Busca los mejores parámetros en una rejilla (paso 0.01; 0.05 y luego 0.01 con tres parámetros).
fn search(method: Method, a: &[f64], base: &Params, crit: Criterion) -> Params {
    let eval = |p: &Params| forecast(method, a, p, 0).map(|fit| score(&errors(a, &fit.f), crit)).unwrap_or(f64::INFINITY);
    let mut best = base.clone();
    let mut bv = eval(&best);
    let try_p = |p: Params, best: &mut Params, bv: &mut f64| {
        let v = eval(&p);
        if v < *bv - 1e-12 {
            *bv = v;
            *best = p;
        }
    };
    let grid = |lo: f64, hi: f64, step: f64| -> Vec<f64> {
        let k = ((hi - lo) / step).round() as usize;
        (0..=k).map(|i| ((lo + i as f64 * step) * 1e6).round() / 1e6).filter(|x| *x > 0.0 && *x <= 1.0).collect()
    };
    match method {
        Method::MovingAverage | Method::WeightedMovingAverage => {
            for m in 1..a.len() {
                try_p(Params { m, weights: vec![], ..base.clone() }, &mut best, &mut bv);
            }
        }
        Method::MovingAverageTrend => {
            for m in 2..a.len() {
                try_p(Params { m, ..base.clone() }, &mut best, &mut bv);
            }
        }
        Method::SingleExponential | Method::DoubleExponential | Method::DoubleExponentialTrend => {
            for al in grid(0.01, 0.99, 0.01) {
                try_p(Params { alpha: al, ..base.clone() }, &mut best, &mut bv);
            }
        }
        Method::SingleExponentialTrend | Method::AdaptiveExponential => {
            for al in grid(0.01, 0.99, 0.01) {
                for be in grid(0.01, 0.99, 0.01) {
                    try_p(Params { alpha: al, beta: be, ..base.clone() }, &mut best, &mut bv);
                }
            }
        }
        Method::HoltWintersAdditive | Method::HoltWintersMultiplicative => {
            for al in grid(0.05, 0.95, 0.05) {
                for be in grid(0.05, 0.95, 0.05) {
                    for ga in grid(0.05, 0.95, 0.05) {
                        try_p(Params { alpha: al, beta: be, gamma: ga, ..base.clone() }, &mut best, &mut bv);
                    }
                }
            }
            let c = best.clone();
            for al in grid(c.alpha - 0.05, c.alpha + 0.05, 0.01) {
                for be in grid(c.beta - 0.05, c.beta + 0.05, 0.01) {
                    for ga in grid(c.gamma - 0.05, c.gamma + 0.05, 0.01) {
                        try_p(Params { alpha: al, beta: be, gamma: ga, ..c.clone() }, &mut best, &mut bv);
                    }
                }
            }
        }
        _ => {}
    }
    best
}

fn params_text(method: Method, p: &Params) -> String {
    match method {
        Method::SimpleAverage => String::new(),
        Method::MovingAverage | Method::MovingAverageTrend => format!("m = {}", p.m),
        Method::WeightedMovingAverage => format!("m = {}{}", p.m, if p.weights.is_empty() { String::new() } else { format!(", pesos {}", p.weights.iter().map(|w| fmt(*w)).collect::<Vec<_>>().join(", ")) }),
        Method::SingleExponential | Method::DoubleExponential | Method::DoubleExponentialTrend => format!("α = {}", fmt(p.alpha)),
        Method::SingleExponentialTrend | Method::AdaptiveExponential => format!("α = {}, β = {}", fmt(p.alpha), fmt(p.beta)),
        Method::LinearTrend => String::new(),
        Method::HoltWintersAdditive | Method::HoltWintersMultiplicative => format!("c = {}, α = {}, β = {}, γ = {}", p.c, fmt(p.alpha), fmt(p.beta), fmt(p.gamma)),
    }
}

fn formula(method: Method) -> &'static str {
    match method {
        Method::SimpleAverage => "F(t+1) = promedio de todos los datos hasta t.",
        Method::MovingAverage => "F(t+1) = promedio de los últimos m datos.",
        Method::WeightedMovingAverage => "F(t+1) = Σ wᵢ·A(t−m+i), con pesos normalizados para que sumen 1 (el último peso es el del dato más reciente).",
        Method::MovingAverageTrend => "Se ajusta una recta de mínimos cuadrados a los últimos m datos y se prolonga un período: F(t+1) = a + b·(m + 1).",
        Method::SingleExponential => "F(t+1) = F(t) + α·(A(t) − F(t)). Sin F(0), el primer pronóstico es el primer dato.",
        Method::SingleExponentialTrend => "S(t) = α·A(t) + (1 − α)·(S(t−1) + T(t−1)); T(t) = β·(S(t) − S(t−1)) + (1 − β)·T(t−1); F(t+k) = S(t) + k·T(t).",
        Method::DoubleExponential => "F(t) = α·A(t) + (1 − α)·F(t−1); F'(t) = α·F(t) + (1 − α)·F'(t−1); pronóstico = 2F(t) − F'(t).",
        Method::DoubleExponentialTrend => "Como el doble, más la tendencia b = α/(1 − α)·(F(t) − F'(t)): F(t+k) = 2F(t) − F'(t) + k·b.",
        Method::AdaptiveExponential => "F(t+1) = F(t) + αₜ·(A(t) − F(t)), con αₜ = |E(t)/M(t)|: E y M son el error y el error absoluto suavizados con β.",
        Method::LinearTrend => "Recta de mínimos cuadrados con el tiempo: F(t) = a + b·t.",
        Method::HoltWintersAdditive => "F(t) = α·(A(t) − S(t−c)) + (1 − α)·(F(t−1) + T(t−1)); T(t) = β·(F(t) − F(t−1)) + (1 − β)·T(t−1); S(t) = γ·(A(t) − F(t)) + (1 − γ)·S(t−c); pronóstico = F(t) + k·T(t) + S(t−c+k).",
        Method::HoltWintersMultiplicative => "F(t) = α·A(t)/S(t−c) + (1 − α)·(F(t−1) + T(t−1)); T(t) = β·(F(t) − F(t−1)) + (1 − β)·T(t−1); S(t) = γ·A(t)/F(t) + (1 − γ)·S(t−c); pronóstico = (F(t) + k·T(t))·S(t−c+k).",
    }
}

pub fn parse_list(s: &str) -> Result<Vec<f64>, ModError> {
    s.split([',', ';', ' '])
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(|x| iaqsb_core::winqsb::parse_num(x).ok_or_else(|| ModError::Invalid(format!("«{x}» no es un número"))))
        .collect()
}

pub fn solve(m: &FcModel, o: &FcOptions) -> Result<Report, ModError> {
    // los datos vacíos al final se ignoran; en medio, no
    let mut vals: Vec<Option<f64>> = m.data.iter().map(|d| d.value).collect();
    while vals.last().map(|v| v.is_none()).unwrap_or(false) {
        vals.pop();
    }
    if let Some(i) = vals.iter().position(|v| v.is_none()) {
        return invalid(format!("falta el dato del período {}", m.data[i].name));
    }
    let a: Vec<f64> = vals.into_iter().map(|v| v.unwrap()).collect();
    let n = a.len();
    if n < 2 {
        return invalid("hacen falta al menos dos datos");
    }
    let names: Vec<String> = m.data.iter().take(n).enumerate().map(|(i, d)| if d.name.trim().is_empty() { (i + 1).to_string() } else { d.name.clone() }).collect();
    let h = o.horizon.min(10_000) as usize;
    let mut p = Params {
        m: o.periods.max(1) as usize,
        weights: parse_list(&o.weights)?,
        alpha: o.alpha,
        beta: o.beta,
        gamma: o.gamma,
        c: o.cycle as usize,
        f0: o.initial_level,
        t0: o.initial_trend,
    };
    if o.search {
        p = search(o.method, &a, &p, o.criterion);
    }
    let fit = forecast(o.method, &a, &p, h).map_err(ModError::Invalid)?;
    let er = errors(&a, &fit.f);
    let u = &m.time_unit;
    let labels: Vec<String> = names.iter().cloned().chain((1..=h).map(|k| format!("{}", n + k))).collect();
    // detalle
    let mut head: Vec<String> = vec![format!("Período ({u})"), "Dato".into(), "Pronóstico".into()];
    head.extend(fit.aux.iter().map(|x| x.0.clone()));
    head.extend(["Error", "CFE", "MAD", "MSE", "MAPE (%)", "Señal de rastreo"].map(String::from));
    let mut dt = Table::new(Some(&format!("Detalle del pronóstico: {}", o.method.label())), &head).left(0);
    for t in 0..n + h {
        let mut row = vec![Cell::from(&labels[t]), if t < n { Cell::Num(a[t]) } else { Cell::Empty }, fit.f[t].map(|x| Cell::Num(tidy(x))).unwrap_or(Cell::Empty)];
        row.extend(fit.aux.iter().map(|x| x.1[t].map(|v| Cell::Num(tidy(v))).unwrap_or(Cell::Empty)));
        match if t < n { er.rows[t] } else { None } {
            Some(r) => row.extend(r.iter().map(|v| Cell::Num(tidy(*v)))),
            None => row.extend((0..6).map(|_| Cell::Empty)),
        }
        dt.row_tone(row, if t >= n { Tone::Highlight } else { Tone::Normal });
    }
    let next: Vec<f64> = (0..h).filter_map(|k| fit.f[n + k]).collect();
    let mut mets = vec![];
    if let Some(v) = next.first() {
        mets.push(metric_main(format!("Pronóstico del período {}", n + 1), tidy(*v)));
    }
    mets.extend([metric("CFE", tidy(er.cfe)), metric("MAD", tidy(er.mad)), metric("MSE", tidy(er.mse)), metric("MAPE (%)", tidy(er.mape)), metric("Señal de rastreo", tidy(er.signal))]);
    let mut blocks = vec![metrics(mets)];
    let pt = params_text(o.method, &p);
    if !pt.is_empty() {
        blocks.push(text(format!("{}{}: {pt}.", if o.search { "Mejores parámetros encontrados" } else { "Parámetros" }, if o.search { format!(" (según {:?})", o.criterion).replace("Mse", "MSE").replace("Mad", "MAD").replace("Mape", "MAPE") } else { String::new() })));
    }
    if let Some((a0, b0)) = fit.line {
        blocks.push(text(format!("Recta: F(t) = {} + {}·t", fmt(tidy(a0)), fmt(tidy(b0)))));
    }
    if er.signal.abs() > 4.0 {
        blocks.push(text_tone(format!("La señal de rastreo ({}) está fuera de ±4: el pronóstico tiene sesgo.", fmt(tidy(er.signal))), Tone::Accent));
    }
    if !fit.seasonal.is_empty() {
        let mut st = Table::new(Some("Índices estacionales finales"), &["Posición en el ciclo", "Índice"]);
        for (i, s) in fit.seasonal.iter().enumerate() {
            st.row(cells![i + 1, tidy(*s)]);
        }
        blocks.push(st.block());
    }
    blocks.push(dt.clone().block());
    // gráfico
    let mut ch = Chart::new(Some("Datos y pronóstico"), &format!("Período ({u})"), "Valor");
    ch.categories = Some(labels.clone());
    ch = ch
        .series(Series::new("Dato", SeriesKind::Line, (0..n).map(|t| [t as f64, a[t]]).collect()).tone(Tone::Accent))
        .series(Series::new("Pronóstico", SeriesKind::Line, (0..n + h).map(|t| [t as f64, fit.f[t].unwrap_or(f64::NAN)]).collect()).tone(Tone::Highlight).dashed());
    // comparación de los métodos con los mismos parámetros
    let mut ct = Table::new(Some("Comparación de métodos (mismos parámetros)"), &["Método", "Parámetros", "MAD", "MSE", "MAPE (%)", "Señal", "Próximo pronóstico"]).left(0).left(1);
    let mut cmp = vec![];
    for &mt in &Method::ALL {
        match forecast(mt, &a, &p, 1) {
            Ok(ft) => {
                let e = errors(&a, &ft.f);
                ct.row_tone(cells![mt.label(), params_text(mt, &p), tidy(e.mad), tidy(e.mse), tidy(e.mape), tidy(e.signal), ft.f[n].map(tidy)], if mt == o.method { Tone::Highlight } else { Tone::Normal });
                cmp.push(json!({"method": mt.id(), "mad": tidy(e.mad), "mse": tidy(e.mse), "mape": tidy(e.mape)}));
            }
            Err(msg) => {
                ct.row_tone(cells![mt.label(), msg, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty], Tone::Muted);
            }
        }
    }
    let mut st = vec![];
    if o.steps {
        let first: Vec<usize> = (0..n).filter(|t| fit.f[*t].is_some()).take(3).collect();
        let ex = first
            .iter()
            .map(|&t| format!("Período {}: pronóstico {}, dato {}, error {}.", labels[t], fmt(tidy(fit.f[t].unwrap())), fmt(a[t]), fmt(tidy(a[t] - fit.f[t].unwrap()))))
            .collect::<Vec<_>>()
            .join(" ");
        let mut v = vec![step(o.method.label(), format!("{} {}", formula(o.method), if pt.is_empty() { String::new() } else { format!("Parámetros: {pt}.") }), vec![])];
        if o.search {
            v.push(step("Búsqueda de parámetros", "Se probaron los parámetros en una rejilla (paso 0.01; con tres parámetros, 0.05 y luego 0.01 cerca del mejor) y se eligió el de menor error.", vec![]));
        }
        v.push(step("Primeros pronósticos", ex, vec![]));
        v.push(step(
            "Medidas del error",
            "Error = dato − pronóstico. CFE = suma de los errores (sesgo); MAD = promedio de |error|; MSE = promedio de error²; MAPE = promedio de |error/dato|·100; señal de rastreo = CFE/MAD (fuera de ±4 indica sesgo).",
            vec![dt.block()],
        ));
        v.push(step("Comparar métodos", "Con los mismos datos y parámetros, el método con menor MAD o MSE suele ser el mejor candidato.", vec![ct.clone().block()]));
        st.push(steps(v));
    }
    let headline = match next.first() {
        Some(v) => format!("{}: pronóstico {} para el período {}; MAD {}, MSE {}.", o.method.short(), fmt(tidy(*v)), n + 1, fmt(tidy(er.mad)), fmt(tidy(er.mse))),
        None => format!("{}: MAD {}, MSE {}.", o.method.short(), fmt(tidy(er.mad)), fmt(tidy(er.mse))),
    };
    Ok(Report::new("fc", &m.name, Status::Solved, headline)
        .section("solucion", "Solución", blocks)
        .section("grafico", "Gráfico", vec![ch.block()])
        .section("comparacion", "Comparación", vec![ct.block()])
        .section("pasos", "Paso a paso", st)
        .data(json!({
            "method": o.method.id(),
            "forecast": next.iter().map(|x| tidy(*x)).collect::<Vec<_>>(),
            "fitted": fit.f[..n].iter().map(|x| x.map(tidy)).collect::<Vec<_>>(),
            "cfe": tidy(er.cfe), "mad": tidy(er.mad), "mse": tidy(er.mse), "mape": tidy(er.mape), "tracking_signal": tidy(er.signal),
            "parameters": {"periods": p.m, "alpha": p.alpha, "beta": p.beta, "gamma": p.gamma, "cycle": p.c},
            "comparison": cmp,
        })))
}
