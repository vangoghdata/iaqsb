//! Muestreo de aceptación por atributos: simple, doble, múltiple, secuencial
//! (Wald), en cadena ChSP-1, continuo CSP-1 y por saltos de lotes SkSP-2.

use super::{AsaModel, AsaOptions, Criterion, Mode, Plan, ProbDist, Stage};
use iaqsb_core::*;
use serde_json::{json, Value};
use statrs::distribution::{ContinuousCDF, Normal};
use statrs::function::gamma::ln_gamma;

fn ln_choose(n: f64, k: f64) -> f64 {
    ln_gamma(n + 1.0) - ln_gamma(k + 1.0) - ln_gamma(n - k + 1.0)
}
fn phi(z: f64) -> f64 {
    Normal::new(0.0, 1.0).unwrap().cdf(z)
}

/// Contexto de cálculo para una fracción defectuosa p.
#[derive(Clone, Copy)]
pub struct Ctx {
    pub dist: ProbDist,
    pub lot: Option<f64>,
    /// Fracción defectuosa aparente (con errores de inspección).
    pub pe: f64,
}

impl Ctx {
    pub fn new(m: &AsaModel, p: f64) -> Ctx {
        let (e1, e2) = (m.error_good / 100.0, m.error_bad / 100.0);
        let pe = (p * (1.0 - e2) + (1.0 - p) * e1).clamp(0.0, 1.0);
        Ctx { dist: m.distribution, lot: m.lot_size, pe }
    }
    /// Distribución efectiva («Program selection» de WinQSB para el muestreo simple).
    fn effective(&self, n: usize) -> ProbDist {
        match self.dist {
            ProbDist::Auto => {
                let frac = self.lot.map(|nn| n as f64 / nn).unwrap_or(0.0);
                let np = n as f64 * self.pe;
                if frac < 0.1 && np > 10.0 && self.pe < 0.5 {
                    ProbDist::Normal
                } else if frac < 0.1 && self.pe < 0.1 {
                    ProbDist::Poisson
                } else if frac < 0.1 {
                    ProbDist::Binomial
                } else {
                    ProbDist::Hypergeometric
                }
            }
            ProbDist::Hypergeometric if self.lot.is_none() => ProbDist::Binomial,
            d => d,
        }
    }
    /// P(X = x), x = 0..=n, para una muestra de n tomada después de `before` unidades con `xb` defectuosos.
    pub fn pmf(&self, n: usize, before: usize, xb: usize) -> Vec<f64> {
        let pe = self.pe;
        let mut v = vec![0.0; n + 1];
        if n == 0 {
            v[0] = 1.0;
            return v;
        }
        match self.effective(n) {
            ProbDist::Binomial | ProbDist::Auto => {
                if pe <= 0.0 {
                    v[0] = 1.0;
                } else if pe >= 1.0 {
                    v[n] = 1.0;
                } else {
                    for x in 0..=n {
                        v[x] = (ln_choose(n as f64, x as f64) + x as f64 * pe.ln() + (n - x) as f64 * (1.0 - pe).ln()).exp();
                    }
                }
            }
            ProbDist::Poisson => {
                let lam = n as f64 * pe;
                if lam <= 0.0 {
                    v[0] = 1.0;
                } else {
                    let mut acc = 0.0;
                    for x in 0..n {
                        v[x] = (x as f64 * lam.ln() - lam - ln_gamma(x as f64 + 1.0)).exp();
                        acc += v[x];
                    }
                    v[n] = (1.0 - acc).max(0.0);
                }
            }
            ProbDist::Normal => {
                let mu = n as f64 * pe;
                let sd = (n as f64 * pe * (1.0 - pe)).sqrt();
                if sd <= 0.0 {
                    v[(mu.round() as usize).min(n)] = 1.0;
                } else {
                    let f = |x: f64| phi((x - mu) / sd);
                    for x in 0..=n {
                        let lo = if x == 0 { 0.0 } else { f(x as f64 - 0.5) };
                        let hi = if x == n { 1.0 } else { f(x as f64 + 0.5) };
                        v[x] = (hi - lo).max(0.0);
                    }
                }
            }
            ProbDist::Hypergeometric => {
                let nn = self.lot.unwrap_or(1e9);
                let l = (nn - before as f64).max(n as f64);
                let d_tot = (nn * pe).round();
                let d = (d_tot - xb as f64).clamp(0.0, l);
                for x in 0..=n {
                    let xf = x as f64;
                    if xf > d || (n as f64 - xf) > l - d {
                        continue;
                    }
                    v[x] = (ln_choose(d, xf) + ln_choose(l - d, n as f64 - xf) - ln_choose(l, n as f64)).exp();
                }
            }
        }
        v
    }
}

/// Resultado de un plan para una fracción defectuosa.
#[derive(Clone, Copy, Debug, Default)]
pub struct Eval {
    pub pa: f64,
    pub asn: f64,
    pub aoq: f64,
    pub ati: Option<f64>,
    pub cost: Option<f64>,
    /// CSP-1: fracción media inspeccionada.
    pub afi: Option<f64>,
}

/// Etapas (n, c, r) del plan; la última tiene r = c + 1.
fn stage_list(st: &[Stage]) -> Vec<(usize, i64, i64)> {
    let k = st.len();
    st.iter()
        .enumerate()
        .map(|(i, s)| {
            let c = s.c.round() as i64;
            let r = if i + 1 == k { c + 1 } else { s.r.map(|r| r.round() as i64).unwrap_or(c + 1) };
            (s.n.round().max(0.0) as usize, c, r)
        })
        .collect()
}

/// Probabilidades de aceptar en cada etapa y de llegar a ella.
fn stages(list: &[(usize, i64, i64)], ctx: &Ctx) -> (Vec<f64>, Vec<f64>) {
    let mut state = vec![1.0f64];
    let mut before = 0usize;
    let mut acc = vec![];
    let mut reach = vec![];
    let hyp = ctx.effective(list.first().map(|x| x.0).unwrap_or(0)) == ProbDist::Hypergeometric;
    for &(n, c, r) in list {
        reach.push(state.iter().sum::<f64>());
        let mut new = vec![0.0; state.len() + n];
        let common = if hyp { None } else { Some(ctx.pmf(n, before, 0)) };
        for (x, &w) in state.iter().enumerate() {
            if w <= 0.0 {
                continue;
            }
            let pm = match &common {
                Some(v) => v.clone(),
                None => ctx.pmf(n, before, x),
            };
            for (y, &q) in pm.iter().enumerate() {
                new[x + y] += w * q;
            }
        }
        let mut a = 0.0;
        for (x, v) in new.iter_mut().enumerate() {
            let xi = x as i64;
            if xi <= c {
                a += *v;
                *v = 0.0;
            } else if xi >= r {
                *v = 0.0;
            }
        }
        acc.push(a);
        state = new;
        before += n;
    }
    (acc, reach)
}

/// ASN con muestreo truncado: se deja de inspeccionar en cuanto se alcanza el número de rechazo.
fn asn_curtailed(list: &[(usize, i64, i64)], pe: f64) -> f64 {
    let mut state = vec![1.0f64];
    let mut asn = 0.0;
    for &(n, c, r) in list {
        for _ in 0..n {
            asn += state.iter().sum::<f64>();
            let mut new = vec![0.0; state.len() + 1];
            for (x, &w) in state.iter().enumerate() {
                new[x] += w * (1.0 - pe);
                new[x + 1] += w * pe;
            }
            for (x, v) in new.iter_mut().enumerate() {
                if x as i64 >= r {
                    *v = 0.0;
                }
            }
            state = new;
        }
        for (x, v) in state.iter_mut().enumerate() {
            if x as i64 <= c {
                *v = 0.0;
            }
        }
    }
    asn
}

/// Parámetros de Wald del plan secuencial.
pub struct Wald {
    pub h1: f64,
    pub h2: f64,
    pub s: f64,
    p1: f64,
    p2: f64,
    a: f64,
    b: f64,
}

pub fn wald(m: &AsaModel) -> Result<Wald, ModError> {
    let (p1, p2) = (m.aql / 100.0, m.rql / 100.0);
    let (al, be) = (m.alpha / 100.0, m.beta / 100.0);
    if !(p1 > 0.0 && p2 > p1 && p2 < 1.0) {
        return invalid("el plan secuencial necesita 0 < AQL < RQL < 100 %");
    }
    if !(al > 0.0 && al < 1.0 && be > 0.0 && be < 1.0) {
        return invalid("α y β deben estar entre 0 y 100 %");
    }
    let k = ((p2 * (1.0 - p1)) / (p1 * (1.0 - p2))).ln();
    let s = ((1.0 - p1) / (1.0 - p2)).ln() / k;
    let h1 = ((1.0 - al) / be).ln() / k;
    let h2 = ((1.0 - be) / al).ln() / k;
    Ok(Wald { h1, h2, s, p1, p2, a: (1.0 - be) / al, b: be / (1.0 - al) })
}

impl Wald {
    /// OC y ASN de Wald para una fracción p (por el parámetro h).
    fn eval(&self, p: f64) -> (f64, f64) {
        let (p1, p2) = (self.p1, self.p2);
        let r1 = p2 / p1;
        let r0 = (1.0 - p2) / (1.0 - p1);
        let p_of = |h: f64| -> f64 {
            if h.abs() < 1e-9 {
                return self.s;
            }
            (1.0 - r0.powf(h)) / (r1.powf(h) - r0.powf(h))
        };
        let (la, lb) = (self.a.ln(), self.b.ln());
        let pa_of = |h: f64| -> f64 {
            if h.abs() < 1e-9 {
                return la / (la - lb);
            }
            (self.a.powf(h) - 1.0) / (self.a.powf(h) - self.b.powf(h))
        };
        let pa = if p <= 0.0 {
            1.0
        } else if p >= 1.0 {
            0.0
        } else {
            // p(h) decrece con h
            let (mut lo, mut hi) = (-60.0f64, 60.0f64);
            for _ in 0..200 {
                let mid = 0.5 * (lo + hi);
                if p_of(mid) > p {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            pa_of(0.5 * (lo + hi)).clamp(0.0, 1.0)
        };
        let den = p * r1.ln() + (1.0 - p) * r0.ln();
        let asn = if den.abs() < 1e-12 { -la * lb / (r1.ln() * r0.ln()) } else { (pa * lb + (1.0 - pa) * la) / den };
        (pa, asn.max(1.0))
    }
}

pub fn eval(m: &AsaModel, p: f64) -> Eval {
    let ctx = Ctx::new(m, p);
    let lot = m.lot_size;
    let e2 = m.error_bad / 100.0;
    let mut e = Eval::default();
    match m.problem_type {
        Plan::Single | Plan::Double | Plan::Multiple => {
            let list = stage_list(&m.stages);
            let (acc, reach) = stages(&list, &ctx);
            e.pa = acc.iter().sum::<f64>().clamp(0.0, 1.0);
            e.asn = if m.curtailed { asn_curtailed(&list, ctx.pe) } else { reach.iter().zip(&list).map(|(r, s)| r * s.0 as f64).sum() };
            let mut cum = 0usize;
            let mut aoq = 0.0;
            let mut ati = 0.0;
            for (k, &(n, _, _)) in list.iter().enumerate() {
                cum += n;
                let c = cum as f64;
                if let Some(nn) = lot {
                    let out = if m.rectified { nn } else { (nn - c * p).max(1.0) };
                    aoq += acc[k] * (p * (nn - c).max(0.0) + p * e2 * c) / out;
                    ati += acc[k] * c;
                } else {
                    aoq += acc[k] * p;
                }
            }
            if let Some(nn) = lot {
                aoq += (1.0 - e.pa) * p * e2;
                ati += (1.0 - e.pa) * nn;
                e.ati = Some(ati);
            }
            e.aoq = aoq;
        }
        Plan::Chain => {
            let n = m.stages.first().map(|s| s.n.round().max(0.0) as usize).unwrap_or(0);
            let pm = ctx.pmf(n, 0, 0);
            let p0 = pm[0];
            let p1 = pm.get(1).copied().unwrap_or(0.0);
            e.pa = (p0 + p1 * p0.powf(m.clearance.max(0.0))).clamp(0.0, 1.0);
            e.asn = n as f64;
            single_like(&mut e, p, n as f64, lot, m.rectified, e2);
        }
        Plan::SkipLot => {
            let n = m.stages.first().map(|s| s.n.round().max(0.0) as usize).unwrap_or(0);
            let c = m.stages.first().map(|s| s.c.round() as i64).unwrap_or(0);
            let pm = ctx.pmf(n, 0, 0);
            let pr: f64 = pm.iter().take((c + 1).max(0) as usize).sum();
            let f = (m.fraction / 100.0).clamp(1e-9, 1.0);
            let pi = pr.powf(m.clearance.max(0.0));
            let ff = f / ((1.0 - f) * pi + f);
            e.pa = ((f * pr + (1.0 - f) * pi) / (f + (1.0 - f) * pi)).clamp(0.0, 1.0);
            e.asn = ff * n as f64;
            match lot {
                Some(nn) => {
                    e.aoq = p * ((1.0 - ff) * nn + ff * pr * (nn - n as f64)) / nn;
                    e.ati = Some(ff * (n as f64 + (1.0 - pr) * (nn - n as f64)));
                }
                None => e.aoq = e.pa * p,
            }
        }
        Plan::Continuous => {
            let pe = ctx.pe;
            let f = (m.fraction / 100.0).clamp(1e-9, 1.0);
            let i = m.clearance.max(0.0);
            let (afi, pa) = if pe <= 0.0 {
                (f, 1.0)
            } else if pe >= 1.0 {
                (1.0, 0.0)
            } else {
                let q = 1.0 - pe;
                let u = (1.0 - q.powf(i)) / (pe * q.powf(i));
                let v = 1.0 / (f * pe);
                ((u + f * v) / (u + v), v / (u + v))
            };
            e.afi = Some(afi);
            e.pa = pa;
            e.aoq = p * (1.0 - afi);
            e.asn = afi;
        }
        Plan::Sequential => {
            if let Ok(w) = wald(m) {
                let (pa, asn) = w.eval(ctx.pe);
                e.pa = pa;
                e.asn = asn;
                match lot {
                    Some(nn) => {
                        e.aoq = pa * p * (nn - asn).max(0.0) / nn;
                        e.ati = Some(pa * asn + (1.0 - pa) * nn);
                    }
                    None => e.aoq = pa * p,
                }
            }
        }
        _ => {}
    }
    // costo total medio por lote: ATI·C(I) + ASN·C(S) + n(d)·C(P) + N(d)·C(C)
    let has_cost = m.sampling_cost != 0.0 || m.inspection_cost != 0.0 || m.producer_cost != 0.0 || m.consumer_cost != 0.0;
    if has_cost {
        if let (Some(ati), Some(nn)) = (e.ati, lot) {
            let found = p * (1.0 - e2) * ati;
            let passed = (nn * p - found).max(0.0);
            e.cost = Some(ati * m.inspection_cost + e.asn * m.sampling_cost + found * m.producer_cost + passed * m.consumer_cost);
        }
    }
    e
}

fn single_like(e: &mut Eval, p: f64, n: f64, lot: Option<f64>, rect: bool, e2: f64) {
    match lot {
        Some(nn) => {
            let out = if rect { nn } else { (nn - n * p).max(1.0) };
            e.aoq = (e.pa * (p * (nn - n).max(0.0) + p * e2 * n) + (1.0 - e.pa) * p * e2 * nn) / out;
            e.ati = Some(e.pa * n + (1.0 - e.pa) * nn);
        }
        None => e.aoq = e.pa * p,
    }
}

/// p tal que Pa(p) = objetivo (Pa decrece con p).
fn p_for(m: &AsaModel, target: f64) -> Option<f64> {
    let f = |p: f64| eval(m, p).pa - target;
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    if f(lo) < 0.0 || f(hi) > 0.0 {
        return None;
    }
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if f(mid) > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(0.5 * (lo + hi))
}

/// AOQL: máximo de AOQ (rejilla y refinamiento).
fn aoql(m: &AsaModel) -> (f64, f64) {
    let hi = m.rql.max(m.aql * 4.0).max(5.0).min(100.0) / 100.0 * 3.0;
    let hi = hi.min(1.0);
    let mut best = (0.0, 0.0);
    let k = 300;
    for i in 1..=k {
        let p = hi * i as f64 / k as f64;
        let a = eval(m, p).aoq;
        if a > best.1 {
            best = (p, a);
        }
    }
    let (mut lo, mut up) = ((best.0 - hi / k as f64).max(0.0), (best.0 + hi / k as f64).min(1.0));
    for _ in 0..60 {
        let m1 = lo + (up - lo) / 3.0;
        let m2 = up - (up - lo) / 3.0;
        if eval(m, m1).aoq < eval(m, m2).aoq {
            lo = m1;
        } else {
            up = m2;
        }
    }
    let p = 0.5 * (lo + up);
    let a = eval(m, p).aoq;
    if a > best.1 {
        (p, a)
    } else {
        best
    }
}

fn plan_text(m: &AsaModel) -> String {
    let st = stage_list(&m.stages);
    match m.problem_type {
        Plan::Single => {
            let (n, c, _) = st[0];
            format!("Se toma una muestra de {n} unidades: se acepta el lote si hay {c} defectuosos o menos y se rechaza si hay más{}.", if m.curtailed { " (se deja de inspeccionar al encontrar el defectuoso número " .to_string() + &(c + 1).to_string() + ")" } else { String::new() })
        }
        Plan::Double | Plan::Multiple => {
            let parts: Vec<String> = st
                .iter()
                .enumerate()
                .map(|(i, &(n, c, r))| {
                    if i + 1 == st.len() {
                        format!("muestra {} de {n}: con el acumulado, aceptar si hay ≤ {c} defectuosos y rechazar si no", i + 1)
                    } else {
                        format!("muestra {} de {n}: aceptar si el acumulado es ≤ {c}, rechazar si es ≥ {r} y, si no, seguir", i + 1)
                    }
                })
                .collect();
            format!("{}.", capitalize(&parts.join("; ")))
        }
        Plan::Chain => format!("ChSP-1: muestra de {} por lote; con 0 defectuosos se acepta, con 2 o más se rechaza y con 1 se acepta solo si los {} lotes anteriores no tuvieron defectuosos.", st[0].0, fmt(m.clearance)),
        Plan::SkipLot => format!(
            "SkSP-2: plan de referencia n = {}, c = {}. Se inspeccionan todos los lotes hasta aceptar {} seguidos; después solo el {} % de los lotes, hasta que se rechace uno.",
            st[0].0,
            st[0].1,
            fmt(m.clearance),
            fmt(m.fraction)
        ),
        Plan::Continuous => format!("CSP-1: inspección al 100 % hasta encontrar {} unidades seguidas sin defectos; después se inspecciona el {} % de las unidades hasta encontrar un defectuoso.", fmt(m.clearance), fmt(m.fraction)),
        Plan::Sequential => "Se inspecciona unidad por unidad y se compara el número acumulado de defectuosos con las rectas de aceptación y de rechazo de Wald.".into(),
        _ => String::new(),
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn validate(m: &AsaModel) -> Result<(), ModError> {
    let p = m.problem_type;
    if !(0.0..=100.0).contains(&m.aql) || !(0.0..=100.0).contains(&m.rql) {
        return invalid("AQL y RQL son porcentajes entre 0 y 100");
    }
    if !(0.0..100.0).contains(&m.alpha) || !(0.0..100.0).contains(&m.beta) {
        return invalid("α y β son porcentajes entre 0 y 100");
    }
    if let Some(n) = m.lot_size {
        if !(n >= 1.0) {
            return invalid("el tamaño del lote debe ser positivo");
        }
    }
    if m.distribution == ProbDist::Hypergeometric && m.lot_size.is_none() && p != Plan::Continuous {
        return invalid("la distribución hipergeométrica necesita el tamaño del lote (N)");
    }
    match p {
        Plan::Single | Plan::Double | Plan::Multiple | Plan::Chain | Plan::SkipLot => {
            if m.stages.is_empty() {
                return invalid("falta el plan de muestreo (n y c)");
            }
            if p == Plan::Double && m.stages.len() != 2 {
                return invalid("el muestreo doble tiene dos etapas");
            }
            let list = stage_list(&m.stages);
            let mut tot = 0usize;
            for (i, &(n, c, r)) in list.iter().enumerate() {
                if n == 0 {
                    return invalid(format!("etapa {}: el tamaño de la muestra debe ser positivo", i + 1));
                }
                if c < -1 || r <= c {
                    return invalid(format!("etapa {}: el número de rechazo debe ser mayor que el de aceptación", i + 1));
                }
                tot += n;
            }
            if let Some(nn) = m.lot_size {
                if tot as f64 > nn {
                    return invalid("las muestras suman más que el lote");
                }
            }
            if tot > 200_000 {
                return invalid("muestras demasiado grandes");
            }
        }
        Plan::Continuous => {
            if !(m.fraction > 0.0 && m.fraction <= 100.0) {
                return invalid("el porcentaje inspeccionado f debe estar entre 0 y 100");
            }
            if m.clearance < 1.0 {
                return invalid("el número de liberación i debe ser al menos 1");
            }
        }
        _ => {}
    }
    if p == Plan::SkipLot && !(m.fraction > 0.0 && m.fraction <= 100.0) {
        return invalid("el porcentaje de lotes inspeccionados f debe estar entre 0 y 100");
    }
    Ok(())
}

/// Búsqueda del plan (simple, doble, en cadena, saltos de lotes y CSP-1).
fn search(m: &AsaModel, o: &AsaOptions) -> Result<(AsaModel, Table, String), ModError> {
    let (p1, p2) = (m.aql / 100.0, m.rql / 100.0);
    let (al, be) = (m.alpha / 100.0, m.beta / 100.0);
    if o.criterion != Criterion::Aoql && !(p2 > p1) {
        return invalid("para buscar el plan indica AQL < RQL");
    }
    let nmax = m.lot_size.map(|n| n as usize).unwrap_or(5000).min(5000);
    let mut t = Table::new(Some("Planes evaluados"), &["Plan", "α real (%)", "β real (%)", "AOQL (%)", "ASN en AQL", "Criterio"]).left(0);
    let score = |x: &AsaModel| -> (f64, f64, f64) {
        let a = 1.0 - eval(x, p1).pa;
        let b = eval(x, p2).pa;
        (a, b, eval(x, p1).asn)
    };
    let label = |x: &AsaModel| -> String {
        match x.problem_type {
            Plan::Continuous => format!("i = {}, f = {} %", fmt(x.clearance), fmt(x.fraction)),
            Plan::Double => format!("n1 = {}, c1 = {}, r1 = {}, n2 = {}, c2 = {}", x.stages[0].n, x.stages[0].c, x.stages[0].r.unwrap_or(0.0), x.stages[1].n, x.stages[1].c),
            _ => format!("n = {}, c = {}", x.stages[0].n, x.stages[0].c),
        }
    };
    let mut cands: Vec<(f64, AsaModel)> = vec![];
    let mut push = |x: AsaModel, crit: f64, t: &mut Table| {
        let (a, b, asn) = score(&x);
        let aq = aoql(&x).1;
        let cv = if crit >= 1e12 { Cell::from("no cumple α") } else { Cell::Num(tidy(crit)) };
        t.row(cells![label(&x), tidy(100.0 * a), tidy(100.0 * b), tidy(100.0 * aq), tidy(asn), cv]);
        cands.push((crit, x));
    };
    let with = |n: usize, c: i64| {
        let mut x = m.clone();
        x.stages = vec![Stage { n: n as f64, c: c as f64, r: None }];
        x
    };
    // n mínimo con Pa(RQL) ≤ β para un c dado (Pa decrece con n)
    let min_n = |c: i64| -> Option<usize> {
        let ok = |n: usize| eval(&with(n, c), p2).pa <= be;
        if !ok(nmax) {
            return None;
        }
        let (mut lo, mut hi) = ((c.max(0) as usize + 1).min(nmax), nmax);
        if ok(lo) {
            return Some(lo);
        }
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if ok(mid) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        Some(hi)
    };
    let how;
    match m.problem_type {
        Plan::Single | Plan::SkipLot | Plan::Chain => {
            let cmax = if m.problem_type == Plan::Chain { 0 } else { 30 };
            match o.criterion {
                Criterion::Risks => {
                    how = "el menor n que cumple β en RQL, para el menor c que además cumple α en AQL".to_string();
                    for c in 0..=cmax {
                        if let Some(n) = min_n(c) {
                            let x = with(n, c);
                            let (a, _, _) = score(&x);
                            let meets = a <= al + 1e-12;
                            // los que no cumplen α quedan en la tabla pero no se eligen (salvo en cadena, donde c = 0)
                            push(x, if meets || m.problem_type == Plan::Chain { n as f64 } else { 1e12 + n as f64 }, &mut t);
                            if meets || m.problem_type == Plan::Chain {
                                break;
                            }
                        }
                    }
                }
                Criterion::Weighted => {
                    how = format!("la menor desviación ponderada {}·|α real − α| + {}·|β real − β|", fmt(o.weight_alpha), fmt(o.weight_beta));
                    for c in 0..=cmax.min(15) {
                        let mut best: Option<(f64, usize)> = None;
                        let step = (nmax / 400).max(1);
                        let mut n = (c as usize + 1).max(1);
                        while n <= nmax {
                            let x = with(n, c);
                            let (a, b, _) = score(&x);
                            let d = o.weight_alpha * (a - al).abs() + o.weight_beta * (b - be).abs();
                            if best.map(|b| d < b.0).unwrap_or(true) {
                                best = Some((d, n));
                            }
                            n += step;
                        }
                        if let Some((d, n)) = best {
                            push(with(n, c), d, &mut t);
                        }
                    }
                }
                Criterion::Aoql => {
                    how = format!("el plan con AOQL ≤ {} % y la menor inspección total (ATI) en AQL", fmt(o.aoql_limit));
                    for c in 0..=cmax.min(15) {
                        // n mínimo que logra el AOQL (AOQL decrece con n)
                        let ok = |n: usize| aoql(&with(n, c)).1 * 100.0 <= o.aoql_limit;
                        if !ok(nmax) {
                            continue;
                        }
                        let (mut lo, mut hi) = (c as usize + 1, nmax);
                        if !ok(lo) {
                            while hi - lo > 1 {
                                let mid = (lo + hi) / 2;
                                if ok(mid) {
                                    hi = mid;
                                } else {
                                    lo = mid;
                                }
                            }
                        } else {
                            hi = lo;
                        }
                        let x = with(hi, c);
                        let e = eval(&x, p1);
                        push(x, e.ati.unwrap_or(e.asn), &mut t);
                    }
                }
            }
        }
        Plan::Double => {
            how = format!(
                "n2 = 2·n1 y r1 = c2 + 1; para cada (c1, c2) el menor n1 que cumple β, entre los que cumplen α el de menor ASN en AQL{}",
                if o.criterion != Criterion::Risks { " (en el muestreo doble la búsqueda siempre usa α y β)" } else { "" }
            );
            for c1 in 0..=4i64 {
                for c2 in (c1 + 1)..=(c1 + 8) {
                    let mk = |n1: usize| {
                        let mut x = m.clone();
                        x.stages = vec![Stage { n: n1 as f64, c: c1 as f64, r: Some(c2 as f64 + 1.0) }, Stage { n: 2.0 * n1 as f64, c: c2 as f64, r: None }];
                        x
                    };
                    let cap = nmax / 3;
                    let ok = |n1: usize| eval(&mk(n1), p2).pa <= be;
                    if cap < 1 || !ok(cap) {
                        continue;
                    }
                    let (mut lo, mut hi) = (1usize, cap);
                    if !ok(lo) {
                        while hi - lo > 1 {
                            let mid = (lo + hi) / 2;
                            if ok(mid) {
                                hi = mid;
                            } else {
                                lo = mid;
                            }
                        }
                    } else {
                        hi = 1;
                    }
                    let x = mk(hi);
                    let (a, _, asn) = score(&x);
                    if a <= al + 1e-12 {
                        push(x, asn, &mut t);
                    }
                }
            }
        }
        Plan::Continuous => {
            how = format!("el menor número de liberación i con AOQL ≤ {} % para f = {} %", fmt(o.aoql_limit), fmt(m.fraction));
            for i in 1..=5000 {
                let mut x = m.clone();
                x.clearance = i as f64;
                let a = aoql(&x).1;
                if a * 100.0 <= o.aoql_limit || i == 5000 {
                    push(x, i as f64, &mut t);
                    break;
                }
            }
        }
        _ => return invalid("WinQSB no busca planes múltiples ni secuenciales: el secuencial queda definido por AQL, RQL, α y β"),
    }
    let best = cands.iter().min_by(|a, b| a.0.partial_cmp(&b.0).unwrap()).map(|x| x.1.clone());
    match best {
        Some(b) => {
            let lab = label(&b);
            for (i, (_, x)) in cands.iter().enumerate() {
                if label(x) == lab {
                    if let Some(r) = t.rows.get_mut(i) {
                        r.tone = Tone::Highlight;
                    }
                }
            }
            Ok((b, t, how))
        }
        None => invalid("no se encontró un plan que cumpla las condiciones (prueba con un lote mayor o riesgos menos exigentes)"),
    }
}

pub fn solve(m0: &AsaModel, o: &AsaOptions) -> Result<Report, ModError> {
    validate(m0)?;
    let mut blocks_search = vec![];
    let m = if o.mode == Mode::Search && m0.problem_type != Plan::Multiple && m0.problem_type != Plan::Sequential {
        let (b, t, how) = search(m0, o)?;
        blocks_search.push(text(format!("Búsqueda: {how}.")));
        blocks_search.push(t.block());
        b
    } else {
        m0.clone()
    };
    let (p1, p2) = (m.aql / 100.0, m.rql / 100.0);
    let (al, be) = (m.alpha / 100.0, m.beta / 100.0);
    let e1 = eval(&m, p1);
    let e2 = eval(&m, p2);
    let (aoql_p, aoql_v) = aoql(&m);
    let aql_for = p_for(&m, 1.0 - al);
    let rql_for = p_for(&m, be);
    let csp = m.problem_type == Plan::Continuous;
    // curvas
    let hi = o.to.map(|x| x / 100.0).unwrap_or((m.rql * 2.0).max(m.aql * 4.0).max(aoql_p * 100.0 * 2.5).max(2.0).min(100.0) / 100.0);
    let lo = o.from.map(|x| x / 100.0).unwrap_or(0.0);
    let k = o.points.clamp(4, 400) as usize;
    let mut cols = vec!["% defectuoso", if csp { "Fracción aceptada sin inspección total" } else { "Pa" }, "AOQ (%)"];
    if csp {
        cols.push("AFI (%)");
    } else {
        cols.push("ASN");
        if m.lot_size.is_some() {
            cols.push("ATI");
        }
    }
    let has_cost = e1.cost.is_some();
    if has_cost {
        cols.push("Costo total por lote");
    }
    let mut ct = Table::new(Some("Desempeño del plan"), &cols);
    let (mut oc, mut aoq, mut asn, mut ati, mut cost) = (vec![], vec![], vec![], vec![], vec![]);
    for i in 0..=k {
        let p = lo + (hi - lo) * i as f64 / k as f64;
        let e = eval(&m, p);
        let x = 100.0 * p;
        oc.push([x, e.pa]);
        aoq.push([x, 100.0 * e.aoq]);
        asn.push([x, if csp { 100.0 * e.afi.unwrap_or(0.0) } else { e.asn }]);
        if let Some(a) = e.ati {
            ati.push([x, a]);
        }
        if let Some(c) = e.cost {
            cost.push([x, c]);
        }
        let mut row = cells![tidy(x), tidy(e.pa), tidy(100.0 * e.aoq)];
        if csp {
            row.push(Cell::Num(tidy(100.0 * e.afi.unwrap_or(0.0))));
        } else {
            row.push(Cell::Num(tidy(e.asn)));
            if m.lot_size.is_some() {
                row.push(e.ati.map(|a| Cell::Num(tidy(a))).unwrap_or(Cell::Empty));
            }
        }
        if has_cost {
            row.push(e.cost.map(|a| Cell::Num(tidy(a))).unwrap_or(Cell::Empty));
        }
        ct.row(row);
    }
    let pct = |x: f64| tidy(100.0 * x);
    let mut mets = vec![
        metric_main("α real (%)", pct(1.0 - e1.pa)),
        metric("β real (%)", pct(e2.pa)),
        metric(format!("AOQL (%) en p = {} %", fmt(pct(aoql_p))), pct(aoql_v)),
    ];
    if csp {
        mets.push(metric("AFI en AQL (%)", pct(e1.afi.unwrap_or(0.0))));
    } else {
        mets.push(metric("ASN en AQL", tidy(e1.asn)));
        if let Some(a) = e1.ati {
            mets.push(metric("ATI en AQL", tidy(a)));
        }
    }
    if let Some(c) = e1.cost {
        mets.push(metric("Costo por lote en AQL", tidy(c)));
    }
    let mut sol = vec![metrics(mets), text(plan_text(&m))];
    let mut rt = Table::new(Some("Riesgos y niveles de calidad"), &["Medida", "Valor"]).left(0);
    rt.row(cells![format!("Probabilidad de aceptar en AQL = {} %", fmt(m.aql)), tidy(e1.pa)]);
    rt.row(cells!["Riesgo del productor α real (%)", pct(1.0 - e1.pa)]);
    rt.row(cells![format!("Probabilidad de aceptar en RQL = {} % (riesgo del consumidor β real, %)", fmt(m.rql)), pct(e2.pa)]);
    rt.row(cells![format!("AQL que da α = {} % (%)", fmt(m.alpha)), aql_for.map(|x| Cell::Num(pct(x))).unwrap_or(Cell::Empty)]);
    rt.row(cells![format!("RQL que da β = {} % (%)", fmt(m.beta)), rql_for.map(|x| Cell::Num(pct(x))).unwrap_or(Cell::Empty)]);
    rt.row(cells!["AOQL (%)", pct(aoql_v)]);
    sol.push(rt.block());
    if let Some(w) = o.what_if {
        let e = eval(&m, w / 100.0);
        let mut wt = Table::new(Some(&format!("Qué pasa si el lote tiene {} % defectuoso", fmt(w))), &["Medida", "Valor"]).left(0);
        wt.row(cells!["Probabilidad de aceptación", tidy(e.pa)]);
        wt.row(cells!["Calidad de salida AOQ (%)", pct(e.aoq)]);
        if csp {
            wt.row(cells!["Fracción inspeccionada AFI (%)", pct(e.afi.unwrap_or(0.0))]);
        } else {
            wt.row(cells!["Muestra media ASN", tidy(e.asn)]);
        }
        if let Some(a) = e.ati {
            wt.row(cells!["Inspección total media ATI", tidy(a)]);
        }
        if let Some(c) = e.cost {
            wt.row(cells!["Costo total por lote", tidy(c)]);
        }
        sol.push(wt.block());
    }
    // gráficas
    let mut charts = vec![];
    let ocl = if csp { "Fracción aceptada" } else { "Probabilidad de aceptación" };
    charts.push(
        Chart::new(Some("Curva característica de operación (OC)"), "% defectuoso", ocl)
            .series(Series::new("OC", SeriesKind::Line, oc).tone(Tone::Accent))
            .vline(m.aql, "AQL", Tone::Good)
            .vline(m.rql, "RQL", Tone::Bad)
            .hline(1.0 - al, "1 − α", Tone::Good)
            .hline(be, "β", Tone::Bad)
            .block(),
    );
    charts.push(Chart::new(Some("Calidad media de salida (AOQ)"), "% defectuoso", "AOQ (%)").series(Series::new("AOQ", SeriesKind::Line, aoq).tone(Tone::Highlight)).hline(pct(aoql_v), "AOQL", Tone::Bad).block());
    charts.push(Chart::new(Some(if csp { "Fracción media inspeccionada (AFI)" } else { "Muestra media (ASN)" }), "% defectuoso", if csp { "AFI (%)" } else { "ASN" }).series(Series::new(if csp { "AFI" } else { "ASN" }, SeriesKind::Line, asn).tone(Tone::Accent)).block());
    if !ati.is_empty() {
        charts.push(Chart::new(Some("Inspección total media (ATI)"), "% defectuoso", "ATI").series(Series::new("ATI", SeriesKind::Line, ati).tone(Tone::Accent)).block());
    }
    if !cost.is_empty() {
        charts.push(Chart::new(Some("Costo total medio por lote"), "% defectuoso", "Costo").series(Series::new("Costo", SeriesKind::Line, cost).tone(Tone::Highlight)).block());
    }
    // proceso secuencial
    let mut seq_blocks = vec![];
    let mut seq_data = Value::Null;
    if m.problem_type == Plan::Sequential {
        let w = wald(&m)?;
        let obs: Vec<f64> = o.observations.split([',', ';', ' ', '\n']).map(str::trim).filter(|s| !s.is_empty()).filter_map(|s| s.parse::<f64>().ok()).collect();
        let nmax = obs.len().max(((w.h2 + w.h1) / w.s.max(1e-6) * 1.5).ceil() as usize).clamp(10, 2000);
        let acc_line: Vec<[f64; 2]> = (1..=nmax).map(|n| [n as f64, -w.h1 + w.s * n as f64]).collect();
        let rej_line: Vec<[f64; 2]> = (1..=nmax).map(|n| [n as f64, w.h2 + w.s * n as f64]).collect();
        let mut ch = Chart::new(Some("Rectas de aceptación y de rechazo"), "Unidades inspeccionadas (n)", "Defectuosos acumulados (x)")
            .series(Series::new("Aceptación: x ≤ −h1 + s·n", SeriesKind::Line, acc_line).tone(Tone::Good))
            .series(Series::new("Rechazo: x ≥ h2 + s·n", SeriesKind::Line, rej_line).tone(Tone::Bad));
        let mut lt = Table::new(Some("Parámetros de Wald"), &["Parámetro", "Valor"]).left(0);
        lt.row(cells!["Pendiente s", tidy(w.s)]);
        lt.row(cells!["h1 (aceptación)", tidy(w.h1)]);
        lt.row(cells!["h2 (rechazo)", tidy(w.h2)]);
        let first_acc = (1..=100_000).find(|&n| (-w.h1 + w.s * n as f64) >= 0.0);
        if let Some(n) = first_acc {
            lt.row(cells!["Menor n para aceptar (sin defectuosos)", n as f64]);
        }
        lt.row(cells!["Menor n para rechazar", w.h2.ceil().max(1.0)]);
        seq_blocks.push(lt.block());
        if !obs.is_empty() {
            let mut pt = Table::new(Some("Proceso secuencial"), &["n", "Defectuosos acumulados", "Límite de aceptación", "Límite de rechazo", "Decisión"]).left(4);
            let mut x = 0.0;
            let mut path = vec![];
            let mut decision = "seguir muestreando".to_string();
            for (i, d) in obs.iter().enumerate() {
                x += d;
                let n = (i + 1) as f64;
                let (xa, xr) = (-w.h1 + w.s * n, w.h2 + w.s * n);
                let dec = if x <= xa {
                    "aceptar"
                } else if x >= xr {
                    "rechazar"
                } else {
                    "seguir"
                };
                path.push([n, x]);
                pt.row_tone(cells![n, x, tidy(xa), tidy(xr), dec], if dec == "seguir" { Tone::Normal } else if dec == "aceptar" { Tone::Good } else { Tone::Bad });
                if dec != "seguir" {
                    decision = format!("{dec} el lote en la unidad {}", i + 1);
                    break;
                }
            }
            ch = ch.series(Series::new("Datos", SeriesKind::Line, path).tone(Tone::Accent));
            seq_blocks.insert(0, text(format!("Con los datos observados: {decision}.")));
            seq_blocks.push(pt.block());
            seq_data = json!({"decision": decision});
        }
        seq_blocks.insert(0, ch.block());
    }
    // paso a paso
    let mut st = vec![];
    if o.steps {
        let ctx = Ctx::new(&m, p1);
        let how = match m.problem_type {
            Plan::Single => {
                let (n, c, _) = stage_list(&m.stages)[0];
                let pm = ctx.pmf(n, 0, 0);
                let terms: Vec<String> = (0..=(c.max(0) as usize).min(n)).map(|x| format!("P({x}) = {}", fmt(tidy(pm[x])))).collect();
                format!("Pa = P(x ≤ {c}) con x ~ {} (n = {n}, p = {} %): {} → Pa = {}.", dist_label(ctx.effective(n)), fmt(m.aql), terms.join(", "), fmt(tidy(e1.pa)))
            }
            Plan::Double | Plan::Multiple => "Pa = suma de las probabilidades de aceptar en cada etapa: en la etapa k se acepta si el acumulado es ≤ ck; se sigue si queda entre ck y rk. La distribución del acumulado se obtiene convolucionando las de cada muestra.".into(),
            Plan::Chain => format!("Pa = P(0) + P(1)·P(0)^i con i = {}.", fmt(m.clearance)),
            Plan::SkipLot => "Con P = Pa del plan de referencia: Pa = (f·P + (1 − f)·P^i)/(f + (1 − f)·P^i); fracción media de lotes inspeccionados F = f/((1 − f)·P^i + f).".into(),
            Plan::Continuous => "Con q = 1 − p: u = (1 − q^i)/(p·q^i) unidades en inspección total y v = 1/(f·p) en muestreo; AFI = (u + f·v)/(u + v), fracción aceptada = v/(u + v) y AOQ = p·(1 − AFI).".into(),
            Plan::Sequential => {
                let w = wald(&m)?;
                format!(
                    "K = ln[RQL(1 − AQL)/(AQL(1 − RQL))], s = ln[(1 − AQL)/(1 − RQL)]/K = {}, h1 = ln[(1 − α)/β]/K = {}, h2 = ln[(1 − β)/α]/K = {}. Se acepta si x ≤ −h1 + s·n y se rechaza si x ≥ h2 + s·n. La OC y el ASN salen de las fórmulas de Wald.",
                    fmt(tidy(w.s)),
                    fmt(tidy(w.h1)),
                    fmt(tidy(w.h2))
                )
            }
            _ => String::new(),
        };
        let mut v = vec![step("Probabilidad de aceptación", how, vec![])];
        if m.error_good != 0.0 || m.error_bad != 0.0 {
            v.insert(0, step("Errores de inspección", format!("Fracción aparente pe = p(1 − e2) + (1 − p)e1 = {} % en AQL; con ella se calculan las probabilidades.", fmt(pct(ctx.pe))), vec![]));
        }
        v.push(step(
            "Medidas del plan",
            if m.rectified { "AOQ = Pa·p·(N − n)/N (los lotes rechazados se inspeccionan al 100 % y los defectuosos se reemplazan); ATI = n + (1 − Pa)(N − n); ASN = muestra media; costo = ATI·C(I) + ASN·C(S) + n(d)·C(P) + N(d)·C(C)." } else { "AOQ = Pa·p·(N − n)/(N − n·p) (los defectuosos encontrados no se reemplazan); ATI = n + (1 − Pa)(N − n); ASN = muestra media; costo = ATI·C(I) + ASN·C(S) + n(d)·C(P) + N(d)·C(C)." },
            vec![],
        ));
        v.push(step("Riesgos", format!("α = 1 − Pa(AQL) = {} %; β = Pa(RQL) = {} %. AOQL = máximo de la curva AOQ = {} %.", fmt(pct(1.0 - e1.pa)), fmt(pct(e2.pa)), fmt(pct(aoql_v))), vec![]));
        st.push(steps(v));
    }
    let headline = match m.problem_type {
        Plan::Continuous => format!("AOQL {} %; en AQL se inspecciona el {} % de las unidades.", fmt(pct(aoql_v)), fmt(pct(e1.afi.unwrap_or(0.0)))),
        _ => format!("α real {} % y β real {} %; AOQL {} %.", fmt(pct(1.0 - e1.pa)), fmt(pct(e2.pa)), fmt(pct(aoql_v))),
    };
    let headline = if o.mode == Mode::Search && !blocks_search.is_empty() { format!("Plan encontrado: {}. {headline}", plan_short(&m)) } else { headline };
    let mut r = Report::new("asa", &m.name, Status::Solved, headline).section("solucion", "Solución", sol);
    if !blocks_search.is_empty() {
        r = r.section("busqueda", "Búsqueda", blocks_search);
    }
    if !seq_blocks.is_empty() {
        r = r.section("secuencial", "Proceso secuencial", seq_blocks);
    }
    r = r.section("curvas", "Curvas", charts).section("tabla", "Tabla", vec![ct.block()]).section("pasos", "Paso a paso", st);
    let plan = serde_json::to_value(&m.stages).unwrap_or(Value::Null);
    Ok(r.data(json!({
        "pa_aql": tidy(e1.pa), "alpha": pct(1.0 - e1.pa), "beta": pct(e2.pa), "aoql": pct(aoql_v), "aoql_at": pct(aoql_p),
        "asn_aql": tidy(e1.asn), "ati_aql": e1.ati.map(tidy), "cost_aql": e1.cost.map(tidy), "afi_aql": e1.afi.map(|x| pct(x)),
        "aql_for_alpha": aql_for.map(pct), "rql_for_beta": rql_for.map(pct), "plan": plan, "clearance": m.clearance, "fraction": m.fraction, "sequential": seq_data,
    })))
}

fn plan_short(m: &AsaModel) -> String {
    match m.problem_type {
        Plan::Continuous => format!("i = {}, f = {} %", fmt(m.clearance), fmt(m.fraction)),
        Plan::Double => format!("n1 = {}, c1 = {}, r1 = {}, n2 = {}, c2 = {}", m.stages[0].n, m.stages[0].c, m.stages[0].r.unwrap_or(0.0), m.stages[1].n, m.stages[1].c),
        _ => format!("n = {}, c = {}", m.stages[0].n, m.stages[0].c),
    }
}

fn dist_label(d: ProbDist) -> &'static str {
    match d {
        ProbDist::Binomial | ProbDist::Auto => "binomial",
        ProbDist::Hypergeometric => "hipergeométrica",
        ProbDist::Poisson => "Poisson",
        ProbDist::Normal => "normal (aproximación con corrección de continuidad)",
    }
}
