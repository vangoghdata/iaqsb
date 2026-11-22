//! Gráficos de control: estadísticos, estimación de la línea central y de la
//! desviación, límites (k desviaciones, probabilidad o valores dados) y reglas.

use super::consts::consts;
use super::*;
use statrs::distribution::{ChiSquared, ContinuousCDF, Normal};

fn zq(p: f64) -> f64 {
    Normal::new(0.0, 1.0).unwrap().inverse_cdf(p.clamp(1e-15, 1.0 - 1e-15))
}
fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        f64::NAN
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}
pub fn median(v: &[f64]) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = s.len();
    if n % 2 == 1 {
        s[n / 2]
    } else {
        0.5 * (s[n / 2 - 1] + s[n / 2])
    }
}
pub fn sd(v: &[f64]) -> f64 {
    if v.len() < 2 {
        return 0.0;
    }
    let m = mean(v);
    (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (v.len() - 1) as f64).sqrt()
}

/// Un subgrupo (variables) o fila (atributos).
#[derive(Clone, Debug)]
pub struct Sub {
    pub label: String,
    pub vals: Vec<f64>,
    pub enabled: bool,
    pub size: f64,
    pub cause: Option<f64>,
    pub action: Option<f64>,
}

impl Sub {
    fn n(&self) -> usize {
        self.vals.len()
    }
    fn mean(&self) -> f64 {
        mean(&self.vals)
    }
    fn range(&self) -> f64 {
        let mx = self.vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let mn = self.vals.iter().cloned().fold(f64::INFINITY, f64::min);
        mx - mn
    }
    fn sd(&self) -> f64 {
        sd(&self.vals)
    }
    fn median(&self) -> f64 {
        median(&self.vals)
    }
    fn midrange(&self) -> f64 {
        let mx = self.vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let mn = self.vals.iter().cloned().fold(f64::INFINITY, f64::min);
        0.5 * (mx + mn)
    }
}

/// Subgrupos de una característica de datos por variables (en orden de aparición).
pub fn subgroups(m: &QccModel, c: usize) -> Vec<Sub> {
    let mut out: Vec<(f64, Sub)> = vec![];
    for (i, r) in m.rows.iter().enumerate() {
        let sg = r.subgroup.unwrap_or((i + 1) as f64);
        let pos = match out.iter().position(|(s, _)| *s == sg) {
            Some(p) => p,
            None => {
                out.push((sg, Sub { label: fmt(sg), vals: vec![], enabled: false, size: 0.0, cause: r.cause, action: r.action }));
                out.len() - 1
            }
        };
        if let Some(v) = m.data.get(i).and_then(|d| d.get(c)).cloned().flatten() {
            if !r.disabled {
                out[pos].1.vals.push(v);
                out[pos].1.enabled = true;
            }
        }
    }
    out.into_iter().map(|(_, s)| s).collect()
}

/// Observaciones individuales de una característica (en orden).
pub fn individuals(m: &QccModel, c: usize) -> Vec<(String, f64, bool)> {
    m.rows.iter().enumerate().filter_map(|(i, r)| m.data.get(i).and_then(|d| d.get(c)).cloned().flatten().map(|v| (if r.id.is_empty() { (i + 1).to_string() } else { r.id.clone() }, v, !r.disabled))).collect()
}

/// Filas de datos por atributos (conteo o proporción de una característica).
pub fn attr_rows(m: &QccModel, c: usize) -> Vec<Sub> {
    m.rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let v = m.data.get(i).and_then(|d| d.get(c)).cloned().flatten();
            Sub { label: if r.id.is_empty() { (i + 1).to_string() } else { r.id.clone() }, vals: v.into_iter().collect(), enabled: !r.disabled && v.is_some(), size: r.size.unwrap_or(1.0), cause: r.cause, action: r.action }
        })
        .collect()
}

#[derive(Clone, Debug)]
pub enum Oc {
    /// Estadístico normal: β según el corrimiento de la media en desviaciones del proceso.
    Mean { ku: f64, kl: f64, scale: f64 },
    /// Estadístico de dispersión: β según la razón σ1/σ0 (aproximación normal).
    Spread { cl: f64, ucl: f64, lcl: f64, rel_sd: f64 },
    /// Varianza: ji cuadrada exacta.
    Variance { n: f64, ucl: f64, lcl: f64, s2: f64 },
    Binomial { n: f64, ucl: f64, lcl: f64, p: f64, count: bool },
    Poisson { n: f64, ucl: f64, lcl: f64, c: f64, per_unit: bool },
}

pub struct Plot {
    pub title: String,
    pub ylabel: String,
    pub labels: Vec<String>,
    pub w: Vec<f64>,
    pub enabled: Vec<bool>,
    pub cl: Vec<f64>,
    pub ucl: Vec<f64>,
    pub lcl: Vec<f64>,
    /// Desviación del estadístico en cada punto (para las zonas de las reglas).
    pub sd: Vec<f64>,
    pub cusum: Option<(Vec<f64>, Vec<f64>, f64)>,
    pub est: Vec<(String, f64)>,
    pub formula: String,
    pub oc: Option<Oc>,
    pub mu: f64,
    pub sigma: f64,
}

fn k_limits(o: &QccOptions) -> (f64, f64) {
    match o.limits {
        Limits::Probability => (zq(1.0 - o.prob_upper / 100.0), zq(1.0 - o.prob_lower / 100.0)),
        _ => (o.k_upper.abs(), o.k_lower.abs()),
    }
}

/// Línea central y desviación del proceso para datos por variables.
fn est_var(subs: &[Sub], ind: &[(String, f64, bool)], o: &QccOptions, kind: ChartKind) -> Result<(f64, f64, String, String), ModError> {
    let en: Vec<&Sub> = subs.iter().filter(|s| s.enabled && s.n() > 0).collect();
    let all: Vec<f64> = ind.iter().filter(|x| x.2).map(|x| x.1).collect();
    if all.len() < 2 {
        return invalid("hacen falta al menos dos observaciones habilitadas");
    }
    let individual = matches!(kind, ChartKind::X | ChartKind::CusumX | ChartKind::TrendX | ChartKind::GmaX | ChartKind::MaX | ChartKind::ModifiedX | ChartKind::AcceptanceX);
    let (mu, mname) = match o.mean_method {
        MeanMethod::Preset => (o.mean.ok_or_else(|| ModError::Invalid("indica la línea central".into()))?, "valor dado".to_string()),
        MeanMethod::Grand => (mean(&all), "media general".into()),
        MeanMethod::Median => (if individual { median(&all) } else { median(&en.iter().map(|s| s.median()).collect::<Vec<_>>()) }, "mediana".into()),
        MeanMethod::MeanOfMedians => (mean(&en.iter().map(|s| s.median()).collect::<Vec<_>>()), "media de las medianas".into()),
        MeanMethod::MeanOfMeans => (mean(&en.iter().map(|s| s.mean()).collect::<Vec<_>>()), "media de las medias".into()),
        MeanMethod::Auto => match kind {
            ChartKind::Median => (mean(&en.iter().map(|s| s.median()).collect::<Vec<_>>()), "media de las medianas".into()),
            ChartKind::Midrange => (mean(&en.iter().map(|s| s.midrange()).collect::<Vec<_>>()), "media de los rangos medios".into()),
            _ if individual => (mean(&all), "media general".into()),
            _ => (mean(&en.iter().map(|s| s.mean()).collect::<Vec<_>>()), "media de las medias".into()),
        },
    };
    let multi: Vec<&&Sub> = en.iter().filter(|s| s.n() >= 2).collect();
    let mr = || -> Option<f64> {
        let v: Vec<f64> = all.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
        (!v.is_empty()).then(|| mean(&v) / consts(2).d2)
    };
    let method = match o.sigma_method {
        SigmaMethod::Auto => {
            if individual || multi.is_empty() {
                SigmaMethod::MovingRange
            } else {
                SigmaMethod::Rbar
            }
        }
        x => x,
    };
    let (sg, sname) = match method {
        SigmaMethod::Preset => (o.sigma.ok_or_else(|| ModError::Invalid("indica la desviación estándar".into()))?, "valor dado".to_string()),
        SigmaMethod::Overall => (sd(&all), "desviación de todas las observaciones".into()),
        SigmaMethod::MovingRange => (mr().unwrap_or(0.0), "rango móvil medio / d2(2) = MR̄/1.128".into()),
        SigmaMethod::Rbar | SigmaMethod::Rmedian | SigmaMethod::Sbar | SigmaMethod::Auto => {
            if multi.is_empty() {
                return invalid("los subgrupos tienen una sola observación: usa el rango móvil o la desviación general");
            }
            match method {
                SigmaMethod::Rbar => (mean(&multi.iter().map(|s| s.range() / consts(s.n()).d2).collect::<Vec<_>>()), "R̄/d2".into()),
                SigmaMethod::Rmedian => {
                    let nb = (mean(&multi.iter().map(|s| s.n() as f64).collect::<Vec<_>>())).round() as usize;
                    (median(&multi.iter().map(|s| s.range()).collect::<Vec<_>>()) / consts(nb).d4, "R̃/d4".into())
                }
                _ => (mean(&multi.iter().map(|s| s.sd() / consts(s.n()).c4).collect::<Vec<_>>()), "S̄/c4".into()),
            }
        }
    };
    if !(sg > 0.0) {
        return invalid("la desviación estimada es cero: no hay variación en los datos");
    }
    Ok((mu, sg, mname, sname))
}

fn cusum(w: &[f64], en: &[bool], w0: f64, sdv: &[f64], o: &QccOptions) -> (Vec<f64>, Vec<f64>, f64, f64) {
    let delta = o.delta.max(1e-6);
    let (a, b) = ((o.alpha / 100.0).clamp(1e-9, 0.5), (o.beta / 100.0).clamp(1e-9, 0.99));
    let d = 2.0 * ((1.0 - b) / a).ln() / (delta * delta);
    let h = d * delta / 2.0;
    let (mut zh, mut zl) = (vec![], vec![]);
    let (mut a1, mut a2) = (0.0f64, 0.0f64);
    for i in 0..w.len() {
        if en[i] && w[i].is_finite() && sdv[i] > 0.0 {
            let z = (w[i] - w0) / sdv[i];
            a1 = (z - delta / 2.0 + a1).max(0.0);
            a2 = (-z - delta / 2.0 + a2).max(0.0);
        }
        zh.push(a1);
        zl.push(a2);
    }
    (zh, zl, h, d)
}

pub fn build(m: &QccModel, o: &QccOptions, kind: ChartKind, c: usize) -> Result<Plot, ModError> {
    let (ku, kl) = k_limits(o);
    let ch = &m.characteristics[c];
    let mut p = Plot { title: format!("Gráfico {} — {}", kind.label(), ch.name), ylabel: String::new(), labels: vec![], w: vec![], enabled: vec![], cl: vec![], ucl: vec![], lcl: vec![], sd: vec![], cusum: None, est: vec![], formula: String::new(), oc: None, mu: f64::NAN, sigma: f64::NAN };
    if !kind.attribute() {
        let subs = subgroups(m, c);
        let ind = individuals(m, c);
        let (mu, sg, mname, sname) = est_var(&subs, &ind, o, kind)?;
        p.mu = mu;
        p.sigma = sg;
        p.est.push((format!("Línea central ({mname})"), mu));
        p.est.push((format!("Desviación del proceso σ ({sname})"), sg));
        let individual = matches!(kind, ChartKind::X | ChartKind::CusumX | ChartKind::TrendX | ChartKind::GmaX | ChartKind::MaX | ChartKind::ModifiedX | ChartKind::AcceptanceX);
        // estadístico base
        let (labels, w, en, nn): (Vec<String>, Vec<f64>, Vec<bool>, Vec<f64>) = if individual {
            (ind.iter().map(|x| x.0.clone()).collect(), ind.iter().map(|x| x.1).collect(), ind.iter().map(|x| x.2).collect(), vec![1.0; ind.len()])
        } else {
            let stat = |s: &Sub| -> f64 {
                if s.n() == 0 {
                    return f64::NAN;
                }
                match kind {
                    ChartKind::R | ChartKind::CusumR => s.range(),
                    ChartKind::S | ChartKind::CusumS => s.sd(),
                    ChartKind::Variance => s.sd().powi(2),
                    ChartKind::Median => s.median(),
                    ChartKind::Midrange => s.midrange(),
                    _ => s.mean(),
                }
            };
            (subs.iter().map(|s| format!("{}", s.label)).collect(), subs.iter().map(stat).collect(), subs.iter().map(|s| s.enabled).collect(), subs.iter().map(|s| s.n().max(1) as f64).collect())
        };
        let n_of = |i: usize| nn[i].max(1.0) as usize;
        if matches!(kind, ChartKind::R | ChartKind::S | ChartKind::Variance | ChartKind::CusumR | ChartKind::CusumS) && nn.iter().all(|n| *n < 2.0) {
            return invalid("los gráficos de dispersión necesitan subgrupos de dos o más observaciones");
        }
        let navg = (mean(&nn)).round().max(1.0);
        p.labels = labels;
        p.enabled = en.clone();
        let len = w.len();
        let const_lim = |p: &mut Plot, cl: Vec<f64>, sdv: Vec<f64>| {
            p.ucl = cl.iter().zip(&sdv).map(|(c, s)| c + ku * s).collect();
            p.lcl = cl.iter().zip(&sdv).map(|(c, s)| c - kl * s).collect();
            p.cl = cl;
            p.sd = sdv;
        };
        match kind {
            ChartKind::Xbar | ChartKind::X => {
                p.ylabel = if individual { "X".into() } else { "Media del subgrupo".into() };
                let sdv: Vec<f64> = (0..len).map(|i| sg / (n_of(i) as f64).sqrt()).collect();
                p.w = w;
                const_lim(&mut p, vec![mu; len], sdv);
                p.formula = if individual { format!("LC = {}; LCS/LCI = LC ± k·σ con σ = {}.", fmt(tidy(mu)), fmt(tidy(sg))) } else { format!("LC = {}; LCS/LCI = LC ± k·σ/√n con σ = {}.", fmt(tidy(mu)), fmt(tidy(sg))) };
                p.oc = Some(Oc::Mean { ku, kl, scale: if individual { 1.0 } else { navg.sqrt() } });
            }
            ChartKind::R => {
                p.ylabel = "Rango R".into();
                let cl: Vec<f64> = (0..len).map(|i| consts(n_of(i)).d2 * sg).collect();
                let sdv: Vec<f64> = (0..len).map(|i| consts(n_of(i)).d3 * sg).collect();
                p.w = w;
                const_lim(&mut p, cl, sdv);
                p.lcl.iter_mut().for_each(|x| *x = x.max(0.0));
                let k = consts(navg as usize);
                p.formula = format!("LC = d2·σ, LCS/LCI = (d2 ± k·d3)·σ con d2 = {} y d3 = {} para n = {} (LCI no menor que 0).", fmt(tidy(k.d2)), fmt(tidy(k.d3)), navg);
                p.oc = Some(Oc::Spread { cl: k.d2 * sg, ucl: (k.d2 + ku * k.d3) * sg, lcl: ((k.d2 - kl * k.d3) * sg).max(0.0), rel_sd: k.d3 / k.d2 });
            }
            ChartKind::S => {
                p.ylabel = "Desviación S".into();
                let cl: Vec<f64> = (0..len).map(|i| consts(n_of(i)).c4 * sg).collect();
                let sdv: Vec<f64> = (0..len).map(|i| consts(n_of(i)).c5 * sg).collect();
                p.w = w;
                const_lim(&mut p, cl, sdv);
                p.lcl.iter_mut().for_each(|x| *x = x.max(0.0));
                let k = consts(navg as usize);
                p.formula = format!("LC = c4·σ, LCS/LCI = (c4 ± k·√(1 − c4²))·σ con c4 = {} para n = {}.", fmt(tidy(k.c4)), navg);
                p.oc = Some(Oc::Spread { cl: k.c4 * sg, ucl: (k.c4 + ku * k.c5) * sg, lcl: ((k.c4 - kl * k.c5) * sg).max(0.0), rel_sd: k.c5 / k.c4 });
            }
            ChartKind::Variance => {
                p.ylabel = "Varianza S²".into();
                let s2 = sg * sg;
                let (au, al) = match o.limits {
                    Limits::Probability => (o.prob_upper / 100.0, o.prob_lower / 100.0),
                    _ => (1.0 - Normal::new(0.0, 1.0).unwrap().cdf(ku), 1.0 - Normal::new(0.0, 1.0).unwrap().cdf(kl)),
                };
                p.w = w;
                p.cl = vec![s2; len];
                p.ucl = (0..len).map(|i| {
                    let df = (n_of(i).max(2) - 1) as f64;
                    s2 * ChiSquared::new(df).unwrap().inverse_cdf(1.0 - au) / df
                }).collect();
                p.lcl = (0..len).map(|i| {
                    let df = (n_of(i).max(2) - 1) as f64;
                    s2 * ChiSquared::new(df).unwrap().inverse_cdf(al) / df
                }).collect();
                p.sd = (0..len).map(|i| s2 * (2.0 / (n_of(i).max(2) - 1) as f64).sqrt()).collect();
                p.formula = format!("LC = σ² = {}; LCS = σ²·χ²(1 − α/2, n − 1)/(n − 1) y LCI = σ²·χ²(α/2, n − 1)/(n − 1).", fmt(tidy(s2)));
                let df = navg.max(2.0) - 1.0;
                p.oc = Some(Oc::Variance { n: navg, ucl: s2 * ChiSquared::new(df).unwrap().inverse_cdf(1.0 - au) / df, lcl: s2 * ChiSquared::new(df).unwrap().inverse_cdf(al) / df, s2 });
            }
            ChartKind::Median | ChartKind::Midrange => {
                p.ylabel = if kind == ChartKind::Median { "Mediana del subgrupo".into() } else { "Rango medio del subgrupo".into() };
                let f = |n: usize| if kind == ChartKind::Median { consts(n).e4 } else { consts(n).e3 };
                let sdv: Vec<f64> = (0..len).map(|i| f(n_of(i)) * sg).collect();
                p.w = w;
                const_lim(&mut p, vec![mu; len], sdv);
                p.formula = format!("LC = {}; LCS/LCI = LC ± k·{}·σ, con {} = {} (desviación de la {} de n = {} normales en unidades de σ).", fmt(tidy(mu)), if kind == ChartKind::Median { "e4" } else { "e3" }, if kind == ChartKind::Median { "e4" } else { "e3" }, fmt(tidy(f(navg as usize))), if kind == ChartKind::Median { "mediana" } else { "semisuma del máximo y el mínimo" }, navg);
                p.oc = Some(Oc::Mean { ku, kl, scale: 1.0 / f(navg as usize) });
            }
            ChartKind::CusumXbar | ChartKind::CusumX | ChartKind::CusumR | ChartKind::CusumS => {
                let (w0, sdv): (f64, Vec<f64>) = match kind {
                    ChartKind::CusumR => (consts(navg as usize).d2 * sg, (0..len).map(|i| consts(n_of(i)).d3 * sg).collect()),
                    ChartKind::CusumS => (consts(navg as usize).c4 * sg, (0..len).map(|i| consts(n_of(i)).c5 * sg).collect()),
                    _ => (ch.target.filter(|_| o.mean_method == MeanMethod::Auto).unwrap_or(mu), (0..len).map(|i| sg / (n_of(i) as f64).sqrt()).collect()),
                };
                cusum_plot(&mut p, w, &en, w0, sdv, o);
            }
            ChartKind::TrendXbar | ChartKind::TrendX => {
                p.ylabel = if individual { "X".into() } else { "Media del subgrupo".into() };
                let pts: Vec<(f64, f64)> = (0..len).filter(|&i| en[i] && w[i].is_finite()).map(|i| ((i + 1) as f64, w[i])).collect();
                let (xm, ym) = (mean(&pts.iter().map(|x| x.0).collect::<Vec<_>>()), mean(&pts.iter().map(|x| x.1).collect::<Vec<_>>()));
                let sxx: f64 = pts.iter().map(|x| (x.0 - xm).powi(2)).sum();
                let b = if sxx > 0.0 { pts.iter().map(|x| (x.0 - xm) * (x.1 - ym)).sum::<f64>() / sxx } else { 0.0 };
                let a = ym - b * xm;
                let sdv: Vec<f64> = (0..len).map(|i| sg / (n_of(i) as f64).sqrt()).collect();
                p.w = w;
                const_lim(&mut p, (0..len).map(|i| a + b * (i + 1) as f64).collect(), sdv);
                p.est.push(("Ordenada al origen a".into(), a));
                p.est.push(("Pendiente b".into(), b));
                p.formula = format!("Recta de mínimos cuadrados LC(i) = a + b·i = {} + {}·i; límites LC(i) ± k·σ{}.", fmt(tidy(a)), fmt(tidy(b)), if individual { "" } else { "/√n" });
            }
            ChartKind::GmaXbar | ChartKind::GmaX => {
                let sdv: Vec<f64> = (0..len).map(|i| sg / (n_of(i) as f64).sqrt()).collect();
                gma_plot(&mut p, &w, &en, mu, sdv, o, ku, kl);
            }
            ChartKind::MaXbar | ChartKind::MaX => {
                let sdv: Vec<f64> = (0..len).map(|i| sg / (n_of(i) as f64).sqrt()).collect();
                ma_plot(&mut p, &w, &en, mu, sdv, o, ku, kl);
            }
            ChartKind::ModifiedXbar | ChartKind::ModifiedX | ChartKind::AcceptanceXbar | ChartKind::AcceptanceX => {
                let (usl, lsl) = (ch.usl, ch.lsl);
                if usl.is_none() && lsl.is_none() {
                    return invalid(format!("{}: este gráfico necesita los límites de especificación LSL y/o USL", ch.name));
                }
                p.ylabel = if individual { "X".into() } else { "Media del subgrupo".into() };
                let modified = matches!(kind, ChartKind::ModifiedXbar | ChartKind::ModifiedX);
                let off = |n: f64| {
                    if modified {
                        zq(1.0 - o.mod_delta / 100.0) - zq(1.0 - o.mod_alpha / 100.0) / n.sqrt()
                    } else {
                        zq(1.0 - o.acc_gamma / 100.0) + zq(1.0 - o.acc_beta / 100.0) / n.sqrt()
                    }
                };
                p.w = w;
                p.cl = vec![mu; len];
                p.ucl = (0..len).map(|i| usl.map(|u| u - off(nn[i]) * sg).unwrap_or(f64::NAN)).collect();
                p.lcl = (0..len).map(|i| lsl.map(|l| l + off(nn[i]) * sg).unwrap_or(f64::NAN)).collect();
                p.sd = (0..len).map(|i| sg / nn[i].sqrt()).collect();
                p.formula = if modified {
                    format!("LCS = USL − (Z_δ − Z_α/√n)·σ y LCI = LSL + (Z_δ − Z_α/√n)·σ, con δ = {} % y α = {} %.", fmt(o.mod_delta), fmt(o.mod_alpha))
                } else {
                    format!("LCS = USL − (Z_γ + Z_β/√n)·σ y LCI = LSL + (Z_γ + Z_β/√n)·σ, con γ = {} % y β = {} %.", fmt(o.acc_gamma), fmt(o.acc_beta))
                };
            }
            _ => unreachable!(),
        }
        return Ok(p);
    }
    // ---------------------------------------------------------- atributos
    let rows = attr_rows(m, c);
    let en: Vec<bool> = rows.iter().map(|r| r.enabled).collect();
    let nn: Vec<f64> = rows.iter().map(|r| r.size.max(1e-12)).collect();
    let raw: Vec<f64> = rows.iter().map(|r| r.vals.first().copied().unwrap_or(f64::NAN)).collect();
    // proporciones si todos los valores están entre 0 y 1 y alguno es fraccionario
    let props = raw.iter().filter(|x| x.is_finite()).all(|x| (0.0..=1.0).contains(x)) && raw.iter().any(|x| x.is_finite() && x.fract() != 0.0);
    let d: Vec<f64> = raw.iter().zip(&nn).map(|(v, n)| if props && matches!(kind, ChartKind::P | ChartKind::Np | ChartKind::CusumP | ChartKind::GmaP | ChartKind::MaP | ChartKind::StdP | ChartKind::StdNp) { v * n } else { *v }).collect();
    p.labels = rows.iter().map(|r| r.label.clone()).collect();
    p.enabled = en.clone();
    let len = rows.len();
    let tot = |f: &dyn Fn(usize) -> f64| (0..len).filter(|&i| en[i] && d[i].is_finite()).map(f).sum::<f64>();
    if !(0..len).any(|i| en[i] && d[i].is_finite()) {
        return invalid("no hay subgrupos habilitados con datos");
    }
    let nbar = mean(&(0..len).filter(|&i| en[i]).map(|i| nn[i]).collect::<Vec<_>>());
    let pbar = o.mean.filter(|_| o.mean_method == MeanMethod::Preset).unwrap_or(tot(&|i| d[i]) / tot(&|i| nn[i]));
    let cbar = o.mean.filter(|_| o.mean_method == MeanMethod::Preset).unwrap_or(mean(&(0..len).filter(|&i| en[i] && d[i].is_finite()).map(|i| d[i]).collect::<Vec<_>>()));
    let ubar = o.mean.filter(|_| o.mean_method == MeanMethod::Preset).unwrap_or(tot(&|i| d[i]) / tot(&|i| nn[i]));
    let preset_sd = o.sigma.filter(|_| o.sigma_method == SigmaMethod::Preset);
    let limits = |p: &mut Plot, w: Vec<f64>, cl: Vec<f64>, sdv: Vec<f64>| {
        p.ucl = cl.iter().zip(&sdv).map(|(c, s)| c + ku * s).collect();
        p.lcl = cl.iter().zip(&sdv).map(|(c, s)| (c - kl * s).max(0.0)).collect();
        p.w = w;
        p.cl = cl;
        p.sd = sdv;
    };
    match kind {
        ChartKind::P | ChartKind::CusumP | ChartKind::GmaP | ChartKind::MaP | ChartKind::StdP => {
            let w: Vec<f64> = (0..len).map(|i| d[i] / nn[i]).collect();
            let sdv: Vec<f64> = (0..len).map(|i| preset_sd.unwrap_or((pbar * (1.0 - pbar) / nn[i]).sqrt())).collect();
            p.mu = pbar;
            p.est.push(("Proporción media p̄ = Σd/Σn".into(), pbar));
            p.est.push(("Tamaño medio de subgrupo".into(), nbar));
            match kind {
                ChartKind::P => {
                    p.ylabel = "p".into();
                    limits(&mut p, w, vec![pbar; len], sdv);
                    p.formula = format!("LC = p̄ = {}; LCS/LCI = p̄ ± k·√(p̄(1 − p̄)/n), LCI no menor que 0.", fmt(tidy(pbar)));
                    p.oc = Some(Oc::Binomial { n: nbar.round(), ucl: p.ucl[0], lcl: p.lcl[0], p: pbar, count: false });
                }
                ChartKind::CusumP => cusum_plot(&mut p, w, &en, pbar, sdv, o),
                ChartKind::GmaP => gma_plot(&mut p, &w, &en, pbar, sdv, o, ku, kl),
                ChartKind::MaP => ma_plot(&mut p, &w, &en, pbar, sdv, o, ku, kl),
                _ => std_plot(&mut p, &w, pbar, &sdv, ku, kl, "p", "(p − p̄)/√(p̄(1 − p̄)/n)"),
            }
        }
        ChartKind::Np | ChartKind::StdNp => {
            let w = d.clone();
            let cl: Vec<f64> = nn.iter().map(|n| n * pbar).collect();
            let sdv: Vec<f64> = nn.iter().map(|n| preset_sd.unwrap_or((n * pbar * (1.0 - pbar)).sqrt())).collect();
            p.mu = pbar;
            p.est.push(("Proporción media p̄".into(), pbar));
            if kind == ChartKind::Np {
                p.ylabel = "np".into();
                limits(&mut p, w, cl, sdv);
                p.formula = format!("LC = n·p̄; LCS/LCI = n·p̄ ± k·√(n·p̄(1 − p̄)) con p̄ = {}.", fmt(tidy(pbar)));
                p.oc = Some(Oc::Binomial { n: nbar.round(), ucl: p.ucl[0], lcl: p.lcl[0], p: pbar, count: true });
            } else {
                let z: Vec<f64> = (0..len).map(|i| (w[i] - cl[i]) / sdv[i]).collect();
                std_plot(&mut p, &z, 0.0, &vec![1.0; len], ku, kl, "z", "(np − n·p̄)/√(n·p̄(1 − p̄))");
                p.w = z;
            }
        }
        ChartKind::C | ChartKind::CusumC | ChartKind::GmaC | ChartKind::MaC | ChartKind::StdC => {
            let w = d.clone();
            let sdv = vec![preset_sd.unwrap_or(cbar.sqrt()); len];
            p.mu = cbar;
            p.est.push(("Defectos medios c̄".into(), cbar));
            match kind {
                ChartKind::C => {
                    p.ylabel = "c".into();
                    limits(&mut p, w, vec![cbar; len], sdv);
                    p.formula = format!("LC = c̄ = {}; LCS/LCI = c̄ ± k·√c̄ (Poisson), LCI no menor que 0.", fmt(tidy(cbar)));
                    p.oc = Some(Oc::Poisson { n: 1.0, ucl: p.ucl[0], lcl: p.lcl[0], c: cbar, per_unit: false });
                }
                ChartKind::CusumC => cusum_plot(&mut p, w, &en, cbar, sdv, o),
                ChartKind::GmaC => gma_plot(&mut p, &w, &en, cbar, sdv, o, ku, kl),
                ChartKind::MaC => ma_plot(&mut p, &w, &en, cbar, sdv, o, ku, kl),
                _ => std_plot(&mut p, &w, cbar, &sdv, ku, kl, "c", "(c − c̄)/√c̄"),
            }
        }
        ChartKind::U | ChartKind::StdU => {
            let w: Vec<f64> = (0..len).map(|i| d[i] / nn[i]).collect();
            let sdv: Vec<f64> = nn.iter().map(|n| preset_sd.unwrap_or((ubar / n).sqrt())).collect();
            p.mu = ubar;
            p.est.push(("Defectos por unidad ū = Σc/Σn".into(), ubar));
            if kind == ChartKind::U {
                p.ylabel = "u".into();
                limits(&mut p, w, vec![ubar; len], sdv);
                p.formula = format!("LC = ū = {}; LCS/LCI = ū ± k·√(ū/n).", fmt(tidy(ubar)));
                p.oc = Some(Oc::Poisson { n: nbar, ucl: p.ucl[0], lcl: p.lcl[0], c: ubar, per_unit: true });
            } else {
                std_plot(&mut p, &w, ubar, &sdv, ku, kl, "u", "(u − ū)/√(ū/n)");
            }
        }
        ChartKind::Demerits => {
            let k = m.characteristics.len();
            let wts: Vec<f64> = m.characteristics.iter().map(|c| c.demerit.unwrap_or(1.0)).collect();
            let cnt = |i: usize, j: usize| m.data.get(i).and_then(|r| r.get(j)).cloned().flatten().unwrap_or(0.0);
            let sum_n: f64 = (0..len).filter(|&i| en[i]).map(|i| nn[i]).sum();
            let ubar_j: Vec<f64> = (0..k).map(|j| (0..len).filter(|&i| en[i]).map(|i| cnt(i, j)).sum::<f64>() / sum_n).collect();
            let cl: f64 = (0..k).map(|j| wts[j] * ubar_j[j]).sum();
            let y: f64 = (0..k).map(|j| wts[j] * wts[j] * ubar_j[j]).sum();
            let w: Vec<f64> = (0..len).map(|i| (0..k).map(|j| wts[j] * cnt(i, j)).sum::<f64>() / nn[i]).collect();
            let sdv: Vec<f64> = nn.iter().map(|n| preset_sd.unwrap_or((y / n).sqrt())).collect();
            p.title = format!("Gráfico {}", kind.label());
            p.ylabel = "U".into();
            p.mu = cl;
            p.est.push(("Deméritos medios por unidad Σ dⱼ·ūⱼ".into(), cl));
            p.est.push(("Y = Σ dⱼ²·ūⱼ".into(), y));
            limits(&mut p, w, vec![cl; len], sdv);
            p.formula = format!("U = Σ dⱼ·cⱼ/n; LC = Σ dⱼ·ūⱼ = {}; LCS/LCI = LC ± k·√(Y/n) con Y = Σ dⱼ²·ūⱼ = {}.", fmt(tidy(cl)), fmt(tidy(y)));
        }
        _ => unreachable!(),
    }
    Ok(p)
}

fn cusum_plot(p: &mut Plot, w: Vec<f64>, en: &[bool], w0: f64, sdv: Vec<f64>, o: &QccOptions) {
    let (zh, zl, h, d) = cusum(&w, en, w0, &sdv, o);
    let mut y = 0.0;
    p.w = w.iter().zip(en).map(|(x, e)| {
        if *e && x.is_finite() {
            y += x - w0;
        }
        y
    }).collect();
    p.ylabel = "Σ(w − w₀)".into();
    let len = p.w.len();
    p.cl = vec![0.0; len];
    p.ucl = vec![f64::NAN; len];
    p.lcl = vec![f64::NAN; len];
    p.sd = vec![f64::NAN; len];
    p.cusum = Some((zh, zl, h));
    p.est.push(("Valor objetivo w₀".into(), w0));
    p.est.push(("Intervalo de decisión H (en desviaciones)".into(), h));
    p.est.push(("Valor de referencia K = Δ/2 (en desviaciones)".into(), o.delta / 2.0));
    p.est.push(("Distancia d de la máscara V".into(), d));
    p.est.push(("Semiángulo de la máscara V θ (grados)".into(), (o.delta / 4.0).atan().to_degrees()));
    p.formula = format!(
        "Suma acumulada y(i) = Σ (w − w₀). CUSUM tabular estandarizado: ZH(i) = máx[0, z − Δ/2 + ZH(i−1)] y ZL(i) = máx[0, −z − Δ/2 + ZL(i−1)] con z = (w − w₀)/σ_w. Fuera de control si ZH o ZL pasa de H = d·Δ/2 = {}, con d = 2·ln[(1 − β)/α]/Δ².",
        fmt(tidy(h))
    );
}

#[allow(clippy::too_many_arguments)]
fn gma_plot(p: &mut Plot, w: &[f64], en: &[bool], mu: f64, sdv: Vec<f64>, o: &QccOptions, ku: f64, kl: f64) {
    let l = o.lambda.clamp(1e-6, 1.0);
    let mut z = mu;
    p.w = w.iter().zip(en).map(|(x, e)| {
        if *e && x.is_finite() {
            z = l * x + (1.0 - l) * z;
        }
        z
    }).collect();
    let f = (l / (2.0 - l)).sqrt();
    let len = w.len();
    p.ylabel = "Z (EWMA)".into();
    p.cl = vec![mu; len];
    p.ucl = sdv.iter().map(|s| mu + ku * s * f).collect();
    p.lcl = sdv.iter().map(|s| mu - kl * s * f).collect();
    p.sd = sdv.iter().map(|s| s * f).collect();
    p.formula = format!("Z(i) = λ·w(i) + (1 − λ)·Z(i − 1) con Z(0) = {} y λ = {}; LCS/LCI = LC ± k·σ_w·√(λ/(2 − λ)).", fmt(tidy(mu)), fmt(l));
}

#[allow(clippy::too_many_arguments)]
fn ma_plot(p: &mut Plot, w: &[f64], en: &[bool], mu: f64, sdv: Vec<f64>, o: &QccOptions, ku: f64, kl: f64) {
    let span = o.span.max(1) as usize;
    let mut buf: Vec<f64> = vec![];
    let mut cnt = vec![];
    p.w = w.iter().zip(en).map(|(x, e)| {
        if *e && x.is_finite() {
            buf.push(*x);
            if buf.len() > span {
                buf.remove(0);
            }
        }
        cnt.push(buf.len().max(1));
        if buf.is_empty() { f64::NAN } else { mean(&buf) }
    }).collect();
    p.ylabel = format!("Media móvil de {span}");
    let len = w.len();
    p.cl = vec![mu; len];
    p.sd = (0..len).map(|i| sdv[i] / (cnt[i] as f64).sqrt()).collect();
    p.ucl = (0..len).map(|i| mu + ku * p.sd[i]).collect();
    p.lcl = (0..len).map(|i| mu - kl * p.sd[i]).collect();
    p.formula = format!("Z(i) = media de los últimos {span} valores; LCS/LCI = LC ± k·σ_w/√(amplitud) (al principio, con los puntos que haya).");
}

#[allow(clippy::too_many_arguments)]
fn std_plot(p: &mut Plot, w: &[f64], mu: f64, sdv: &[f64], ku: f64, kl: f64, sym: &str, how: &str) {
    let len = w.len();
    p.w = (0..len).map(|i| (w[i] - mu) / sdv[i]).collect();
    p.ylabel = format!("z de {sym}");
    p.cl = vec![0.0; len];
    p.ucl = vec![ku; len];
    p.lcl = vec![-kl; len];
    p.sd = vec![1.0; len];
    p.formula = format!("Estadístico estandarizado z = {how}; LC = 0 y límites ±k (constantes aunque cambie n).");
}

/// Reglas violadas en cada punto (índices de regla).
pub fn rules(p: &Plot, rules: &[Rule]) -> Vec<Vec<usize>> {
    let len = p.w.len();
    let mut out = vec![vec![]; len];
    let idx: Vec<usize> = (0..len).filter(|&i| p.enabled[i] && p.w[i].is_finite()).collect();
    if idx.is_empty() {
        return out;
    }
    let med = median(&idx.iter().map(|&i| p.w[i]).collect::<Vec<_>>());
    for (r, rule) in rules.iter().enumerate() {
        let n = rule.n_of.max(1.0) as usize;
        let mm = rule.m_points.max(rule.n_of).max(1.0) as usize;
        // condición por punto según la primera columna no nula (como WinQSB)
        let cond = |k: usize| -> Option<bool> {
            let i = idx[k];
            let (w, cl, sd) = (p.w[i], p.cl[i], p.sd[i]);
            if rule.off_limit != 0.0 {
                return Some(if rule.off_limit > 0.0 { p.ucl[i].is_finite() && w > p.ucl[i] + 1e-12 } else { p.lcl[i].is_finite() && w < p.lcl[i] - 1e-12 });
            }
            if rule.sigma_off != 0.0 {
                if !sd.is_finite() || sd <= 0.0 {
                    return Some(false);
                }
                return Some(if rule.sigma_off > 0.0 { w > cl + rule.sigma_off * sd } else { w < cl + rule.sigma_off * sd });
            }
            if rule.row_cl != 0.0 {
                return Some(if rule.row_cl > 0.0 { w > cl } else { w < cl });
            }
            if rule.row_median != 0.0 {
                return Some(if rule.row_median > 0.0 { w > med } else { w < med });
            }
            if rule.row_updown != 0.0 {
                if k == 0 {
                    return Some(false);
                }
                let prev = p.w[idx[k - 1]];
                return Some(if rule.row_updown > 0.0 { w > prev } else { w < prev });
            }
            if rule.jump != 0.0 {
                if k == 0 || !sd.is_finite() || sd <= 0.0 {
                    return Some(false);
                }
                let prev = p.w[idx[k - 1]];
                return Some(if rule.jump > 0.0 { w - prev >= rule.jump * sd } else { w - prev <= rule.jump * sd });
            }
            None
        };
        if cond(0).is_none() && idx.len() > 1 && cond(1).is_none() {
            continue; // regla vacía
        }
        let flags: Vec<bool> = (0..idx.len()).map(|k| cond(k).unwrap_or(false)).collect();
        // subir/bajar: m puntos seguidos son m − 1 cambios
        let (need, win) = if rule.row_updown != 0.0 { (mm.saturating_sub(1).max(1), mm.saturating_sub(1).max(1)) } else { (n, mm) };
        for k in 0..idx.len() {
            if !flags[k] || k + 1 < win {
                continue;
            }
            let hit = flags[k + 1 - win..=k].iter().filter(|f| **f).count();
            if hit >= need {
                out[idx[k]].push(r);
            }
        }
    }
    out
}
