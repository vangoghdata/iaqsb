//! Muestreo de aceptación por variables: media de la muestra, fracción
//! defectuosa por los métodos k y M, y muestreo secuencial (Wald) para la media.
//! Con σ conocida se usa la normal; si σ es la de la muestra, la t de Student
//! (media) o la aproximación de Wallis (métodos k y M).

use super::{AsaModel, AsaOptions, Mode, Plan, Side};
use iaqsb_core::*;
use serde_json::{json, Value};
use statrs::distribution::{ContinuousCDF, Normal, StudentsT};

fn phi(z: f64) -> f64 {
    Normal::new(0.0, 1.0).unwrap().cdf(z)
}
fn zq(p: f64) -> f64 {
    Normal::new(0.0, 1.0).unwrap().inverse_cdf(p.clamp(1e-15, 1.0 - 1e-15))
}
/// CDF de la referencia: normal (σ conocida) o t con n − 1 grados (σ de la muestra).
fn cdf_ref(z: f64, n: f64, known: bool) -> f64 {
    if known || n < 2.0 {
        phi(z)
    } else {
        StudentsT::new(0.0, 1.0, n - 1.0).map(|t| t.cdf(z)).unwrap_or(phi(z))
    }
}
fn q_ref(p: f64, n: f64, known: bool) -> f64 {
    if known || n < 2.0 {
        zq(p)
    } else {
        StudentsT::new(0.0, 1.0, n - 1.0).map(|t| t.inverse_cdf(p.clamp(1e-15, 1.0 - 1e-15))).unwrap_or(zq(p))
    }
}

/// Fracción fuera de especificación cuando la media del proceso es μ.
fn frac(m: &AsaModel, mu: f64) -> Option<f64> {
    let s = m.sigma;
    match m.side {
        Side::Lower => m.lsl.map(|l| phi((l - mu) / s)),
        Side::Upper => m.usl.map(|u| 1.0 - phi((u - mu) / s)),
        Side::Both => match (m.lsl, m.usl) {
            (Some(l), Some(u)) => Some(phi((l - mu) / s) + 1.0 - phi((u - mu) / s)),
            _ => None,
        },
    }
}

/// Media del proceso que da una fracción defectuosa p (rama superior si hay dos límites).
fn mu_for(m: &AsaModel, p: f64) -> Option<f64> {
    let s = m.sigma;
    let p = p.clamp(1e-12, 1.0 - 1e-12);
    match m.side {
        Side::Lower => m.lsl.map(|l| l - zq(p) * s),
        Side::Upper => m.usl.map(|u| u - zq(1.0 - p) * s),
        Side::Both => {
            let (l, u) = (m.lsl?, m.usl?);
            let c = 0.5 * (l + u);
            if frac(m, c)? >= p {
                return Some(c);
            }
            let (mut lo, mut hi) = (c, u + 40.0 * s);
            for _ in 0..200 {
                let mid = 0.5 * (lo + hi);
                if frac(m, mid)? < p {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            Some(0.5 * (lo + hi))
        }
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct VEval {
    pub pa: f64,
    pub asn: f64,
    pub p: Option<f64>,
    pub aoq: Option<f64>,
    pub ati: Option<f64>,
    pub cost: Option<f64>,
}

/// Constante de los métodos k y M (el M se convierte en un k equivalente).
fn k_of(m: &AsaModel) -> f64 {
    match m.problem_type {
        Plan::MMethod => {
            let c = (m.n / (m.n - 1.0).max(1e-9)).sqrt();
            zq(1.0 - m.m / 100.0) / c
        }
        _ => m.k,
    }
}

/// Wald para la media normal: H0 = θ0 (buena) contra H1 = θ1 (mala).
fn wald_normal(mu: f64, t0: f64, t1: f64, sigma: f64, al: f64, be: f64) -> (f64, f64) {
    let a = (1.0 - be) / al;
    let b = be / (1.0 - al);
    let (la, lb) = (a.ln(), b.ln());
    let h = (t1 + t0 - 2.0 * mu) / (t1 - t0);
    let pa = if h.abs() < 1e-9 { la / (la - lb) } else { (a.powf(h) - 1.0) / (a.powf(h) - b.powf(h)) };
    let pa = if pa.is_finite() { pa.clamp(0.0, 1.0) } else if h > 0.0 { 1.0 } else { 0.0 };
    let ez = (t1 - t0) * (2.0 * mu - t0 - t1) / (2.0 * sigma * sigma);
    let asn = if ez.abs() < 1e-12 { -la * lb / ((t1 - t0) * (t1 - t0) / (sigma * sigma)) } else { (pa * lb + (1.0 - pa) * la) / ez };
    (pa, asn.max(1.0))
}

pub fn eval(m: &AsaModel, mu: f64) -> VEval {
    let s = m.sigma;
    let n = m.n.max(1.0);
    let rn = n.sqrt();
    let known = m.sigma_known;
    let (al, be) = (m.alpha / 100.0, m.beta / 100.0);
    let mut e = VEval::default();
    match m.problem_type {
        Plan::Mean => {
            let up = |x: f64| cdf_ref((x - mu) * rn / s, n, known);
            e.pa = match m.side {
                Side::Lower => 1.0 - up(m.xla.unwrap_or(f64::NEG_INFINITY)),
                Side::Upper => up(m.xua.unwrap_or(f64::INFINITY)),
                Side::Both => (up(m.xua.unwrap_or(f64::INFINITY)) - up(m.xla.unwrap_or(f64::NEG_INFINITY))).max(0.0),
            };
            e.asn = n;
        }
        Plan::KMethod | Plan::MMethod => {
            let k = k_of(m);
            if known && m.problem_type == Plan::MMethod && m.side == Side::Both {
                // región de aceptación de la media: p̂(x) = [1 − Φ(C(x − LSL)/σ)] + [1 − Φ(C(USL − x)/σ)] ≤ M
                let (l, u) = (m.lsl.unwrap_or(0.0), m.usl.unwrap_or(0.0));
                let c = (n / (n - 1.0).max(1e-9)).sqrt();
                let ph = |x: f64| (1.0 - phi(c * (x - l) / s)) + (1.0 - phi(c * (u - x) / s));
                let mid = 0.5 * (l + u);
                let mm = m.m / 100.0;
                e.pa = if ph(mid) > mm {
                    0.0
                } else {
                    let root = |mut a: f64, mut b: f64| {
                        for _ in 0..200 {
                            let x = 0.5 * (a + b);
                            if (ph(x) > mm) == (ph(a) > mm) {
                                a = x;
                            } else {
                                b = x;
                            }
                        }
                        0.5 * (a + b)
                    };
                    let lo = root(l - 40.0 * s, mid);
                    let hi = root(mid, u + 40.0 * s);
                    (phi((hi - mu) * rn / s) - phi((lo - mu) * rn / s)).max(0.0)
                };
            } else {
                // σ conocida: exacto; σ de la muestra: aproximación de Wallis
                let v = if known { 1.0 / n } else { 1.0 / n + k * k / (2.0 * n) };
                let sd = v.sqrt();
                let side = |z: f64| phi((z - k) / sd);
                e.pa = match m.side {
                    Side::Lower => side((mu - m.lsl.unwrap_or(0.0)) / s),
                    Side::Upper => side((m.usl.unwrap_or(0.0) - mu) / s),
                    Side::Both => {
                        let zl = (mu - m.lsl.unwrap_or(0.0)) / s;
                        let zu = (m.usl.unwrap_or(0.0) - mu) / s;
                        if known {
                            (phi((zu - k) * rn) + phi((zl - k) * rn) - 1.0).max(0.0)
                        } else {
                            (side(zu) + side(zl) - 1.0).max(0.0)
                        }
                    }
                };
            }
            e.asn = n;
        }
        Plan::SequentialVariable => {
            let (pa, asn) = match m.side {
                Side::Lower => wald_normal(mu, m.x1, m.x2l.unwrap_or(m.x1 - s), s, al, be),
                Side::Upper => wald_normal(mu, m.x1, m.x2u.unwrap_or(m.x1 + s), s, al, be),
                Side::Both => {
                    let (xl, xu) = (m.x2l.unwrap_or(m.x1 - s), m.x2u.unwrap_or(m.x1 + s));
                    let c = 0.5 * (xl + xu);
                    let (pu, nu) = wald_normal(mu, c, xu, s, al / 2.0, be);
                    let (pl, nl) = wald_normal(mu, c, xl, s, al / 2.0, be);
                    ((pu + pl - 1.0).max(0.0), nu.max(nl))
                }
            };
            e.pa = pa;
            e.asn = asn;
        }
        _ => {}
    }
    e.p = frac(m, mu);
    if let Some(p) = e.p {
        match m.lot_size {
            Some(nn) => {
                e.aoq = Some(e.pa * p * (nn - e.asn).max(0.0) / nn);
                let ati = e.asn * e.pa + (1.0 - e.pa) * nn;
                e.ati = Some(ati);
                if m.sampling_cost != 0.0 || m.inspection_cost != 0.0 || m.producer_cost != 0.0 || m.consumer_cost != 0.0 {
                    let found = p * ati;
                    e.cost = Some(ati * m.inspection_cost + e.asn * m.sampling_cost + found * m.producer_cost + (nn * p - found).max(0.0) * m.consumer_cost);
                }
            }
            None => e.aoq = Some(e.pa * p),
        }
    }
    e
}

fn validate(m: &AsaModel) -> Result<(), ModError> {
    if !(m.sigma > 0.0) {
        return invalid("la desviación estándar σ debe ser positiva");
    }
    if !(m.alpha > 0.0 && m.alpha < 100.0 && m.beta > 0.0 && m.beta < 100.0) {
        return invalid("α y β son porcentajes entre 0 y 100");
    }
    let need = |o: Option<f64>, what: &str| o.map(|_| ()).ok_or_else(|| ModError::Invalid(format!("falta {what}")));
    match m.problem_type {
        Plan::Mean => match m.side {
            Side::Lower => need(m.x2l, "la media rechazable (RQL)")?,
            Side::Upper => need(m.x2u, "la media rechazable (RQL)")?,
            Side::Both => {
                need(m.x2l, "la media rechazable inferior")?;
                need(m.x2u, "la media rechazable superior")?;
            }
        },
        Plan::KMethod | Plan::MMethod => {
            match m.side {
                Side::Lower => need(m.lsl, "el límite inferior de especificación (LSL)")?,
                Side::Upper => need(m.usl, "el límite superior de especificación (USL)")?,
                Side::Both => {
                    need(m.lsl, "el límite inferior de especificación (LSL)")?;
                    need(m.usl, "el límite superior de especificación (USL)")?;
                    if m.usl <= m.lsl {
                        return invalid("USL debe ser mayor que LSL");
                    }
                }
            }
            if !(m.aql > 0.0 && m.rql > m.aql && m.rql < 100.0) {
                return invalid("indica 0 < AQL < RQL < 100 (% defectuoso)");
            }
            if m.problem_type == Plan::MMethod && !(m.m > 0.0 && m.m < 100.0) {
                return invalid("M es un porcentaje entre 0 y 100");
            }
        }
        Plan::SequentialVariable => match m.side {
            Side::Lower => {
                need(m.x2l, "la media rechazable (RQL/XL)")?;
                if m.x2l.unwrap() >= m.x1 {
                    return invalid("con límite inferior la media buena (XU) debe ser mayor que la mala (XL)");
                }
            }
            Side::Upper => {
                need(m.x2u, "la media rechazable (RQL/XU)")?;
                if m.x2u.unwrap() <= m.x1 {
                    return invalid("con límite superior la media buena (XL) debe ser menor que la mala (XU)");
                }
            }
            Side::Both => {
                need(m.x2l, "la media mínima (XL)")?;
                need(m.x2u, "la media máxima (XU)")?;
                if m.x2u <= m.x2l {
                    return invalid("XU debe ser mayor que XL");
                }
            }
        },
        _ => {}
    }
    Ok(())
}

/// Plan que cumple α y β (fórmulas clásicas; con σ de la muestra se ajusta n).
fn search(m: &AsaModel) -> Result<(AsaModel, String), ModError> {
    let mut x = m.clone();
    let (al, be) = (m.alpha / 100.0, m.beta / 100.0);
    let s = m.sigma;
    match m.problem_type {
        Plan::Mean => {
            let two = m.side == Side::Both;
            let a_side = if two { al / 2.0 } else { al };
            let d = match m.side {
                Side::Lower => m.x1 - m.x2l.unwrap(),
                Side::Upper => m.x2u.unwrap() - m.x1,
                Side::Both => (m.x1 - m.x2l.unwrap()).min(m.x2u.unwrap() - m.x1),
            };
            if !(d > 0.0) {
                return invalid("la media aceptable debe estar del lado bueno de la rechazable");
            }
            let need = |n: f64| q_ref(1.0 - a_side, n, m.sigma_known) + q_ref(1.0 - be, n, m.sigma_known) <= d * n.sqrt() / s;
            let mut n = 2.0;
            while !need(n) && n < 1e6 {
                n += 1.0;
            }
            x.n = n;
            let off = q_ref(1.0 - a_side, n, m.sigma_known) * s / n.sqrt();
            match m.side {
                Side::Lower => x.xla = Some(m.x1 - off),
                Side::Upper => x.xua = Some(m.x1 + off),
                Side::Both => {
                    x.xla = Some(m.x1 - off);
                    x.xua = Some(m.x1 + off);
                }
            }
            Ok((x, "n = ⌈((z_α + z_β)·σ/|X1 − X2|)²⌉ y límite = X1 ∓ z_α·σ/√n (con dos límites, α/2 en cada lado)".into()))
        }
        Plan::KMethod | Plan::MMethod => {
            let (p1, p2) = (m.aql / 100.0, m.rql / 100.0);
            let (z1, z2) = (zq(1.0 - p1), zq(1.0 - p2));
            let (za, zb) = (zq(1.0 - al), zq(1.0 - be));
            let k = (z1 * zb + z2 * za) / (za + zb);
            let mut n = ((za + zb) / (z1 - z2)).powi(2);
            if !m.sigma_known {
                n *= 1.0 + k * k / 2.0;
            }
            let n = n.ceil().max(2.0);
            x.n = n;
            x.k = k;
            if m.problem_type == Plan::MMethod {
                let c = (n / (n - 1.0)).sqrt();
                x.m = 100.0 * (1.0 - phi(k * c));
            }
            Ok((x, "k = (z_AQL·z_β + z_RQL·z_α)/(z_α + z_β) y n = ((z_α + z_β)/(z_AQL − z_RQL))², multiplicado por (1 + k²/2) si σ es la de la muestra; M = 1 − Φ(k·√(n/(n − 1)))".into()))
        }
        _ => invalid("este plan no tiene búsqueda"),
    }
}

pub fn solve(m0: &AsaModel, o: &AsaOptions) -> Result<Report, ModError> {
    validate(m0)?;
    let mut search_blocks = vec![];
    let m = if o.mode == Mode::Search && m0.problem_type != Plan::SequentialVariable {
        let (x, how) = search(m0)?;
        search_blocks.push(text(format!("Búsqueda: {how}.")));
        x
    } else {
        m0.clone()
    };
    let p = m.problem_type;
    if matches!(p, Plan::Mean | Plan::KMethod | Plan::MMethod) && !(m.n >= 2.0) {
        return invalid("el tamaño de la muestra debe ser al menos 2");
    }
    if p == Plan::Mean {
        match m.side {
            Side::Lower if m.xla.is_none() => return invalid("falta el límite de aceptación XLa (o usa la búsqueda)"),
            Side::Upper if m.xua.is_none() => return invalid("falta el límite de aceptación XUa (o usa la búsqueda)"),
            Side::Both if m.xla.is_none() || m.xua.is_none() => return invalid("faltan los límites de aceptación (o usa la búsqueda)"),
            _ => {}
        }
    }
    let (al, be) = (m.alpha / 100.0, m.beta / 100.0);
    let s = m.sigma;
    let by_p = matches!(p, Plan::KMethod | Plan::MMethod);
    // riesgos reales
    let (mu_good, mu_bad): (f64, Vec<f64>) = if by_p {
        (mu_for(&m, m.aql / 100.0).unwrap_or(0.0), vec![mu_for(&m, m.rql / 100.0).unwrap_or(0.0)])
    } else {
        let bad = match m.side {
            Side::Lower => vec![m.x2l.unwrap()],
            Side::Upper => vec![m.x2u.unwrap()],
            Side::Both => vec![m.x2l.unwrap(), m.x2u.unwrap()],
        };
        let good = if p == Plan::SequentialVariable && m.side == Side::Both { 0.5 * (bad[0] + bad[1]) } else { m.x1 };
        (good, bad)
    };
    let eg = eval(&m, mu_good);
    let ebs: Vec<VEval> = mu_bad.iter().map(|&x| eval(&m, x)).collect();
    let alpha_r = 1.0 - eg.pa;
    let beta_r = ebs.iter().map(|e| e.pa).fold(0.0, f64::max);
    let pct = |x: f64| tidy(100.0 * x);
    // curvas: contra la media (media y secuencial) o contra el % defectuoso (k y M)
    let k = o.points.clamp(4, 400) as usize;
    let xs: Vec<f64> = if by_p {
        let hi = o.to.unwrap_or((m.rql * 2.0).max(m.aql * 4.0).min(100.0));
        let lo = o.from.unwrap_or(0.0);
        (0..=k).map(|i| lo + (hi - lo) * i as f64 / k as f64).collect()
    } else {
        let all: Vec<f64> = mu_bad.iter().copied().chain([mu_good, m.xla.unwrap_or(mu_good), m.xua.unwrap_or(mu_good)]).collect();
        let span = s / m.n.max(1.0).sqrt() * 3.0 + if p == Plan::SequentialVariable { s } else { 0.0 };
        let lo = o.from.unwrap_or(all.iter().cloned().fold(f64::INFINITY, f64::min) - span);
        let hi = o.to.unwrap_or(all.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + span);
        (0..=k).map(|i| lo + (hi - lo) * i as f64 / k as f64).collect()
    };
    let xlab = if by_p { "% defectuoso" } else { "Media del proceso (X̄)" };
    let mut cols = vec![xlab, "Pa"];
    if !by_p {
        cols.push("% defectuoso");
    }
    cols.extend(["AOQ (%)", "ASN"]);
    if m.lot_size.is_some() {
        cols.push("ATI");
    }
    let has_cost = eg.cost.is_some();
    if has_cost {
        cols.push("Costo total por lote");
    }
    let mut tt = Table::new(Some("Desempeño del plan"), &cols);
    let (mut oc, mut aoq, mut asn, mut ati, mut cost) = (vec![], vec![], vec![], vec![], vec![]);
    let mut aoql = (0.0f64, 0.0f64);
    for &x in &xs {
        let mu = if by_p { mu_for(&m, x / 100.0).unwrap_or(0.0) } else { x };
        let e = eval(&m, mu);
        oc.push([x, e.pa]);
        if let Some(a) = e.aoq {
            aoq.push([x, 100.0 * a]);
            if a > aoql.1 {
                aoql = (x, a);
            }
        }
        asn.push([x, e.asn]);
        if let Some(a) = e.ati {
            ati.push([x, a]);
        }
        if let Some(c) = e.cost {
            cost.push([x, c]);
        }
        let mut row = cells![tidy(x), tidy(e.pa)];
        if !by_p {
            row.push(e.p.map(|v| Cell::Num(pct(v))).unwrap_or(Cell::Empty));
        }
        row.push(e.aoq.map(|v| Cell::Num(pct(v))).unwrap_or(Cell::Empty));
        row.push(Cell::Num(tidy(e.asn)));
        if m.lot_size.is_some() {
            row.push(e.ati.map(|v| Cell::Num(tidy(v))).unwrap_or(Cell::Empty));
        }
        if has_cost {
            row.push(e.cost.map(|v| Cell::Num(tidy(v))).unwrap_or(Cell::Empty));
        }
        tt.row(row);
    }
    // texto del plan
    let desc = match p {
        Plan::Mean => match m.side {
            Side::Lower => format!("Se toma una muestra de {} y se acepta el lote si la media X̄ ≥ XLa = {}.", fmt(m.n), fmt(tidy(m.xla.unwrap()))),
            Side::Upper => format!("Se toma una muestra de {} y se acepta el lote si la media X̄ ≤ XUa = {}.", fmt(m.n), fmt(tidy(m.xua.unwrap()))),
            Side::Both => format!("Se toma una muestra de {} y se acepta el lote si {} ≤ X̄ ≤ {}.", fmt(m.n), fmt(tidy(m.xla.unwrap())), fmt(tidy(m.xua.unwrap()))),
        },
        Plan::KMethod => format!(
            "Se toma una muestra de {} y se acepta si {} ≥ k = {}.",
            fmt(m.n),
            match m.side {
                Side::Lower => "ZL = (X̄ − LSL)/σ",
                Side::Upper => "ZU = (USL − X̄)/σ",
                Side::Both => "ZL y ZU",
            },
            fmt(tidy(m.k))
        ),
        Plan::MMethod => format!("Se toma una muestra de {}, se estima la fracción defectuosa con Q = √(n/(n − 1))·(distancia al límite)/σ y se acepta si no pasa de M = {} % (equivale a k = {}).", fmt(m.n), fmt(tidy(m.m)), fmt(tidy(k_of(&m)))),
        Plan::SequentialVariable => "Se mide unidad por unidad, se acumula Y = Σ(X − c) y se compara con las rectas de aceptación y rechazo.".into(),
        _ => String::new(),
    };
    let mut mets = vec![metric_main("α real (%)", pct(alpha_r)), metric("β real (%)", pct(beta_r))];
    if p != Plan::SequentialVariable {
        mets.push(metric("Tamaño de muestra", m.n));
    } else {
        mets.push(metric("ASN con la media buena", tidy(eg.asn)));
    }
    if aoql.1 > 0.0 {
        mets.push(metric("AOQL (%)", pct(aoql.1)));
    }
    let mut sol = vec![metrics(mets), text(desc)];
    let mut rt = Table::new(Some("Riesgos"), &["Medida", "Valor"]).left(0);
    rt.row(cells![if by_p { format!("Pa en AQL = {} %", fmt(m.aql)) } else { format!("Pa con la media buena {}", fmt(tidy(mu_good))) }, tidy(eg.pa)]);
    rt.row(cells!["Riesgo del productor α real (%)", pct(alpha_r)]);
    for (i, e) in ebs.iter().enumerate() {
        rt.row(cells![if by_p { format!("Pa en RQL = {} % (β real)", fmt(m.rql)) } else { format!("Pa con la media mala {} (β real)", fmt(tidy(mu_bad[i]))) }, tidy(e.pa)]);
    }
    if p == Plan::Mean || by_p {
        let mut pt = Table::new(Some("Plan"), &["Parámetro", "Valor"]).left(0);
        pt.row(cells!["n", m.n]);
        if p == Plan::Mean {
            if let Some(v) = m.xla {
                pt.row(cells!["XLa", tidy(v)]);
            }
            if let Some(v) = m.xua {
                pt.row(cells!["XUa", tidy(v)]);
            }
        } else {
            pt.row(cells!["k", tidy(m.k)]);
            if p == Plan::MMethod {
                pt.row(cells!["M (%)", tidy(m.m)]);
            }
        }
        sol.push(pt.block());
    }
    sol.push(rt.block());
    if let Some(w) = o.what_if {
        let mu = if by_p { mu_for(&m, w / 100.0).unwrap_or(0.0) } else { w };
        let e = eval(&m, mu);
        let mut wt = Table::new(Some(&format!("Qué pasa si {} = {}", if by_p { "el % defectuoso" } else { "la media" }, fmt(w))), &["Medida", "Valor"]).left(0);
        wt.row(cells!["Probabilidad de aceptación", tidy(e.pa)]);
        if let Some(a) = e.aoq {
            wt.row(cells!["AOQ (%)", pct(a)]);
        }
        wt.row(cells!["ASN", tidy(e.asn)]);
        if let Some(a) = e.ati {
            wt.row(cells!["ATI", tidy(a)]);
        }
        if let Some(c) = e.cost {
            wt.row(cells!["Costo total por lote", tidy(c)]);
        }
        sol.push(wt.block());
    }
    // gráficas
    let mut ch = Chart::new(Some("Curva característica de operación (OC)"), xlab, "Probabilidad de aceptación").series(Series::new("OC", SeriesKind::Line, oc).tone(Tone::Accent)).hline(1.0 - al, "1 − α", Tone::Good).hline(be, "β", Tone::Bad);
    if by_p {
        ch = ch.vline(m.aql, "AQL", Tone::Good).vline(m.rql, "RQL", Tone::Bad);
    } else {
        ch = ch.vline(mu_good, "buena", Tone::Good);
        for b in &mu_bad {
            ch = ch.vline(*b, "mala", Tone::Bad);
        }
    }
    let mut charts = vec![ch.block()];
    if !aoq.is_empty() {
        charts.push(Chart::new(Some("Calidad media de salida (AOQ)"), xlab, "AOQ (%)").series(Series::new("AOQ", SeriesKind::Line, aoq).tone(Tone::Highlight)).block());
    }
    if p == Plan::SequentialVariable {
        charts.push(Chart::new(Some("Muestra media (ASN)"), xlab, "ASN").series(Series::new("ASN", SeriesKind::Line, asn).tone(Tone::Accent)).block());
    }
    if !ati.is_empty() {
        charts.push(Chart::new(Some("Inspección total media (ATI)"), xlab, "ATI").series(Series::new("ATI", SeriesKind::Line, ati).tone(Tone::Accent)).block());
    }
    if !cost.is_empty() {
        charts.push(Chart::new(Some("Costo total medio por lote"), xlab, "Costo").series(Series::new("Costo", SeriesKind::Line, cost).tone(Tone::Highlight)).block());
    }
    // proceso secuencial
    let mut seq_blocks = vec![];
    let mut seq_data = Value::Null;
    if p == Plan::SequentialVariable {
        let lines = seq_lines(&m);
        let obs: Vec<f64> = o.observations.split([',', ';', ' ', '\n']).map(str::trim).filter(|s| !s.is_empty()).filter_map(|s| s.parse::<f64>().ok()).collect();
        let nmax = obs.len().max(30).min(2000);
        let mut ch = Chart::new(Some("Rectas de aceptación y de rechazo"), "Unidades medidas (n)", "Y = Σ(X − c)");
        for (name, f, tone) in &lines.lines {
            ch = ch.series(Series::new(name.clone(), SeriesKind::Line, (1..=nmax).map(|n| [n as f64, f(n as f64)]).collect()).tone(*tone));
        }
        let mut lt = Table::new(Some("Parámetros"), &["Parámetro", "Valor"]).left(0);
        for (k2, v) in &lines.params {
            lt.row(cells![k2.clone(), tidy(*v)]);
        }
        if !obs.is_empty() {
            let mut pt = Table::new(Some("Proceso secuencial"), &["n", "X", "Y = Σ(X − c)", "Decisión"]).left(3);
            let mut y = 0.0;
            let mut path = vec![];
            let mut decision = "seguir midiendo".to_string();
            for (i, x) in obs.iter().enumerate() {
                y += x - lines.c;
                let n = (i + 1) as f64;
                let d = (lines.decide)(n, y);
                path.push([n, y]);
                pt.row_tone(cells![n, *x, tidy(y), d], if d == "seguir" { Tone::Normal } else if d == "aceptar" { Tone::Good } else { Tone::Bad });
                if d != "seguir" {
                    decision = format!("{d} el lote en la unidad {}", i + 1);
                    break;
                }
            }
            ch = ch.series(Series::new("Datos", SeriesKind::Line, path).tone(Tone::Accent));
            seq_blocks.push(text(format!("Con los datos medidos: {decision}.")));
            seq_blocks.push(pt.block());
            seq_data = json!({"decision": decision});
        }
        seq_blocks.insert(0, lt.block());
        seq_blocks.insert(0, ch.block());
    }
    let mut st = vec![];
    if o.steps {
        let dist = if m.sigma_known { "normal (σ conocida)" } else { "t de Student con n − 1 grados (σ de la muestra)" };
        let how = match p {
            Plan::Mean => format!("Pa(μ) = P(X̄ del lado de aceptación) con (X̄ − μ)√n/σ ~ {dist}. Con la media buena Pa = {} y con la mala {}.", fmt(tidy(eg.pa)), fmt(tidy(beta_r))),
            Plan::KMethod | Plan::MMethod => format!(
                "Si el proceso tiene una fracción p fuera de especificación, su distancia al límite es z_p = Φ⁻¹(1 − p) desviaciones. Pa = Φ((z_p − k)/√v) con v = 1/n{} (k = {}).",
                if m.sigma_known { "" } else { " + k²/(2n) (aproximación de Wallis)" },
                fmt(tidy(k_of(&m)))
            ),
            Plan::SequentialVariable => format!("Prueba secuencial de Wald para la media normal con α = {} % y β = {} %; las rectas salen de h1, h2 y la pendiente s, y la OC y el ASN de las fórmulas de Wald.", fmt(m.alpha), fmt(m.beta)),
            _ => String::new(),
        };
        st.push(steps(vec![step("Probabilidad de aceptación", how, vec![]), step("Riesgos", format!("α = 1 − Pa(buena) = {} %; β = Pa(mala) = {} %.", fmt(pct(alpha_r)), fmt(pct(beta_r))), vec![])]));
    }
    let headline = format!(
        "{}α real {} % y β real {} %{}.",
        if !search_blocks.is_empty() {
            match p {
                Plan::Mean => format!("Plan encontrado: n = {}{}{}. ", fmt(m.n), m.xla.map(|v| format!(", XLa = {}", fmt(tidy(v)))).unwrap_or_default(), m.xua.map(|v| format!(", XUa = {}", fmt(tidy(v)))).unwrap_or_default()),
                Plan::MMethod => format!("Plan encontrado: n = {}, M = {} %. ", fmt(m.n), fmt(tidy(m.m))),
                _ => format!("Plan encontrado: n = {}, k = {}. ", fmt(m.n), fmt(tidy(m.k))),
            }
        } else {
            String::new()
        },
        fmt(pct(alpha_r)),
        fmt(pct(beta_r)),
        if aoql.1 > 0.0 { format!("; AOQL {} %", fmt(pct(aoql.1))) } else { String::new() }
    );
    let mut r = Report::new("asa", &m.name, Status::Solved, headline).section("solucion", "Solución", sol);
    if !search_blocks.is_empty() {
        r = r.section("busqueda", "Búsqueda", search_blocks);
    }
    if !seq_blocks.is_empty() {
        r = r.section("secuencial", "Proceso secuencial", seq_blocks);
    }
    r = r.section("curvas", "Curvas", charts).section("tabla", "Tabla", vec![tt.block()]).section("pasos", "Paso a paso", st);
    Ok(r.data(json!({
        "alpha": pct(alpha_r), "beta": pct(beta_r), "n": m.n, "xla": m.xla.map(tidy), "xua": m.xua.map(tidy), "k": tidy(m.k), "m": tidy(m.m),
        "aoql": if aoql.1 > 0.0 { json!(pct(aoql.1)) } else { Value::Null }, "asn_good": tidy(eg.asn), "sequential": seq_data,
    })))
}

type LineFn = Box<dyn Fn(f64) -> f64>;

struct SeqLines {
    c: f64,
    lines: Vec<(String, LineFn, Tone)>,
    params: Vec<(String, f64)>,
    decide: Box<dyn Fn(f64, f64) -> &'static str>,
}

/// Rectas del muestreo secuencial para variables (ayuda de WinQSB).
fn seq_lines(m: &AsaModel) -> SeqLines {
    let (al, be) = (m.alpha / 100.0, m.beta / 100.0);
    let s2 = m.sigma * m.sigma;
    match m.side {
        Side::Lower | Side::Upper => {
            let (xu, xl) = if m.side == Side::Lower { (m.x1, m.x2l.unwrap()) } else { (m.x2u.unwrap(), m.x1) };
            let c = m.offset;
            let s = (xu + xl) / 2.0 - c;
            let h1 = s2 * ((1.0 - al) / be).ln() / (xu - xl);
            let h2 = s2 * ((1.0 - be) / al).ln() / (xu - xl);
            let params = vec![("Desplazamiento c".into(), c), ("Pendiente s".into(), s), ("h1".into(), h1), ("h2".into(), h2)];
            if m.side == Side::Lower {
                SeqLines {
                    c,
                    lines: vec![("Aceptación: Y ≥ h1 + s·n".into(), Box::new(move |n| h1 + s * n), Tone::Good), ("Rechazo: Y ≤ −h2 + s·n".into(), Box::new(move |n| -h2 + s * n), Tone::Bad)],
                    params,
                    decide: Box::new(move |n, y| if y >= h1 + s * n { "aceptar" } else if y <= -h2 + s * n { "rechazar" } else { "seguir" }),
                }
            } else {
                SeqLines {
                    c,
                    lines: vec![("Aceptación: Y ≤ −h1 + s·n".into(), Box::new(move |n| -h1 + s * n), Tone::Good), ("Rechazo: Y ≥ h2 + s·n".into(), Box::new(move |n| h2 + s * n), Tone::Bad)],
                    params,
                    decide: Box::new(move |n, y| if y >= h2 + s * n { "rechazar" } else if y <= -h1 + s * n { "aceptar" } else { "seguir" }),
                }
            }
        }
        Side::Both => {
            let (xl, xu) = (m.x2l.unwrap(), m.x2u.unwrap());
            let c = (xu + xl) / 2.0;
            let s = (xu - xl) / 4.0;
            let h1 = 2.0 * s2 * ((1.0 - al / 2.0) / be).ln() / (xu - xl);
            let h2 = 2.0 * s2 * (2.0 * (1.0 - be) / al).ln() / (xu - xl);
            SeqLines {
                c,
                lines: vec![
                    ("Aceptación superior: −h1 + s·n".into(), Box::new(move |n| -h1 + s * n), Tone::Good),
                    ("Aceptación inferior: h1 − s·n".into(), Box::new(move |n| h1 - s * n), Tone::Good),
                    ("Rechazo superior: h2 + s·n".into(), Box::new(move |n| h2 + s * n), Tone::Bad),
                    ("Rechazo inferior: −h2 − s·n".into(), Box::new(move |n| -h2 - s * n), Tone::Bad),
                ],
                params: vec![("Centro c".into(), c), ("Pendiente s".into(), s), ("h1".into(), h1), ("h2".into(), h2)],
                decide: Box::new(move |n, y| {
                    if y >= h2 + s * n || y <= -h2 - s * n {
                        "rechazar"
                    } else if y <= -h1 + s * n && y >= h1 - s * n {
                        "aceptar"
                    } else {
                        "seguir"
                    }
                }),
            }
        }
    }
}
