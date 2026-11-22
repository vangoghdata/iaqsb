//! Herramientas de QCC y armado del informe: capacidad del proceso, curva OC,
//! histograma con prueba ji cuadrada, gráfico de probabilidad y Pareto.

use super::chart::{self, median, sd, Oc, Plot};
use super::*;
use statrs::distribution::{Binomial, ChiSquared, Continuous, ContinuousCDF, DiscreteCDF, Exp, Gamma, LogNormal, Normal, Poisson, Uniform, Weibull};

fn phi(z: f64) -> f64 {
    Normal::new(0.0, 1.0).unwrap().cdf(z)
}

// ------------------------------------------------------------ distribuciones

pub struct Fit {
    pub name: String,
    pub params: usize,
    cdf: Box<dyn Fn(f64) -> f64>,
    pdf: Box<dyn Fn(f64) -> f64>,
    inv: Box<dyn Fn(f64) -> f64>,
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len().max(1) as f64
}

pub fn fit(d: FitDist, x: &[f64]) -> Result<Fit, ModError> {
    let m = mean(x);
    let s = sd(x);
    let mk = |name: String, params: usize, c: Box<dyn Fn(f64) -> f64>, p: Box<dyn Fn(f64) -> f64>, i: Box<dyn Fn(f64) -> f64>| Fit { name, params, cdf: c, pdf: p, inv: i };
    Ok(match d {
        FitDist::Normal => {
            if !(s > 0.0) {
                return invalid("los datos no varían");
            }
            let n = Normal::new(m, s).unwrap();
            let (n1, n2) = (n, n);
            mk(format!("Normal(μ = {}, σ = {})", fmt(tidy(m)), fmt(tidy(s))), 2, Box::new(move |t| n.cdf(t)), Box::new(move |t| n1.pdf(t)), Box::new(move |p| n2.inverse_cdf(p)))
        }
        FitDist::Lognormal => {
            if x.iter().any(|v| *v <= 0.0) {
                return invalid("la lognormal necesita datos positivos");
            }
            let l: Vec<f64> = x.iter().map(|v| v.ln()).collect();
            let (ml, sl) = (mean(&l), sd(&l));
            let d = LogNormal::new(ml, sl.max(1e-12)).unwrap();
            mk(format!("Lognormal(μ ln = {}, σ ln = {})", fmt(tidy(ml)), fmt(tidy(sl))), 2, Box::new(move |t| d.cdf(t)), Box::new(move |t| d.pdf(t)), Box::new(move |p| d.inverse_cdf(p)))
        }
        FitDist::Exponential => {
            if x.iter().any(|v| *v < 0.0) || !(m > 0.0) {
                return invalid("la exponencial necesita datos no negativos");
            }
            let d = Exp::new(1.0 / m).unwrap();
            mk(format!("Exponencial(media = {})", fmt(tidy(m))), 1, Box::new(move |t| d.cdf(t)), Box::new(move |t| d.pdf(t)), Box::new(move |p| d.inverse_cdf(p)))
        }
        FitDist::Uniform => {
            let (a, b) = (x.iter().cloned().fold(f64::INFINITY, f64::min), x.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
            if !(b > a) {
                return invalid("los datos no varían");
            }
            let d = Uniform::new(a, b).unwrap();
            mk(format!("Uniforme({}, {})", fmt(tidy(a)), fmt(tidy(b))), 2, Box::new(move |t| d.cdf(t)), Box::new(move |t| d.pdf(t)), Box::new(move |p| a + p * (b - a)))
        }
        FitDist::Gamma => {
            if x.iter().any(|v| *v <= 0.0) || !(s > 0.0) {
                return invalid("la gamma necesita datos positivos que varíen");
            }
            let k = m * m / (s * s);
            let th = s * s / m;
            let d = Gamma::new(k, 1.0 / th).unwrap();
            mk(format!("Gamma(forma = {}, escala = {})", fmt(tidy(k)), fmt(tidy(th))), 2, Box::new(move |t| d.cdf(t)), Box::new(move |t| d.pdf(t)), Box::new(move |p| d.inverse_cdf(p)))
        }
        FitDist::Weibull => {
            if x.iter().any(|v| *v <= 0.0) || !(s > 0.0) {
                return invalid("la Weibull necesita datos positivos que varíen");
            }
            // máxima verosimilitud para la forma k (bisección)
            let lx: Vec<f64> = x.iter().map(|v| v.ln()).collect();
            let ml = mean(&lx);
            let g = |k: f64| {
                let a: f64 = x.iter().map(|v| v.powf(k) * v.ln()).sum();
                let b: f64 = x.iter().map(|v| v.powf(k)).sum();
                a / b - 1.0 / k - ml
            };
            let (mut lo, mut hi) = (0.01f64, 100.0f64);
            for _ in 0..200 {
                let mid = (lo * hi).sqrt();
                if g(mid) > 0.0 {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            let k = (lo * hi).sqrt();
            let lam = (x.iter().map(|v| v.powf(k)).sum::<f64>() / x.len() as f64).powf(1.0 / k);
            let d = Weibull::new(k, lam).unwrap();
            mk(format!("Weibull(forma = {}, escala = {})", fmt(tidy(k)), fmt(tidy(lam))), 2, Box::new(move |t| d.cdf(t)), Box::new(move |t| d.pdf(t)), Box::new(move |p| lam * (-(1.0 - p).ln()).powf(1.0 / k)))
        }
    })
}

/// Histograma con prueba ji cuadrada de bondad de ajuste.
fn histogram(x: &[f64], o: &QccOptions, title: &str) -> Result<(Vec<Block>, Value, Fit), ModError> {
    let n = x.len();
    if n < 3 {
        return invalid("el histograma necesita al menos 3 datos");
    }
    let f = fit(o.distribution, x)?;
    let k = if o.cells > 0 { o.cells as usize } else { ((n as f64).sqrt().round() as usize).clamp(2, 20) };
    let (mut lo, mut hi) = (x.iter().cloned().fold(f64::INFINITY, f64::min), x.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
    if !(hi > lo) {
        return invalid("los datos no varían");
    }
    let (dmin, dmax) = (lo, hi);
    if o.combine_3sigma {
        let (mu, s) = (mean(x), sd(x));
        lo = lo.max(mu - 3.0 * s);
        hi = hi.min(mu + 3.0 * s);
    }
    let wd = (hi - lo) / k as f64;
    let mut obs = vec![0usize; k];
    for v in x {
        let i = (((v - lo) / wd).floor() as isize).clamp(0, k as isize - 1) as usize;
        obs[i] += 1;
    }
    let edges: Vec<f64> = (0..=k).map(|i| lo + wd * i as f64).collect();
    let mut t = Table::new(Some("Frecuencias y prueba ji cuadrada"), &["Celda", "Desde", "Hasta", "Observada", "Esperada", "(O − E)²/E"]).left(0);
    let mut q = 0.0;
    let mut bars = vec![];
    let mut curve = vec![];
    for i in 0..k {
        let a = if i == 0 { 0.0 } else { (f.cdf)(edges[i]) };
        let b = if i + 1 == k { 1.0 } else { (f.cdf)(edges[i + 1]) };
        let e = (b - a) * n as f64;
        let c = if e > 0.0 { (obs[i] as f64 - e).powi(2) / e } else { 0.0 };
        q += c;
        t.row(cells![format!("{}", i + 1), tidy(edges[i]), tidy(edges[i + 1]), obs[i] as f64, tidy(e), tidy(c)]);
        bars.push([0.5 * (edges[i] + edges[i + 1]), obs[i] as f64]);
    }
    for s in 0..=120 {
        let xv = lo - wd + (hi - lo + 2.0 * wd) * s as f64 / 120.0;
        curve.push([xv, (f.pdf)(xv) * n as f64 * wd]);
    }
    let df = k as i64 - 1 - f.params as i64;
    let mut blocks = vec![];
    let ch = Chart::new(Some(title), "Valor", "Frecuencia").series(Series::new("Observada", SeriesKind::Bar, bars).tone(Tone::Accent)).series(Series::new(f.name.clone(), SeriesKind::Line, curve).tone(Tone::Highlight));
    blocks.push(ch.block());
    // momentos
    let m = mean(x);
    let m2 = x.iter().map(|v| (v - m).powi(2)).sum::<f64>() / n as f64;
    let m3 = x.iter().map(|v| (v - m).powi(3)).sum::<f64>() / n as f64;
    let m4 = x.iter().map(|v| (v - m).powi(4)).sum::<f64>() / n as f64;
    let skew = if m2 > 0.0 { m3 / m2 / m2.sqrt() } else { 0.0 };
    let kurt = if m2 > 0.0 { m4 / m2 / m2 - 3.0 } else { 0.0 };
    let mut st = Table::new(Some("Estadísticos de los datos"), &["Medida", "Valor"]).left(0);
    st.row(cells!["Datos", n as f64]);
    st.row(cells!["Media", tidy(m)]);
    st.row(cells!["Desviación estándar", tidy(sd(x))]);
    st.row(cells!["Mediana", tidy(median(x))]);
    st.row(cells!["Mínimo", tidy(dmin)]);
    st.row(cells!["Máximo", tidy(dmax)]);
    st.row(cells!["Asimetría M3/M2^1.5", tidy(skew)]);
    st.row(cells!["Curtosis M4/M2² − 3", tidy(kurt)]);
    let data = if df >= 1 {
        let chi = ChiSquared::new(df as f64).unwrap();
        let crit = chi.inverse_cdf(1.0 - o.significance);
        let pv = 1.0 - chi.cdf(q);
        let reject = q > crit;
        blocks.push(text_tone(
            format!(
                "Prueba ji cuadrada para {}: q = {}, crítico χ²({}, {} gl) = {}, valor p = {}. {}",
                f.name,
                fmt(tidy(q)),
                fmt(o.significance),
                df,
                fmt(tidy(crit)),
                fmt(tidy(pv)),
                if reject { "Se rechaza la distribución." } else { "No se rechaza la distribución." }
            ),
            if reject { Tone::Bad } else { Tone::Good },
        ));
        json!({"chi_square": tidy(q), "df": df, "critical": tidy(crit), "p_value": tidy(pv), "reject": reject, "skewness": tidy(skew), "kurtosis": tidy(kurt)})
    } else {
        blocks.push(text("Hay muy pocas celdas para la prueba ji cuadrada (los grados de libertad serían menos de 1)."));
        json!({"skewness": tidy(skew), "kurtosis": tidy(kurt)})
    };
    blocks.push(t.block());
    blocks.push(st.block());
    Ok((blocks, data, f))
}

fn prob_plot(x: &[f64], f: &Fit) -> (Vec<Block>, f64) {
    let mut s = x.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = s.len();
    let pts: Vec<[f64; 2]> = s.iter().enumerate().map(|(i, v)| [*v, (f.inv)((i as f64 + 0.5) / n as f64)]).collect();
    let (lo, hi) = (s[0], s[n - 1]);
    let xs: Vec<f64> = pts.iter().map(|p| p[0]).collect();
    let ys: Vec<f64> = pts.iter().map(|p| p[1]).collect();
    let (mx, my) = (mean(&xs), mean(&ys));
    let sxy: f64 = xs.iter().zip(&ys).map(|(a, b)| (a - mx) * (b - my)).sum();
    let r = sxy / (xs.iter().map(|a| (a - mx).powi(2)).sum::<f64>().sqrt() * ys.iter().map(|b| (b - my).powi(2)).sum::<f64>().sqrt());
    let mut t = Table::new(Some("Datos ordenados"), &["i", "x(i)", "p(i) = (i − 0.5)/n", "y(i) = F⁻¹(p(i))"]);
    for (i, p) in pts.iter().enumerate().take(200) {
        t.row(cells![(i + 1) as f64, tidy(p[0]), tidy((i as f64 + 0.5) / n as f64), tidy(p[1])]);
    }
    let ch = Chart::new(Some(&format!("Gráfico de probabilidad: {}", f.name)), "Dato x(i)", "Cuantil teórico y(i)").series(Series::new("Datos", SeriesKind::Scatter, pts).tone(Tone::Accent)).series(Series::new("Recta de 45°", SeriesKind::Line, vec![[lo, lo], [hi, hi]]).tone(Tone::Muted).dashed());
    (vec![ch.block(), text(format!("Correlación entre los datos y los cuantiles teóricos: r = {}. Si los puntos siguen la recta de 45°, la distribución es adecuada.", fmt(tidy(r)))), t.block()], r)
}

fn capability(x: &[f64], mu: f64, sigma: f64, ch: &Characteristic) -> Option<(Vec<Block>, Value)> {
    let (l, u) = (ch.lsl, ch.usl);
    if l.is_none() && u.is_none() {
        return None;
    }
    let mut t = Table::new(Some("Capacidad del proceso"), &["Índice", "Valor"]).left(0);
    let mut data = json!({});
    if let (Some(l), Some(u)) = (l, u) {
        let pcr = (u - l) / (6.0 * sigma);
        let tg = ch.target.unwrap_or(0.5 * (l + u));
        let tau = (sigma * sigma + (mu - tg).powi(2)).sqrt();
        let pcrkm = (u - l) / (6.0 * tau);
        t.row(cells!["PCR = (USL − LSL)/6σ", tidy(pcr)]);
        t.row(cells!["PCRkm = (USL − LSL)/6τ, τ² = σ² + (μ − T)²", tidy(pcrkm)]);
        data["pcr"] = json!(tidy(pcr));
        data["pcrkm"] = json!(tidy(pcrkm));
    }
    let pu = u.map(|u| (u - mu) / (3.0 * sigma));
    let pl = l.map(|l| (mu - l) / (3.0 * sigma));
    if let Some(v) = pu {
        t.row(cells!["PCR-U = (USL − μ)/3σ", tidy(v)]);
    }
    if let Some(v) = pl {
        t.row(cells!["PCR-L = (μ − LSL)/3σ", tidy(v)]);
    }
    let pk = match (pu, pl) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) | (None, Some(a)) => a,
        _ => f64::NAN,
    };
    t.row(cells!["PCRk = mín(PCR-U, PCR-L)", tidy(pk)]);
    let out = l.map(|l| phi((l - mu) / sigma)).unwrap_or(0.0) + u.map(|u| 1.0 - phi((u - mu) / sigma)).unwrap_or(0.0);
    t.row(cells!["Fuera de especificación esperado (%)", tidy(100.0 * out)]);
    t.row(cells!["Partes por millón fuera de especificación", tidy(1e6 * out)]);
    let obs_out = x.iter().filter(|v| l.map(|l| **v < l).unwrap_or(false) || u.map(|u| **v > u).unwrap_or(false)).count();
    t.row(cells!["Datos fuera de especificación (observados)", obs_out as f64]);
    data["pcrk"] = json!(tidy(pk));
    data["out_of_spec"] = json!(tidy(out));
    // curva normal con las especificaciones
    let lo = l.unwrap_or(mu - 4.0 * sigma).min(mu - 4.0 * sigma);
    let hi = u.unwrap_or(mu + 4.0 * sigma).max(mu + 4.0 * sigma);
    let nd = Normal::new(mu, sigma).unwrap();
    let curve: Vec<[f64; 2]> = (0..=160).map(|i| {
        let xv = lo + (hi - lo) * i as f64 / 160.0;
        [xv, nd.pdf(xv)]
    }).collect();
    let mut ch2 = Chart::new(Some("Distribución del proceso y especificaciones"), "Valor", "Densidad").series(Series::new(format!("Normal(μ = {}, σ = {})", fmt(tidy(mu)), fmt(tidy(sigma))), SeriesKind::Line, curve).tone(Tone::Accent));
    if let Some(l) = l {
        ch2 = ch2.vline(l, "LSL", Tone::Bad);
    }
    if let Some(u) = u {
        ch2 = ch2.vline(u, "USL", Tone::Bad);
    }
    if let Some(tg) = ch.target {
        ch2 = ch2.vline(tg, "Objetivo", Tone::Good);
    }
    ch2.y_zero = true;
    let verdict = if pk >= 1.33 {
        ("El proceso es capaz (PCRk ≥ 1.33).", Tone::Good)
    } else if pk >= 1.0 {
        ("El proceso es apenas capaz (1 ≤ PCRk < 1.33).", Tone::Accent)
    } else {
        ("El proceso no es capaz (PCRk < 1): produce fuera de especificación.", Tone::Bad)
    };
    Some((vec![metrics(vec![metric_main("PCRk", tidy(pk)), metric("Fuera de especificación (%)", tidy(100.0 * out))]), text_tone(verdict.0, verdict.1), t.block(), ch2.block()], data))
}

fn oc_curve(oc: &Oc) -> Vec<Block> {
    let mut t;
    let pts: Vec<[f64; 2]>;
    let xl;
    match oc {
        Oc::Mean { ku, kl, scale } => {
            xl = "Corrimiento de la media (en σ del proceso)";
            let top = ((ku + kl) / scale.max(1e-9) * 1.2).max(0.5);
            pts = (0..=60).map(|i| {
                let k = top * i as f64 / 60.0;
                [k, phi(ku - k * scale) - phi(-kl - k * scale)]
            }).collect();
        }
        Oc::Spread { cl, ucl, lcl, rel_sd } => {
            xl = "Razón σ₁/σ₀";
            pts = (0..=60).map(|i| {
                let l = 1.0 + 4.0 * i as f64 / 60.0;
                let (m, s) = (cl * l, rel_sd * cl * l);
                [l, phi((ucl - m) / s) - phi((lcl - m) / s)]
            }).collect();
        }
        Oc::Variance { n, ucl, lcl, s2 } => {
            xl = "Razón σ₁/σ₀";
            let df = (n - 1.0).max(1.0);
            let c = ChiSquared::new(df).unwrap();
            pts = (0..=60).map(|i| {
                let l = 1.0 + 4.0 * i as f64 / 60.0;
                [l, c.cdf(df * ucl / (s2 * l * l)) - c.cdf(df * lcl / (s2 * l * l))]
            }).collect();
        }
        Oc::Binomial { n, ucl, lcl, p, count } => {
            xl = "Proporción defectuosa del proceso";
            let nn = n.max(1.0) as u64;
            let (hi, lo) = if *count { (*ucl, *lcl) } else { (ucl * *n, lcl * *n) };
            let top = (p * 4.0).max(0.05).min(1.0);
            pts = (0..=60).map(|i| {
                let q = (top * i as f64 / 60.0).min(1.0);
                let b = Binomial::new(q, nn).unwrap();
                let up = if hi.floor() >= 0.0 { b.cdf(hi.floor() as u64) } else { 0.0 };
                let dn = if lo.ceil() >= 1.0 { b.cdf(lo.ceil() as u64 - 1) } else { 0.0 };
                [q, (up - dn).max(0.0)]
            }).collect();
        }
        Oc::Poisson { n, ucl, lcl, c, per_unit } => {
            xl = if *per_unit { "Defectos por unidad del proceso" } else { "Defectos medios del proceso" };
            let (hi, lo) = if *per_unit { (ucl * n, lcl * n) } else { (*ucl, *lcl) };
            let top = (c * 3.0).max(1.0);
            pts = (0..=60).map(|i| {
                let q = (top * i as f64 / 60.0).max(1e-9);
                let lam = if *per_unit { q * n } else { q };
                let d = Poisson::new(lam).unwrap();
                let up = if hi.floor() >= 0.0 { d.cdf(hi.floor() as u64) } else { 0.0 };
                let dn = if lo.ceil() >= 1.0 { d.cdf(lo.ceil() as u64 - 1) } else { 0.0 };
                [q, (up - dn).max(0.0)]
            }).collect();
        }
    }
    t = Table::new(Some("Probabilidad de no detectar el cambio en la siguiente muestra"), &[xl, "β", "ARL = 1/(1 − β)"]);
    for p in pts.iter().step_by(5) {
        t.row(cells![tidy(p[0]), tidy(p[1]), if p[1] < 1.0 { Cell::Num(tidy(1.0 / (1.0 - p[1]))) } else { Cell::Empty }]);
    }
    let ch = Chart::new(Some("Curva característica de operación (OC)"), xl, "β = P(no detectar)").series(Series::new("OC", SeriesKind::Line, pts).tone(Tone::Accent));
    vec![ch.block(), t.block()]
}

/// ¿La primera característica es el total de las demás? (como en el ejemplo PARETO de WinQSB)
fn total_column(m: &QccModel) -> bool {
    let k = m.characteristics.len();
    k >= 3 && m.data.iter().all(|r| {
        let v: Vec<f64> = r.iter().map(|x| x.unwrap_or(0.0)).collect();
        (v[0] - v[1..].iter().sum::<f64>()).abs() < 1e-9
    }) && m.data.iter().any(|r| r[0].unwrap_or(0.0) != 0.0)
}

fn pareto(items: Vec<(String, f64)>, title: &str) -> Vec<Block> {
    let mut items: Vec<(String, f64)> = items.into_iter().filter(|x| x.1 > 0.0).collect();
    items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let tot: f64 = items.iter().map(|x| x.1).sum();
    if tot <= 0.0 {
        return vec![text("No hay nada que contar para el análisis de Pareto.")];
    }
    let mut acc = 0.0;
    let mut t = Table::new(Some(title), &["Categoría", "Cantidad", "%", "% acumulado"]).left(0);
    let mut bars = vec![];
    let mut cum = vec![];
    for (i, (n, v)) in items.iter().enumerate() {
        acc += v;
        t.row_tone(cells![n.clone(), *v, tidy(100.0 * v / tot), tidy(100.0 * acc / tot)], if acc / tot <= 0.8 + 1e-9 || i == 0 { Tone::Highlight } else { Tone::Normal });
        bars.push([i as f64, 100.0 * v / tot]);
        cum.push([i as f64, 100.0 * acc / tot]);
    }
    let few = items.iter().scan(0.0, |a, x| {
        *a += x.1;
        Some(*a)
    }).position(|a| a / tot >= 0.8 - 1e-9).map(|p| p + 1).unwrap_or(items.len());
    let mut ch = Chart::new(Some(title), "Categoría", "%").series(Series::new("% del total", SeriesKind::Bar, bars).tone(Tone::Accent)).series(Series::new("% acumulado", SeriesKind::Line, cum).tone(Tone::Highlight)).hline(80.0, "80 %", Tone::Muted);
    ch.categories = Some(items.iter().map(|x| x.0.clone()).collect());
    ch.y_zero = true;
    vec![text(format!("{} de {} categorías explican el 80 % del total ({}).", few, items.len(), fmt(tot))), ch.block(), t.block()]
}

fn auto_chart(m: &QccModel, c: usize) -> ChartKind {
    if m.problem_type == QccType::Variable {
        let subs = chart::subgroups(m, c);
        if subs.iter().any(|s| s.vals.len() >= 2) {
            ChartKind::Xbar
        } else {
            ChartKind::X
        }
    } else {
        let v: Vec<f64> = m.data.iter().filter_map(|r| r.get(c).cloned().flatten()).collect();
        let props = v.iter().all(|x| (0.0..=1.0).contains(x)) && v.iter().any(|x| x.fract() != 0.0);
        if props {
            ChartKind::P
        } else {
            ChartKind::C
        }
    }
}

fn chart_block(p: &Plot, viol: &[Vec<usize>], o: &QccOptions, xl: &str) -> Vec<Block> {
    let len = p.w.len();
    let x = |i: usize| (i + 1) as f64;
    let constant = |v: &[f64]| v.iter().filter(|x| x.is_finite()).all(|y| (y - v[0]).abs() <= 1e-9 * v[0].abs().max(1.0)) && v.first().map(|y| y.is_finite()).unwrap_or(false);
    let mut ch = Chart::new(Some(&p.title), xl, &p.ylabel).series(Series::new(p.ylabel.clone(), SeriesKind::Line, (0..len).map(|i| [x(i), p.w[i]]).collect()).tone(Tone::Accent));
    let bad: Vec<[f64; 2]> = (0..len).filter(|&i| !viol[i].is_empty()).map(|i| [x(i), p.w[i]]).collect();
    let off: Vec<[f64; 2]> = (0..len).filter(|&i| !p.enabled[i] && p.w[i].is_finite()).map(|i| [x(i), p.w[i]]).collect();
    if !bad.is_empty() {
        ch = ch.series(Series::new("Fuera de control", SeriesKind::Scatter, bad).tone(Tone::Bad));
    }
    if !off.is_empty() {
        ch = ch.series(Series::new("Deshabilitados", SeriesKind::Scatter, off).tone(Tone::Muted));
    }
    let lines: [(&str, &Vec<f64>, Tone); 3] = [("LCS", &p.ucl, Tone::Bad), ("LC", &p.cl, Tone::Good), ("LCI", &p.lcl, Tone::Bad)];
    for (name, v, tone) in lines {
        if v.iter().all(|y| !y.is_finite()) {
            continue;
        }
        if constant(v) {
            ch = ch.hline(v[0], name, tone);
        } else {
            ch = ch.series(Series::new(name, SeriesKind::Line, (0..len).map(|i| [x(i), v[i]]).collect()).tone(tone).dashed());
        }
    }
    for (show, k) in [(o.warning2, 2.0), (o.warning1, 1.0)] {
        if !show || p.cusum.is_some() || p.sd.iter().all(|s| !s.is_finite()) {
            continue;
        }
        let up: Vec<f64> = (0..len).map(|i| p.cl[i] + k * p.sd[i]).collect();
        let dn: Vec<f64> = (0..len).map(|i| p.cl[i] - k * p.sd[i]).collect();
        if constant(&up) && constant(&dn) {
            ch = ch.hline(up[0], format!("+{k}σ"), Tone::Muted).hline(dn[0], format!("−{k}σ"), Tone::Muted);
        } else {
            ch = ch.series(Series::new(format!("±{k}σ"), SeriesKind::Line, (0..len).map(|i| [x(i), up[i]]).collect()).tone(Tone::Muted).dashed());
            ch = ch.series(Series::new(format!("−{k}σ"), SeriesKind::Line, (0..len).map(|i| [x(i), dn[i]]).collect()).tone(Tone::Muted).dashed());
        }
    }
    let mut out = vec![ch.block()];
    if let Some((zh, zl, h)) = &p.cusum {
        let c2 = Chart::new(Some("CUSUM tabular (en desviaciones)"), xl, "Suma acumulada")
            .series(Series::new("ZH (cambio hacia arriba)", SeriesKind::Line, zh.iter().enumerate().map(|(i, v)| [x(i), *v]).collect()).tone(Tone::Accent))
            .series(Series::new("ZL (cambio hacia abajo)", SeriesKind::Line, zl.iter().enumerate().map(|(i, v)| [x(i), *v]).collect()).tone(Tone::Highlight))
            .hline(*h, "H", Tone::Bad);
        out.push(c2.block());
    }
    out
}

pub fn solve(m: &QccModel, o: &QccOptions) -> Result<Report, ModError> {
    let k = m.characteristics.len();
    if k == 0 {
        return invalid("hace falta al menos una característica");
    }
    if m.data.len() != m.rows.len() {
        return invalid(format!("hay {} filas de datos para {} filas", m.data.len(), m.rows.len()));
    }
    let mut mm = m.clone();
    for r in mm.data.iter_mut() {
        r.resize(k, None);
    }
    let m = &mm;
    let c = (o.characteristic.max(1) as usize - 1).min(k - 1);
    let kind = o.chart.unwrap_or_else(|| auto_chart(m, c));
    if kind.attribute() != (m.problem_type == QccType::Attribute) {
        return invalid(format!("el gráfico {} es para datos por {}", kind.label(), if kind.attribute() { "atributos" } else { "variables" }));
    }
    let mut p = chart::build(m, o, kind, c)?;
    if o.limits == Limits::Preset && p.cusum.is_none() {
        let len = p.w.len();
        if let Some(u) = o.ucl {
            p.ucl = vec![u; len];
        }
        if let Some(l) = o.lcl {
            p.lcl = vec![l; len];
        }
    }
    let len = p.w.len();
    let viol: Vec<Vec<usize>> = match &p.cusum {
        Some((zh, zl, h)) => (0..len).map(|i| if p.enabled[i] && (zh[i] > *h || zl[i] > *h) { vec![usize::MAX] } else { vec![] }).collect(),
        None if o.check_rules => chart::rules(&p, &m.rules),
        None => (0..len).map(|i| if p.enabled[i] && p.w[i].is_finite() && ((p.ucl[i].is_finite() && p.w[i] > p.ucl[i]) || (p.lcl[i].is_finite() && p.w[i] < p.lcl[i])) { vec![0] } else { vec![] }).collect(),
    };
    let rule_name = |r: usize| if r == usize::MAX { "CUSUM fuera del intervalo de decisión".to_string() } else { format!("{}. {}", r + 1, m.rules.get(r).map(|x| x.description.as_str()).unwrap_or("")) };
    let xl = if m.problem_type == QccType::Variable && !matches!(kind, ChartKind::X | ChartKind::CusumX | ChartKind::TrendX | ChartKind::GmaX | ChartKind::MaX | ChartKind::ModifiedX | ChartKind::AcceptanceX) { "Subgrupo" } else if m.problem_type == QccType::Variable { "Observación" } else { "Subgrupo" };
    let mut graf = chart_block(&p, &viol, o, xl);
    // gráfico R (o S) que acompaña al X̄
    if kind == ChartKind::Xbar && o.chart.is_none() {
        let nmax = chart::subgroups(m, c).iter().map(|s| s.vals.len()).max().unwrap_or(1);
        let kk = if nmax > 10 { ChartKind::S } else { ChartKind::R };
        if let Ok(p2) = chart::build(m, o, kk, c) {
            let v2 = if o.check_rules { chart::rules(&p2, &m.rules) } else { vec![vec![]; p2.w.len()] };
            graf.extend(chart_block(&p2, &v2, o, xl));
            let n2 = v2.iter().filter(|v| !v.is_empty()).count();
            if n2 > 0 {
                graf.push(text_tone(format!("En el gráfico {} hay {} puntos fuera de control.", kk.label(), n2), Tone::Bad));
            }
        }
    }
    let out: Vec<usize> = (0..len).filter(|&i| !viol[i].is_empty()).collect();
    // tabla de puntos
    let mut t = Table::new(Some("Datos del gráfico"), &[xl, &p.ylabel, "LCI", "LC", "LCS", "Reglas violadas"]).left(0).left(5);
    for i in 0..len {
        let f = |v: f64| if v.is_finite() { Cell::Num(tidy(v)) } else { Cell::Empty };
        let rv = if !p.enabled[i] { "deshabilitado".to_string() } else { viol[i].iter().map(|r| if *r == usize::MAX { "CUSUM".to_string() } else { (r + 1).to_string() }).collect::<Vec<_>>().join(", ") };
        t.row_tone(cells![p.labels[i].clone(), f(p.w[i]), f(p.lcl[i]), f(p.cl[i]), f(p.ucl[i]), rv], if !viol[i].is_empty() { Tone::Bad } else if !p.enabled[i] { Tone::Muted } else { Tone::Normal });
    }
    let mut et = Table::new(Some("Estimación"), &["Parámetro", "Valor"]).left(0);
    for (a, b) in &p.est {
        et.row(cells![a.clone(), tidy(*b)]);
    }
    let first = |v: &[f64]| v.iter().cloned().find(|x| x.is_finite()).unwrap_or(f64::NAN);
    let mut mets = vec![metric_main("Puntos fuera de control", out.len() as f64)];
    if p.cusum.is_none() {
        mets.push(metric("LC", tidy(first(&p.cl))));
        if first(&p.ucl).is_finite() {
            mets.push(metric("LCS", tidy(first(&p.ucl))));
        }
        if first(&p.lcl).is_finite() {
            mets.push(metric("LCI", tidy(first(&p.lcl))));
        }
    }
    if p.sigma.is_finite() {
        mets.push(metric("σ del proceso", tidy(p.sigma)));
    }
    let mut sol = vec![metrics(mets), text(p.formula.clone())];
    if out.is_empty() {
        sol.push(text_tone("Todos los puntos están dentro de los límites y no se viola ninguna regla: el proceso está bajo control estadístico.", Tone::Good));
    } else {
        sol.push(text_tone(format!("Fuera de control en {} {}: {}.", out.len(), if out.len() == 1 { "punto" } else { "puntos" }, out.iter().take(30).map(|&i| p.labels[i].clone()).collect::<Vec<_>>().join(", ")), Tone::Bad));
    }
    sol.push(et.clone().block());
    sol.push(t.block());
    // reglas
    let mut rule_blocks = vec![];
    let mut counts: Vec<(String, f64)> = vec![];
    if !out.is_empty() {
        let mut rt = Table::new(Some("Reglas violadas"), &[xl, "Regla"]).left(0).left(1);
        for &i in &out {
            for r in &viol[i] {
                rt.row(cells![p.labels[i].clone(), rule_name(*r)]);
                match counts.iter_mut().find(|x| x.0 == rule_name(*r)) {
                    Some(x) => x.1 += 1.0,
                    None => counts.push((rule_name(*r), 1.0)),
                }
            }
        }
        rule_blocks.push(rt.block());
    } else {
        rule_blocks.push(text("No se viola ninguna regla."));
    }
    let mut rl = Table::new(Some("Reglas revisadas"), &["Número", "Regla"]).left(1);
    for (i, r) in m.rules.iter().enumerate() {
        if r.n_of > 0.0 {
            rl.row(cells![(i + 1) as f64, r.description.clone()]);
        }
    }
    rule_blocks.push(rl.block());
    // datos para las herramientas
    let ch = &m.characteristics[c];
    let xs: Vec<f64> = if m.problem_type == QccType::Variable { chart::individuals(m, c).into_iter().filter(|x| x.2).map(|x| x.1).collect() } else { (0..len).filter(|&i| p.enabled[i] && p.w[i].is_finite()).map(|i| p.w[i]).collect() };
    let mut r = Report::new("qcc", &m.name, Status::Solved, if out.is_empty() { format!("{}: proceso bajo control estadístico ({} puntos).", kind.label(), len) } else { format!("{}: {} de {} puntos fuera de control.", kind.label(), out.len(), len) })
        .section("grafico", "Gráfico", graf)
        .section("solucion", "Solución", sol)
        .section("reglas", "Reglas", rule_blocks);
    let mut data = json!({
        "chart": CHARTS.iter().find(|x| x.0 == kind).unwrap().1, "characteristic": ch.name,
        "cl": tidy(first(&p.cl)), "ucl": tidy(first(&p.ucl)), "lcl": tidy(first(&p.lcl)),
        "sigma": if p.sigma.is_finite() { json!(tidy(p.sigma)) } else { Value::Null },
        "mean": if p.mu.is_finite() { json!(tidy(p.mu)) } else { Value::Null },
        "out_of_control": out.iter().map(|&i| p.labels[i].clone()).collect::<Vec<_>>(),
        "points": len,
        "estimates": json_rows(&p.est),
    });
    if m.problem_type == QccType::Variable && p.sigma.is_finite() {
        if let Some((b, d)) = capability(&xs, p.mu, p.sigma, ch) {
            r = r.section("capacidad", "Capacidad", b);
            data["capability"] = d;
        }
    }
    if let Some(oc) = &p.oc {
        r = r.section("oc", "Curva OC", oc_curve(oc));
    }
    if o.histogram || o.probability_plot {
        match histogram(&xs, o, &format!("Histograma — {}", if m.problem_type == QccType::Variable { ch.name.clone() } else { p.ylabel.clone() })) {
            Ok((b, d, f)) => {
                if o.histogram {
                    r = r.section("histograma", "Histograma", b);
                    data["histogram"] = d;
                }
                if o.probability_plot {
                    let (b2, rr) = prob_plot(&xs, &f);
                    r = r.section("probabilidad", "Probabilidad", b2);
                    data["probability_plot_r"] = json!(tidy(rr));
                }
            }
            Err(e) => {
                r = r.section("histograma", "Histograma", vec![text_tone(e.to_string(), Tone::Accent)]);
            }
        }
    }
    if o.pareto {
        let has_codes = |f: &dyn Fn(&DataRow) -> Option<f64>| m.rows.iter().any(|r| f(r).is_some());
        let source = match o.pareto_source {
            ParetoSource::Auto => {
                if m.problem_type == QccType::Attribute && k >= 2 {
                    Some(ParetoSource::Characteristics)
                } else if has_codes(&|r| r.cause) {
                    Some(ParetoSource::Causes)
                } else if has_codes(&|r| r.action) {
                    Some(ParetoSource::Actions)
                } else if !counts.is_empty() {
                    Some(ParetoSource::Rules)
                } else {
                    None
                }
            }
            s => Some(s),
        };
        let blocks = match source {
            Some(ParetoSource::Characteristics) => {
                let skip = total_column(m);
                let items: Vec<(String, f64)> = (0..k)
                    .filter(|&j| !(skip && j == 0))
                    .map(|j| {
                        let v: f64 = if m.problem_type == QccType::Attribute {
                            m.rows.iter().enumerate().filter(|(_, r)| !r.disabled).map(|(i, _)| m.data[i][j].unwrap_or(0.0)).sum()
                        } else {
                            let c2 = &m.characteristics[j];
                            chart::individuals(m, j).into_iter().filter(|x| x.2 && (c2.lsl.map(|l| x.1 < l).unwrap_or(false) || c2.usl.map(|u| x.1 > u).unwrap_or(false))).count() as f64
                        };
                        (m.characteristics[j].name.clone(), v)
                    })
                    .collect();
                let mut b = pareto(items, "Pareto de las características");
                if skip {
                    b.insert(0, text(format!("«{}» es el total de las demás características y se deja fuera.", m.characteristics[0].name)));
                }
                b
            }
            Some(ParetoSource::Causes) | Some(ParetoSource::Actions) => {
                let causes = source == Some(ParetoSource::Causes);
                let names = if causes { &m.causes } else { &m.actions };
                let mut items: Vec<(String, f64)> = vec![];
                for row in &m.rows {
                    if let Some(code) = if causes { row.cause } else { row.action } {
                        let i = code as usize;
                        let name = names.get(i.wrapping_sub(1)).cloned().unwrap_or(format!("Código {i}"));
                        match items.iter_mut().find(|x| x.0 == name) {
                            Some(x) => x.1 += 1.0,
                            None => items.push((name, 1.0)),
                        }
                    }
                }
                pareto(items, if causes { "Pareto de las causas" } else { "Pareto de las acciones" })
            }
            Some(ParetoSource::Rules) => pareto(counts.clone(), "Pareto de las reglas violadas"),
            _ => vec![],
        };
        if !blocks.is_empty() {
            r = r.section("pareto", "Pareto", blocks);
        }
    }
    if o.steps {
        let mut v = vec![];
        v.push(step("Estimación", "Se calculan la línea central y la desviación del proceso con los subgrupos habilitados (los deshabilitados no cuentan para los límites).", vec![et.clone().block()]));
        v.push(step("Límites de control", p.formula.clone(), vec![]));
        if p.cusum.is_none() {
            v.push(step(
                "Reglas",
                "Cada punto se compara con los límites y con las zonas de 1 y 2 desviaciones: un punto fuera de los límites, 2 de 3 más allá de 2σ, 4 de 5 más allá de 1σ, 8 seguidos del mismo lado de la línea central o de la mediana, 8 seguidos subiendo o bajando, o un salto grande entre dos puntos indican causas asignables.",
                vec![],
            ));
        }
        r = r.section("pasos", "Paso a paso", vec![steps(v)]);
    }
    Ok(r.data(data))
}
