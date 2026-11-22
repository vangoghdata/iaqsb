//! QA · Teoría de colas (equivalente al módulo QA de WinQSB).
//!
//! - Modelo simple: llegadas de Poisson y servicio exponencial (M/M/s) con
//!   capacidad y población finitas o infinitas.
//! - Modelo general: distribuciones cualesquiera de tiempo entre llegadas y de
//!   servicio, llegadas en lotes, servicio presionado (μ(n) = (n/s)^a·s·μ si
//!   n > s) y llegadas desalentadas (λ(n) = (s/(n+1))^b·λ si n ≥ s), con las
//!   definiciones de la ayuda de WinQSB.
//! - Si las distribuciones son exponenciales la solución es exacta: cadena de
//!   Markov resuelta por balance de cortes (vale con lotes, presión,
//!   desaliento, capacidad y población finitas). M/G/1 por Pollaczek-Khinchine,
//!   G/G/s por la aproximación de WinQSB (Allen-Cunneen) y simulación cuando no
//!   hay fórmula.
//! - Análisis de costos, de capacidad (servidores × capacidad de la cola) y de
//!   sensibilidad para todos los parámetros.
//!
//! Formato de archivo comprobado con los ejemplos originales QUEUE1.QA y QUEUE2.QA.

pub mod sim;

use crate::its::DistIn;
use crate::pert::dist::{Dist, Law};
use iaqsb_core::linear::Bnd;
use iaqsb_core::winqsb::{num, parse_num, Reader, Writer};
use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum QaType {
    #[default]
    Simple,
    General,
}

fn m_inf() -> Bnd {
    Bnd::INF
}
fn hour() -> String {
    "hora".into()
}
fn one() -> u32 {
    1
}
fn expo() -> DistIn {
    DistIn { distribution: Dist::Exponential, p1: Some(0.0), p2: Some(1.0), p3: None, values: None }
}
fn batch_one() -> DistIn {
    DistIn::constant(1.0)
}
fn is_batch_one(d: &DistIn) -> bool {
    d.distribution == Dist::Constant && d.p1 == Some(1.0)
}

/// WinQSB considera infinita una población de más de 10 000 clientes.
pub const POP_INF: f64 = 10_000.0;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QaModel {
    #[serde(default)]
    pub name: String,
    pub problem_type: QaType,
    #[serde(default = "hour")]
    pub time_unit: String,
    /// Modelo simple: llegadas por unidad de tiempo (por cliente si la población es finita).
    #[serde(default)]
    pub arrival_rate: f64,
    /// Modelo simple: servicios por unidad de tiempo de cada servidor.
    #[serde(default)]
    pub service_rate: f64,
    #[serde(default = "one")]
    pub servers: u32,
    /// Lugares en la cola ("M" = ilimitada).
    #[serde(default = "m_inf")]
    pub queue_capacity: Bnd,
    /// Clientes posibles ("M" o más de 10 000 = infinitos).
    #[serde(default = "m_inf")]
    pub population: Bnd,
    /// Modelo general: tiempo entre llegadas y tiempo de servicio.
    #[serde(default = "expo")]
    pub interarrival: DistIn,
    #[serde(default = "expo")]
    pub service: DistIn,
    /// Modelo general: coeficiente de presión del servicio (a): con n > s clientes cada servidor trabaja (n/s)^a veces más rápido.
    #[serde(default)]
    pub pressure: f64,
    /// Modelo general: coeficiente de desaliento de las llegadas (b): con n ≥ s clientes llegan (s/(n+1))^b veces menos.
    #[serde(default)]
    pub discourage: f64,
    /// Modelo general: tamaño del lote de cada llegada (constante 1 por omisión).
    #[serde(default = "batch_one", skip_serializing_if = "is_batch_one")]
    pub batch: DistIn,
    #[serde(default)]
    pub busy_server_cost: f64,
    #[serde(default)]
    pub idle_server_cost: f64,
    /// Costo por cliente y unidad de tiempo en la cola.
    #[serde(default)]
    pub waiting_cost: f64,
    /// Costo por cliente y unidad de tiempo siendo atendido.
    #[serde(default)]
    pub service_cost: f64,
    /// Costo por cliente que no entra (cola llena o desalentado).
    #[serde(default)]
    pub balking_cost: f64,
    /// Costo por lugar de la cola y unidad de tiempo.
    #[serde(default)]
    pub capacity_cost: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum SensParam {
    #[default]
    None,
    Servers,
    ServiceRate,
    Pressure,
    ArrivalRate,
    Discourage,
    QueueCapacity,
    Population,
    BusyServerCost,
    IdleServerCost,
    WaitingCost,
    ServiceCost,
    BalkingCost,
    CapacityCost,
}

impl SensParam {
    const ALL: [SensParam; 14] = [
        SensParam::None,
        SensParam::Servers,
        SensParam::ServiceRate,
        SensParam::Pressure,
        SensParam::ArrivalRate,
        SensParam::Discourage,
        SensParam::QueueCapacity,
        SensParam::Population,
        SensParam::BusyServerCost,
        SensParam::IdleServerCost,
        SensParam::WaitingCost,
        SensParam::ServiceCost,
        SensParam::BalkingCost,
        SensParam::CapacityCost,
    ];
    fn id(self) -> &'static str {
        match self {
            SensParam::None => "none",
            SensParam::Servers => "servers",
            SensParam::ServiceRate => "service_rate",
            SensParam::Pressure => "pressure",
            SensParam::ArrivalRate => "arrival_rate",
            SensParam::Discourage => "discourage",
            SensParam::QueueCapacity => "queue_capacity",
            SensParam::Population => "population",
            SensParam::BusyServerCost => "busy_server_cost",
            SensParam::IdleServerCost => "idle_server_cost",
            SensParam::WaitingCost => "waiting_cost",
            SensParam::ServiceCost => "service_cost",
            SensParam::BalkingCost => "balking_cost",
            SensParam::CapacityCost => "capacity_cost",
        }
    }
    fn label(self) -> &'static str {
        match self {
            SensParam::None => "",
            SensParam::Servers => "Número de servidores",
            SensParam::ServiceRate => "Tasa de servicio (μ)",
            SensParam::Pressure => "Coeficiente de presión (a)",
            SensParam::ArrivalRate => "Tasa de llegada (λ)",
            SensParam::Discourage => "Coeficiente de desaliento (b)",
            SensParam::QueueCapacity => "Capacidad de la cola",
            SensParam::Population => "Población de clientes",
            SensParam::BusyServerCost => "Costo de servidor ocupado",
            SensParam::IdleServerCost => "Costo de servidor ocioso",
            SensParam::WaitingCost => "Costo de cliente esperando",
            SensParam::ServiceCost => "Costo de cliente atendido",
            SensParam::BalkingCost => "Costo por cliente rechazado",
            SensParam::CapacityCost => "Costo por lugar en la cola",
        }
    }
    fn integer(self) -> bool {
        matches!(self, SensParam::Servers | SensParam::QueueCapacity | SensParam::Population)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct QaOptions {
    pub steps: bool,
    /// Análisis de capacidad: número de servidores desde/hasta (0 = automático).
    pub servers_from: u32,
    pub servers_to: u32,
    /// Análisis de capacidad: también la capacidad de la cola desde/hasta.
    pub queue_analysis: bool,
    pub queue_from: u32,
    pub queue_to: u32,
    /// Análisis de sensibilidad.
    pub sensitivity: SensParam,
    pub sens_from: f64,
    pub sens_to: f64,
    pub sens_steps: u32,
    /// Simulación: tiempo simulado (0 = solo si no hay fórmula).
    pub simulation_time: f64,
    pub warmup: f64,
    pub seed: u64,
}

impl Default for QaOptions {
    fn default() -> Self {
        QaOptions {
            steps: true,
            servers_from: 0,
            servers_to: 0,
            queue_analysis: false,
            queue_from: 0,
            queue_to: 5,
            sensitivity: SensParam::None,
            sens_from: 0.0,
            sens_to: 0.0,
            sens_steps: 10,
            simulation_time: 0.0,
            warmup: 0.0,
            seed: 27437,
        }
    }
}

/// Medidas de desempeño de un sistema de colas.
#[derive(Debug, Clone, Default)]
pub struct Perf {
    /// Tasa de llegada base (por cliente si la población es finita).
    pub lam: f64,
    /// Clientes que quieren entrar por unidad de tiempo y los que entran.
    pub offered: f64,
    pub lam_eff: f64,
    pub mu: f64,
    pub l: f64,
    pub lq: f64,
    pub ls: f64,
    pub w: f64,
    pub wq: f64,
    pub ws: f64,
    pub p0: f64,
    /// Probabilidad de que el sistema esté ocupado: Σ P(n) con n ≥ s (definición de WinQSB).
    pub pw: f64,
    pub p_balk: f64,
    pub util: f64,
    pub pn: Vec<f64>,
    pub method: String,
    pub exact: bool,
}

impl Perf {
    /// Clientes rechazados (o desalentados) por unidad de tiempo: B = λ − λₑ.
    pub fn balked(&self) -> f64 {
        (self.offered - self.lam_eff).max(0.0)
    }
}

fn ln_fact(n: usize) -> f64 {
    statrs::function::gamma::ln_gamma(n as f64 + 1.0)
}

/// M/M/s sin límites: fórmulas cerradas.
pub fn mms(lam: f64, mu: f64, s: usize) -> Result<Perf, String> {
    if lam <= 0.0 || mu <= 0.0 {
        return Err("las tasas de llegada y de servicio deben ser positivas".into());
    }
    if s == 0 {
        return Err("hace falta al menos un servidor".into());
    }
    let sf = s as f64;
    let a = lam / mu;
    let rho = a / sf;
    if rho >= 1.0 {
        return Err(format!("el sistema es inestable: ρ = λ/(s·μ) = {} ≥ 1 y la cola crece sin límite", fmt(tidy(rho))));
    }
    let terms: Vec<f64> = (0..s).map(|n| n as f64 * a.ln() - ln_fact(n)).collect();
    let last = sf * a.ln() - ln_fact(s) - (1.0 - rho).ln();
    let mx = terms.iter().cloned().fold(last, f64::max);
    let sum: f64 = terms.iter().map(|t| (t - mx).exp()).sum::<f64>() + (last - mx).exp();
    let lp0 = -(mx + sum.ln());
    let pn = |n: usize| -> f64 {
        if n < s {
            (lp0 + n as f64 * a.ln() - ln_fact(n)).exp()
        } else {
            (lp0 + sf * a.ln() - ln_fact(s) + (n - s) as f64 * rho.ln()).exp()
        }
    };
    let mut p = Perf { lam, offered: lam, lam_eff: lam, mu, method: format!("M/M/{s}"), exact: true, ..Default::default() };
    let ps = pn(s);
    p.lq = ps * rho / (1.0 - rho).powi(2);
    p.pw = ps / (1.0 - rho);
    p.ls = a;
    p.l = p.lq + a;
    p.p0 = pn(0);
    let mut acc = 0.0;
    let mut n = 0;
    while (acc < 0.9999 || n <= s) && n < 2000 {
        let v = pn(n);
        p.pn.push(v);
        acc += v;
        n += 1;
    }
    p.wq = p.lq / lam;
    p.w = p.l / lam;
    p.ws = 1.0 / mu;
    p.util = rho;
    Ok(p)
}

/// Datos de una cadena de Markov de colas.
pub struct Chain<'a> {
    pub lam: f64,
    pub mu: f64,
    pub s: usize,
    pub qcap: Option<usize>,
    pub pop: Option<usize>,
    pub pressure: f64,
    pub discourage: f64,
    /// P(lote = k), k = 0, 1, …
    pub batch: &'a [f64],
}

/// Cadena de Markov exacta por balance de cortes: el flujo que sube del estado n al n + 1
/// (llegadas desde i ≤ n con lote mayor que n − i) iguala al que baja, P(n+1)·μ(n+1).
pub fn chain(c: &Chain) -> Result<Perf, String> {
    let Chain { lam, mu, s, qcap, pop, pressure, discourage, batch } = *c;
    if lam <= 0.0 || mu <= 0.0 {
        return Err("las tasas de llegada y de servicio deben ser positivas".into());
    }
    if s == 0 {
        return Err("hace falta al menos un servidor".into());
    }
    let kmax = batch.len().saturating_sub(1);
    let ex: f64 = batch.iter().enumerate().map(|(k, p)| k as f64 * p).sum();
    if !(ex > 0.0) {
        return Err("el tamaño medio del lote debe ser positivo".into());
    }
    // G[j] = P(lote > j)
    let g: Vec<f64> = (0..=kmax).map(|j| batch.iter().skip(j + 1).sum::<f64>()).collect();
    let gj = |j: usize| g.get(j).copied().unwrap_or(0.0);
    let kcap = match (qcap.map(|q| q + s), pop) {
        (Some(k), Some(n)) => Some(k.min(n)),
        (k, n) => k.or(n),
    };
    let sf = s as f64;
    let base = |n: usize| match pop {
        Some(np) => np.saturating_sub(n) as f64 * lam,
        None => lam,
    };
    let lam_n = |n: usize| base(n) * if discourage != 0.0 && n >= s { (sf / (n as f64 + 1.0)).powf(discourage) } else { 1.0 };
    let mu_n = |n: usize| if n <= s { n as f64 * mu } else { (n as f64 / sf).powf(pressure) * sf * mu };
    if kcap.is_none() && pressure == 0.0 && discourage == 0.0 && lam * ex >= sf * mu {
        return Err(format!("el sistema es inestable: ρ = λ·E[lote]/(s·μ) = {} ≥ 1 y la cola crece sin límite", fmt(tidy(lam * ex / (sf * mu)))));
    }
    let limit = kcap.unwrap_or(400_000);
    let mut p = vec![1.0f64];
    let mut total = 1.0f64;
    let mut converged = kcap.is_some();
    for n in 0..limit {
        let lo = (n + 1).saturating_sub(kmax.max(1));
        let flow: f64 = (lo..=n).map(|i| p[i] * lam_n(i) * gj(n - i)).sum();
        let next = flow / mu_n(n + 1);
        p.push(next);
        total += next;
        if total > 1e250 {
            for v in p.iter_mut() {
                *v /= 1e250;
            }
            total /= 1e250;
        }
        if kcap.is_none() && n > s + kmax + 5 && next <= total * 1e-16 && next <= p[n] {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err("el sistema es inestable: la cola crece sin límite".into());
    }
    let pn: Vec<f64> = p.iter().map(|v| v / total).collect();
    let km = pn.len() - 1;
    // clientes de un lote que caben con n en el sistema
    let e_min = |n: usize| -> f64 {
        match kcap {
            Some(k) => (0..k.saturating_sub(n)).map(gj).sum(),
            None => ex,
        }
    };
    let mut r = Perf { lam, mu, exact: true, ..Default::default() };
    r.lam_eff = (0..=km).map(|n| pn[n] * lam_n(n) * e_min(n)).sum();
    r.offered = (0..=km).map(|n| pn[n] * base(n) * ex).sum();
    r.l = (0..=km).map(|n| n as f64 * pn[n]).sum();
    r.lq = (0..=km).map(|n| n.saturating_sub(s) as f64 * pn[n]).sum();
    r.ls = r.l - r.lq;
    r.pw = (s.min(km + 1)..=km).map(|n| pn[n]).sum();
    r.p0 = pn[0];
    r.util = r.ls / sf;
    r.p_balk = if r.offered > 0.0 { ((r.offered - r.lam_eff) / r.offered).max(0.0) } else { 0.0 };
    if r.lam_eff > 0.0 {
        r.w = r.l / r.lam_eff;
        r.wq = r.lq / r.lam_eff;
    }
    r.ws = r.w - r.wq;
    // recorta la cola despreciable para mostrarla
    let mut acc = 0.0;
    let mut cut = pn.len();
    for (n, v) in pn.iter().enumerate() {
        acc += v;
        if acc > 1.0 - 1e-6 && n > s {
            cut = n + 1;
            break;
        }
    }
    r.pn = pn[..cut.min(2001)].to_vec();
    Ok(r)
}

/// G/G/s sin límites: exacto para M/G/1 y la aproximación de WinQSB (Allen-Cunneen) en otro caso.
pub fn general(ia: &Law, sv: &Law, s: usize) -> Result<Perf, String> {
    let ma = ia.mean();
    let ms = sv.mean();
    if !(ma > 0.0) || !(ms > 0.0) {
        return Err("los tiempos medios entre llegadas y de servicio deben ser positivos".into());
    }
    let lam = 1.0 / ma;
    let mu = 1.0 / ms;
    let ca2 = ia.variance() / (ma * ma);
    let cs2 = sv.variance() / (ms * ms);
    let base = mms(lam, mu, s)?;
    let poisson = ia.dist == Dist::Exponential && ia.p[0] == 0.0;
    let mut p = base.clone();
    if poisson && s == 1 {
        let rho = lam * ms;
        p.lq = (lam * lam * sv.variance() + rho * rho) / (2.0 * (1.0 - rho));
        p.exact = true;
    } else {
        p.lq = base.lq * (ca2 + cs2) / 2.0;
        p.exact = poisson && (cs2 - 1.0).abs() < 1e-12;
    }
    p.wq = p.lq / lam;
    p.w = p.wq + ms;
    p.l = lam * p.w;
    p.ls = lam * ms;
    p.ws = ms;
    if !p.exact {
        p.pn.clear();
    }
    Ok(p)
}

#[derive(Clone, Copy, Default)]
pub struct Costs {
    pub busy: f64,
    pub idle: f64,
    pub wait: f64,
    pub serv: f64,
    pub balk: f64,
    pub cap: f64,
}

impl Costs {
    fn any(&self) -> bool {
        [self.busy, self.idle, self.wait, self.serv, self.balk, self.cap].iter().any(|c| *c != 0.0)
    }
}

/// Costos por unidad de tiempo (fórmulas de la ayuda de WinQSB): servidores ocupados Cs·(L − Lq),
/// ociosos Ci·(s − L + Lq), espera Cw·Wq·λₑ, atención Cu·(W − Wq)·λₑ, rechazo Cb·B y cola Cq·Q.
pub fn cost_of(p: &Perf, s: usize, qcap: Option<usize>, c: &Costs) -> [f64; 7] {
    let busy = c.busy * p.ls;
    let idle = c.idle * (s as f64 - p.ls).max(0.0);
    let wait = c.wait * p.wq * p.lam_eff;
    let serv = c.serv * (p.w - p.wq) * p.lam_eff;
    let balk = c.balk * p.balked();
    let cap = c.cap * qcap.map(|q| q as f64).unwrap_or(0.0);
    [busy, idle, wait, serv, balk, cap, busy + idle + wait + serv + balk + cap]
}

fn as_count(b: Bnd, what: &str) -> Result<Option<usize>, ModError> {
    if !b.0.is_finite() {
        return Ok(None);
    }
    if b.0 < 0.0 || b.0.fract() != 0.0 {
        return invalid(format!("{what} debe ser un entero no negativo o M"));
    }
    Ok(Some(b.0 as usize))
}

/// P(lote = k) de una distribución discreta.
fn batch_pmf(l: &Law) -> Result<Vec<f64>, ModError> {
    let sup = crate::stat::support(l).ok_or_else(|| ModError::Invalid("el tamaño del lote debe tener una distribución discreta (constante, discreta, Poisson, binomial, geométrica…)".into()))?;
    let mut v = vec![];
    for (x, p) in sup {
        if x < 0.0 || (x - x.round()).abs() > 1e-9 {
            return invalid("los tamaños de lote deben ser enteros no negativos");
        }
        let k = x.round() as usize;
        if k > 100_000 {
            return invalid("lotes demasiado grandes");
        }
        if v.len() <= k {
            v.resize(k + 1, 0.0);
        }
        v[k] += p;
    }
    let t: f64 = v.iter().sum();
    if t > 0.0 {
        for x in v.iter_mut() {
            *x /= t;
        }
    }
    Ok(v)
}

pub struct Qa;

impl TypedModule for Qa {
    type Model = QaModel;
    type Options = QaOptions;

    fn file_tag(&self) -> &'static str {
        "QA"
    }

    fn spec(&self) -> ModuleSpec {
        let unit = Field::text("time_unit", "Unidad de tiempo").default("hora");
        let common = vec![
            Field::integer("servers", "Número de servidores", 1),
            Field::bound("queue_capacity", "Capacidad de la cola", Value::from("M")).help("M = ilimitada"),
            Field::bound("population", "Población de clientes", Value::from("M")).help("M = infinita (más de 10 000)"),
        ];
        let costs = vec![
            Field::number("busy_server_cost", "Costo de un servidor ocupado (por tiempo)"),
            Field::number("idle_server_cost", "Costo de un servidor ocioso (por tiempo)"),
            Field::number("waiting_cost", "Costo de un cliente esperando (por tiempo)"),
            Field::number("service_cost", "Costo de un cliente atendido (por tiempo)"),
            Field::number("balking_cost", "Costo por cliente rechazado"),
            Field::number("capacity_cost", "Costo por lugar en la cola (por tiempo)"),
        ];
        let mut simple = vec![unit.clone(), Field::number("service_rate", "Tasa de servicio por servidor (μ)"), Field::number("arrival_rate", "Tasa de llegada (λ)").help("Por cliente si la población es finita")];
        simple.extend(common.clone());
        let mut gen = vec![unit];
        gen.extend(common);
        let sens: Vec<(&str, &str)> = SensParam::ALL.iter().map(|p| (p.id(), if *p == SensParam::None { "Ninguna" } else { p.label() })).collect();
        let sens_ids: Vec<&str> = sens.iter().skip(1).map(|x| x.0).collect();
        let real_ids: Vec<&str> = sens.iter().skip(1).filter(|x| !matches!(x.0, "servers" | "queue_capacity" | "population")).map(|x| x.0).collect();
        ModuleSpec {
            id: "qa".into(),
            winqsb: "QA".into(),
            name: "Teoría de colas".into(),
            description: "M/M/s con capacidad y población finitas, lotes, presión y desaliento, M/G/1, G/G/s, costos, capacidad y sensibilidad".into(),
            file_ext: "QAA".into(),
            problem_types: vec![
                ProblemType {
                    id: "simple".into(),
                    label: "Modelo simple (M/M/s)".into(),
                    description: "Llegadas de Poisson y tiempos de servicio exponenciales".into(),
                    dims: vec![],
                    layout: vec![Widget::params(Some("Sistema"), simple), Widget::params(Some("Costos (opcionales)"), costs.clone())],
                },
                ProblemType {
                    id: "general".into(),
                    label: "Modelo general".into(),
                    description: "Cualquier distribución de llegadas y servicio, llegadas en lotes, servicio presionado y llegadas desalentadas".into(),
                    dims: vec![],
                    layout: vec![
                        Widget::params(Some("Sistema"), gen),
                        crate::its::dist_widget("Tiempo de servicio", "service"),
                        crate::its::dist_widget("Tiempo entre llegadas", "interarrival"),
                        crate::its::dist_widget("Tamaño del lote de llegada", "batch"),
                        Widget::params(
                            Some("Presión y desaliento"),
                            vec![
                                Field::number("pressure", "Coeficiente de presión del servicio (a)").help("Con n > s clientes cada servidor atiende (n/s)^a veces más rápido; 0 = sin presión"),
                                Field::number("discourage", "Coeficiente de desaliento de las llegadas (b)").help("Con n ≥ s clientes llegan (s/(n+1))^b veces menos; 0 = sin desaliento"),
                            ],
                        ),
                        Widget::params(Some("Costos (opcionales)"), costs),
                    ],
                },
            ],
            options: vec![
                Field::boolean("steps", "Mostrar paso a paso", true),
                Field::integer("servers_from", "Capacidad: servidores desde", 0).help("0 = automático"),
                Field::integer("servers_to", "Capacidad: servidores hasta", 0),
                Field::boolean("queue_analysis", "Capacidad: comparar también la capacidad de la cola", false),
                Field::integer("queue_from", "Capacidad de la cola desde", 0).show_if("queue_analysis", &["true"]),
                Field::integer("queue_to", "Capacidad de la cola hasta", 5).show_if("queue_analysis", &["true"]),
                Field::choice("sensitivity", "Sensibilidad respecto a", &sens).default("none"),
                Field::number("sens_from", "Sensibilidad desde").show_if("sensitivity", &sens_ids),
                Field::number("sens_to", "Sensibilidad hasta").show_if("sensitivity", &sens_ids),
                Field::integer("sens_steps", "Pasos", 10).show_if("sensitivity", &real_ids),
                Field::number("simulation_time", "Tiempo simulado").help("0 = solo si no hay fórmula exacta"),
                Field::number("warmup", "Inicio de la recolección de datos"),
                Field::integer("seed", "Semilla", 27437),
            ],
        }
    }

    fn template(&self, pt: &str, _dims: &Value) -> Result<QaModel, ModError> {
        let t: QaType = serde_json::from_value(Value::from(pt)).map_err(|_| ModError::Invalid(format!("tipo de problema desconocido: {pt}")))?;
        Ok(serde_json::from_value(json!({"name": "Sistema de colas", "problem_type": t, "arrival_rate": 2, "service_rate": 3})).unwrap())
    }

    fn solve(&self, m: &QaModel, o: &QaOptions) -> Result<Report, ModError> {
        solve(m, o)
    }

    fn read(&self, bytes: &[u8]) -> Result<QaModel, ModError> {
        read_qa(bytes)
    }

    fn write(&self, m: &QaModel) -> Result<Vec<u8>, ModError> {
        Ok(write_qa(m))
    }

    fn examples(&self) -> Vec<(&'static str, QaModel)> {
        let ex = |s: &str| serde_json::from_str::<QaModel>(s).unwrap();
        vec![
            ("sala_de_emergencias", ex(include_str!("ejemplos/emergencias.json"))),
            ("reciclaje_md1", ex(include_str!("ejemplos/md1.json"))),
            ("reparacion_maquinas", ex(include_str!("ejemplos/maquinas.json"))),
            ("barberia_capacidad", ex(include_str!("ejemplos/barberia.json"))),
            ("llegadas_en_lotes", ex(include_str!("ejemplos/lotes.json"))),
            ("presion_y_desaliento", ex(include_str!("ejemplos/presion.json"))),
        ]
    }
}

/// Sistema ya validado (lo que cambian los análisis de capacidad y sensibilidad).
#[derive(Clone)]
struct Sys {
    s: usize,
    qcap: Option<usize>,
    pop: Option<usize>,
    lam: f64,
    mu: f64,
    lam0: f64,
    mu0: f64,
    ia: Law,
    sv: Law,
    batch: Law,
    pmf: Vec<f64>,
    a: f64,
    b: f64,
    markov: bool,
    costs: Costs,
}

impl Sys {
    fn batch_one(&self) -> bool {
        self.pmf.len() == 2 && (self.pmf[1] - 1.0).abs() < 1e-12
    }
    fn plain(&self) -> bool {
        self.batch_one() && self.a == 0.0 && self.b == 0.0
    }
    /// Distribución escalada si la sensibilidad cambia la tasa (conserva el coeficiente de variación).
    fn scaled(l: &Law, f: f64) -> Law {
        if (f - 1.0).abs() < 1e-15 {
            return l.clone();
        }
        if l.dist == Dist::Exponential {
            return Law { dist: Dist::Exponential, p: [l.p[0] * f, l.p[1] * f, 0.0], values: vec![] };
        }
        Law { dist: Dist::Normal, p: [l.mean() * f, l.variance().sqrt() * f, 0.0], values: vec![] }
    }
    fn ia_law(&self) -> Law {
        Sys::scaled(&self.ia, self.lam0 / self.lam)
    }
    fn sv_law(&self) -> Law {
        Sys::scaled(&self.sv, self.mu0 / self.mu)
    }
    /// Notación de Kendall con los añadidos de WinQSB.
    fn kendall(&self) -> String {
        let letter = |l: &Law| match l.dist {
            Dist::Exponential if l.p[0] == 0.0 => "M".to_string(),
            Dist::Constant => "D".to_string(),
            Dist::Erlang if l.p[0] == 0.0 => format!("E{}", l.p[2]),
            _ => "G".to_string(),
        };
        let mut k = format!("{}{}/{}/{}", letter(&self.ia), if self.batch_one() { "" } else { "ˣ" }, letter(&self.sv), self.s);
        if let Some(q) = self.qcap {
            k += &format!("/{}", q + self.s);
        } else if self.pop.is_some() {
            k += "/∞";
        }
        if let Some(n) = self.pop {
            k += &format!("/{n}");
        }
        if self.a != 0.0 {
            k += " con servicio presionado";
        }
        if self.b != 0.0 {
            k += if self.a != 0.0 { " y llegadas desalentadas" } else { " con llegadas desalentadas" };
        }
        k
    }
}

fn setup(m: &QaModel) -> Result<Sys, ModError> {
    let s = m.servers as usize;
    if s == 0 {
        return invalid("hace falta al menos un servidor");
    }
    let qcap = as_count(m.queue_capacity, "la capacidad de la cola")?;
    let pop = if m.population.0 > POP_INF { None } else { as_count(m.population, "la población")? };
    if pop == Some(0) {
        return invalid("la población debe tener al menos un cliente");
    }
    let general = m.problem_type == QaType::General;
    let (ia, sv, markov) = if general {
        let ia = m.interarrival.law("tiempo entre llegadas")?;
        let sv = m.service.law("tiempo de servicio")?;
        let ex = |l: &Law| l.dist == Dist::Exponential && l.p[0] == 0.0;
        let mk = ex(&ia) && ex(&sv);
        (ia, sv, mk)
    } else {
        if m.arrival_rate <= 0.0 || m.service_rate <= 0.0 {
            return invalid("las tasas de llegada y de servicio deben ser positivas");
        }
        let e = |r: f64| Law { dist: Dist::Exponential, p: [0.0, 1.0 / r, 0.0], values: vec![] };
        (e(m.arrival_rate), e(m.service_rate), true)
    };
    let batch = if general { m.batch.law("tamaño del lote")? } else { Law { dist: Dist::Constant, p: [1.0, 0.0, 0.0], values: vec![] } };
    let pmf = batch_pmf(&batch)?;
    if !(pmf.iter().enumerate().map(|(k, p)| k as f64 * p).sum::<f64>() > 0.0) {
        return invalid("el tamaño medio del lote debe ser positivo");
    }
    let (a, b) = if general { (m.pressure, m.discourage) } else { (0.0, 0.0) };
    if !a.is_finite() || !b.is_finite() {
        return invalid("los coeficientes de presión y desaliento deben ser números");
    }
    if !(ia.mean() > 0.0) || !(sv.mean() > 0.0) {
        return invalid("los tiempos medios entre llegadas y de servicio deben ser positivos");
    }
    let lam = 1.0 / ia.mean();
    let mu = 1.0 / sv.mean();
    let costs = Costs { busy: m.busy_server_cost, idle: m.idle_server_cost, wait: m.waiting_cost, serv: m.service_cost, balk: m.balking_cost, cap: m.capacity_cost };
    Ok(Sys { s, qcap, pop, lam, mu, lam0: lam, mu0: mu, ia, sv, batch, pmf, a, b, markov, costs })
}

fn analyze(st: &Sys) -> Result<Perf, String> {
    let mut p = if st.markov {
        if st.plain() && st.qcap.is_none() && st.pop.is_none() {
            mms(st.lam, st.mu, st.s)?
        } else {
            chain(&Chain { lam: st.lam, mu: st.mu, s: st.s, qcap: st.qcap, pop: st.pop, pressure: st.a, discourage: st.b, batch: &st.pmf })?
        }
    } else if st.plain() && st.qcap.is_none() && st.pop.is_none() {
        general(&st.ia_law(), &st.sv_law(), st.s)?
    } else {
        return Err("con distribuciones no exponenciales y capacidad o población finitas, lotes, presión o desaliento no hay fórmula: se usa la simulación".into());
    };
    p.method = st.kendall();
    p.lam = st.lam;
    p.mu = st.mu;
    Ok(p)
}

fn perf_table(p: &Perf, s: usize, u: &str, costs: Option<[f64; 7]>) -> Table {
    let mut t = Table::new(Some("Desempeño del sistema"), &["Medida", "Valor"]).left(0);
    let mut r = |l: String, v: f64, tone: Tone| {
        t.row_tone(cells![l, tidy(v)], tone);
    };
    r(format!("Tasa de llegada λ (por {u})"), p.lam, Tone::Normal);
    if (p.offered - p.lam).abs() > 1e-9 * p.lam.max(1.0) {
        r(format!("Clientes que llegan (por {u})"), p.offered, Tone::Normal);
    }
    if (p.lam_eff - p.offered).abs() > 1e-12 && p.lam_eff > 0.0 {
        r(format!("Tasa efectiva de llegada λₑ (por {u})"), p.lam_eff, Tone::Normal);
    }
    r(format!("Tasa de servicio por servidor μ (por {u})"), p.mu, Tone::Normal);
    r("Número de servidores".into(), s as f64, Tone::Normal);
    r("Utilización de los servidores (%)".into(), 100.0 * p.util, Tone::Normal);
    r("Clientes en el sistema (L)".into(), p.l, Tone::Highlight);
    r("Clientes en la cola (Lq)".into(), p.lq, Tone::Highlight);
    if p.pw > 1e-12 {
        r("Clientes en la cola con el sistema ocupado (Lb = Lq/Pw)".into(), p.lq / p.pw, Tone::Normal);
    }
    r("Clientes siendo atendidos".into(), p.ls, Tone::Normal);
    r(format!("Tiempo en el sistema (W, {u})"), p.w, Tone::Highlight);
    r(format!("Tiempo en la cola (Wq, {u})"), p.wq, Tone::Highlight);
    if p.pw > 1e-12 {
        r(format!("Tiempo en la cola con el sistema ocupado (Wb = Wq/Pw, {u})"), p.wq / p.pw, Tone::Normal);
    }
    r(format!("Tiempo de servicio ({u})"), p.ws, Tone::Normal);
    if !p.pn.is_empty() {
        r("Probabilidad de que todos los servidores estén ociosos (P0, %)".into(), 100.0 * p.p0, Tone::Normal);
    }
    r("Probabilidad de que el sistema esté ocupado (Pw, %)".into(), 100.0 * p.pw, Tone::Normal);
    if p.balked() > 1e-12 {
        r(format!("Clientes rechazados (B, por {u})"), p.balked(), Tone::Normal);
        r("Probabilidad de rechazo (%)".into(), 100.0 * p.p_balk, Tone::Normal);
    }
    if let Some(c) = costs {
        let labels = ["Costo de servidores ocupados", "Costo de servidores ociosos", "Costo de clientes esperando", "Costo de clientes atendidos", "Costo de clientes rechazados", "Costo de la capacidad de la cola"];
        for (i, l) in labels.iter().enumerate() {
            if c[i] != 0.0 {
                r(format!("{l} (por {u})"), c[i], Tone::Normal);
            }
        }
        r(format!("Costo total (por {u})"), c[6], Tone::Total);
    }
    t
}

fn simulate(st: &Sys, horizon: f64, warmup: f64, seed: u64) -> sim::StationOut {
    let ia = st.ia_law();
    let sv = st.sv_law();
    sim::simulate_station(&sim::SimCfg {
        ia: &ia,
        sv: &sv,
        batch: if st.batch_one() { None } else { Some(&st.batch) },
        s: st.s,
        qcap: st.qcap,
        pop: st.pop,
        pressure: st.a,
        discourage: st.b,
        horizon,
        warmup,
        seed,
    })
}

pub fn solve(m: &QaModel, o: &QaOptions) -> Result<Report, ModError> {
    let st = setup(m)?;
    let u = &m.time_unit;
    let has_cost = st.costs.any();
    let base = analyze(&st);
    let mut sol = vec![];
    let mut data = json!({});
    let mut st_blocks = vec![];
    let mut sim_blocks = vec![];
    let mut steps_v: Option<Vec<Step>> = None;
    let need_sim = base.is_err() || o.simulation_time > 0.0;
    let headline;
    let status;
    match &base {
        Ok(p) => {
            let c = has_cost.then(|| cost_of(p, st.s, st.qcap, &st.costs));
            let mut mets = vec![metric_main("L (en el sistema)", tidy(p.l)), metric("Lq (en la cola)", tidy(p.lq)), metric(format!("W ({u})"), tidy(p.w)), metric(format!("Wq ({u})"), tidy(p.wq)), metric("Utilización (%)", tidy(100.0 * p.util))];
            if let Some(c) = c {
                mets.push(metric(format!("Costo total por {u}"), tidy(c[6])));
            }
            sol.push(metrics(mets));
            let how = if p.exact {
                if st.markov {
                    if st.plain() && st.qcap.is_none() && st.pop.is_none() {
                        "fórmulas cerradas de M/M/s"
                    } else {
                        "cadena de Markov exacta (balance de cortes)"
                    }
                } else {
                    "fórmula de Pollaczek-Khinchine"
                }
            } else {
                "aproximación de G/G/s (resultado aproximado; compáralo con la simulación)"
            };
            sol.push(text(format!("Sistema {}: {how}.", p.method)));
            sol.push(perf_table(p, st.s, u, c).block());
            data = json!({
                "model": p.method, "exact": p.exact,
                "l": tidy(p.l), "lq": tidy(p.lq), "w": tidy(p.w), "wq": tidy(p.wq), "p0": tidy(p.p0), "pw": tidy(p.pw),
                "lb": if p.pw > 0.0 { json!(tidy(p.lq / p.pw)) } else { Value::Null },
                "wb": if p.pw > 0.0 { json!(tidy(p.wq / p.pw)) } else { Value::Null },
                "utilization": tidy(p.util), "effective_arrival_rate": tidy(p.lam_eff), "balked_per_time": tidy(p.balked()), "balk_probability": tidy(p.p_balk),
                "total_cost": c.map(|c| tidy(c[6])),
            });
            headline = format!("L = {}, Lq = {}, W = {} y Wq = {} {u}{}.", fmt(tidy(p.l)), fmt(tidy(p.lq)), fmt(tidy(p.w)), fmt(tidy(p.wq)), c.map(|c| format!("; costo total {} por {u}", fmt(tidy(c[6])))).unwrap_or_default());
            status = Status::Solved;
            if !p.pn.is_empty() {
                let mut pt = Table::new(Some("Probabilidad de n clientes en el sistema"), &["n", "P(n) (%)", "Acumulada (%)"]);
                let mut acc = 0.0;
                let show = p.pn.len().min(60);
                for (n, v) in p.pn.iter().take(show).enumerate() {
                    acc += v;
                    pt.row(cells![n, tidy(100.0 * v), tidy(100.0 * acc.min(1.0))]);
                }
                let ch = Chart::new(Some("Distribución del número de clientes"), "n", "P(n)").series(Series::new("P(n)", SeriesKind::Bar, p.pn.iter().take(show).enumerate().map(|(n, v)| [n as f64, *v]).collect()).tone(Tone::Accent));
                st_blocks.push(ch.block());
                st_blocks.push(pt.block());
            }
            if o.steps {
                steps_v = Some(explain(&st, p, u, c));
            }
        }
        Err(msg) => {
            sol.push(text_tone(format!("Sistema {}: {msg}.", st.kendall()), Tone::Accent));
            headline = "Sin fórmula para este sistema: resultados por simulación.".to_string();
            status = Status::Feasible;
        }
    }
    // capacidad
    let mut cap_blocks = vec![];
    if base.is_ok() {
        let (a, b) = if o.servers_from > 0 && o.servers_to >= o.servers_from {
            (o.servers_from as usize, o.servers_to as usize)
        } else {
            let unbounded = st.qcap.is_none() && st.pop.is_none() && st.a == 0.0 && st.b == 0.0;
            let ex: f64 = st.pmf.iter().enumerate().map(|(k, p)| k as f64 * p).sum();
            let min_stable = if unbounded { (st.lam * ex / st.mu).floor() as usize + 1 } else { 1 };
            (min_stable.max(st.s.saturating_sub(2)).max(1), st.s.max(min_stable) + 4)
        };
        let b = b.min(a + 200);
        let mut t = Table::new(Some("Análisis de capacidad (número de servidores)"), &["Servidores", "Utilización (%)", "L", "Lq", &format!("W ({u})"), &format!("Wq ({u})"), &format!("Costo total por {u}")]);
        let mut rows = vec![];
        for s in a..=b {
            let mut x = st.clone();
            x.s = s;
            if let Ok(p) = analyze(&x) {
                let c = cost_of(&p, s, st.qcap, &st.costs);
                rows.push((s, p, c[6]));
            }
        }
        let best = rows.iter().filter(|_| has_cost).min_by(|x, y| x.2.partial_cmp(&y.2).unwrap()).map(|x| x.0);
        for (s, p, c) in &rows {
            t.row_tone(cells![*s, tidy(100.0 * p.util), tidy(p.l), tidy(p.lq), tidy(p.w), tidy(p.wq), if has_cost { Cell::Num(tidy(*c)) } else { Cell::Empty }], if Some(*s) == best { Tone::Highlight } else if *s == st.s { Tone::Accent } else { Tone::Normal });
        }
        if rows.len() > 1 {
            let mut ch = Chart::new(Some("Según el número de servidores"), "Servidores", if has_cost { "Costo total" } else { "Wq" });
            ch = ch.series(Series::new(if has_cost { "Costo total" } else { "Wq" }, SeriesKind::Line, rows.iter().map(|(s, p, c)| [*s as f64, if has_cost { *c } else { p.wq }]).collect()).tone(Tone::Highlight));
            cap_blocks.push(t.block());
            cap_blocks.push(ch.block());
            if let Some(b) = best {
                cap_blocks.insert(0, text(format!("Con los costos indicados conviene tener {b} servidores.")));
                data["best_servers"] = json!(b);
            }
        }
        // servidores × capacidad de la cola
        if o.queue_analysis {
            let (qa, qb) = (o.queue_from as usize, (o.queue_to as usize).max(o.queue_from as usize).min(o.queue_from as usize + 30));
            let mut cols = vec!["Servidores \\ Capacidad de la cola".to_string()];
            cols.extend((qa..=qb).map(|q| q.to_string()));
            let mut t2 = Table::new(Some(&format!("Costo total por {u} según servidores y capacidad de la cola")), &cols);
            let mut best2: Option<(usize, usize, f64)> = None;
            let mut grid = vec![];
            for s in a..=b {
                let mut row = vec![Cell::from(s.to_string())];
                for q in qa..=qb {
                    let mut x = st.clone();
                    x.s = s;
                    x.qcap = Some(q);
                    match analyze(&x) {
                        Ok(p) => {
                            let v = if has_cost { cost_of(&p, s, Some(q), &st.costs)[6] } else { p.wq };
                            if best2.map(|b| v < b.2).unwrap_or(true) {
                                best2 = Some((s, q, v));
                            }
                            row.push(Cell::Num(tidy(v)));
                        }
                        Err(_) => row.push(Cell::from("—")),
                    }
                }
                grid.push((s, row));
            }
            for (s, row) in grid {
                t2.row(row);
                if let Some((bs, bq, _)) = best2 {
                    if bs == s {
                        let r = s - a;
                        t2.mark(r, bq - qa + 1, Tone::Highlight);
                    }
                }
            }
            if let Some((bs, bq, v)) = best2 {
                cap_blocks.push(text(format!("{} con {bs} servidores y una cola de {bq} lugares ({}).", if has_cost { "El costo total mínimo se obtiene" } else { "La menor espera Wq se obtiene" }, fmt(tidy(v)))));
                data["best_capacity"] = json!({"servers": bs, "queue_capacity": bq, "value": tidy(v)});
            }
            cap_blocks.push(t2.block());
        }
    }
    // sensibilidad
    let mut sens_blocks = vec![];
    if o.sensitivity != SensParam::None && base.is_ok() {
        let p = o.sensitivity;
        let mut t = Table::new(Some("Análisis de sensibilidad"), &[p.label(), "L", "Lq", &format!("W ({u})"), &format!("Wq ({u})"), "Utilización (%)", &format!("Costo por {u}")]).left(0);
        let pts: Vec<f64> = if p.integer() {
            let (a, b) = (o.sens_from.max(0.0) as usize, o.sens_to.max(o.sens_from).max(0.0) as usize);
            (a..=b.min(a + 200)).map(|x| x as f64).collect()
        } else {
            let k = o.sens_steps.clamp(1, 500) as usize;
            (0..=k).map(|i| o.sens_from + (o.sens_to - o.sens_from) * i as f64 / k as f64).collect()
        };
        let mut series = vec![];
        let mut cost_series = vec![];
        for x in pts {
            let mut y = st.clone();
            match p {
                SensParam::Servers => y.s = x as usize,
                SensParam::ServiceRate => y.mu = x,
                SensParam::Pressure => y.a = x,
                SensParam::ArrivalRate => y.lam = x,
                SensParam::Discourage => y.b = x,
                SensParam::QueueCapacity => y.qcap = Some(x as usize),
                SensParam::Population => y.pop = if x > POP_INF { None } else { Some(x as usize) },
                SensParam::BusyServerCost => y.costs.busy = x,
                SensParam::IdleServerCost => y.costs.idle = x,
                SensParam::WaitingCost => y.costs.wait = x,
                SensParam::ServiceCost => y.costs.serv = x,
                SensParam::BalkingCost => y.costs.balk = x,
                SensParam::CapacityCost => y.costs.cap = x,
                SensParam::None => {}
            }
            let ok = y.s > 0 && y.lam > 0.0 && y.mu > 0.0 && y.pop != Some(0);
            match if ok { analyze(&y) } else { Err("fuera de rango".into()) } {
                Ok(q) => {
                    let c = cost_of(&q, y.s, y.qcap, &y.costs);
                    t.row(cells![tidy(x), tidy(q.l), tidy(q.lq), tidy(q.w), tidy(q.wq), tidy(100.0 * q.util), if has_cost || y.costs.any() { Cell::Num(tidy(c[6])) } else { Cell::Empty }]);
                    series.push([x, q.l]);
                    cost_series.push([x, c[6]]);
                }
                Err(_) => {
                    t.row_tone(cells![tidy(x), "inestable o sin fórmula", Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty, Cell::Empty], Tone::Muted);
                }
            }
        }
        let cost_param = matches!(p, SensParam::BusyServerCost | SensParam::IdleServerCost | SensParam::WaitingCost | SensParam::ServiceCost | SensParam::BalkingCost | SensParam::CapacityCost);
        if cost_param {
            sens_blocks.push(Chart::new(Some("Costo total según el parámetro"), p.label(), "Costo total").series(Series::new("Costo total", SeriesKind::Line, cost_series).tone(Tone::Highlight)).block());
        } else {
            sens_blocks.push(Chart::new(Some("L según el parámetro"), p.label(), "L").series(Series::new("L", SeriesKind::Line, series).tone(Tone::Highlight)).block());
        }
        sens_blocks.push(t.block());
    }
    // simulación
    if need_sim {
        let horizon = if o.simulation_time > 0.0 { o.simulation_time } else { 20000.0 / st.lam.max(1e-9) };
        if o.warmup >= horizon {
            return invalid("el inicio de la recolección debe ser menor que el tiempo simulado");
        }
        let so = simulate(&st, horizon, o.warmup, o.seed);
        let mut t = Table::new(Some("Resultados de la simulación"), &["Medida", "Simulado", "Fórmula"]).left(0);
        let f = |x: Option<f64>| x.map(|v| Cell::Num(tidy(v))).unwrap_or(Cell::Empty);
        let b = base.as_ref().ok();
        t.row(cells!["Clientes en el sistema (L)", tidy(so.l), f(b.map(|p| p.l))]);
        t.row(cells!["Clientes en la cola (Lq)", tidy(so.lq), f(b.map(|p| p.lq))]);
        t.row(cells![format!("Tiempo en el sistema (W, {u})"), tidy(so.w), f(b.map(|p| p.w))]);
        t.row(cells![format!("Tiempo en la cola (Wq, {u})"), tidy(so.wq), f(b.map(|p| p.wq))]);
        t.row(cells!["Utilización (%)", tidy(100.0 * so.util), f(b.map(|p| 100.0 * p.util))]);
        t.row(cells!["Rechazados (%)", tidy(100.0 * so.balk), f(b.map(|p| 100.0 * p.p_balk))]);
        t.row(cells!["Clientes atendidos", so.served as f64, Cell::Empty]);
        t.row(cells![format!("Espera máxima en la cola ({u})"), tidy(so.max_wq), Cell::Empty]);
        sim_blocks.push(metrics(vec![metric_main("L simulado", tidy(so.l)), metric(format!("W simulado ({u})"), tidy(so.w)), metric("Tiempo simulado", tidy(horizon))]));
        sim_blocks.push(t.block());
        if base.is_err() {
            sol.push(metrics(vec![metric_main("L (simulado)", tidy(so.l)), metric("Lq (simulado)", tidy(so.lq)), metric(format!("W simulado ({u})"), tidy(so.w)), metric(format!("Wq simulado ({u})"), tidy(so.wq))]));
            data = json!({"model": "simulación", "l": tidy(so.l), "lq": tidy(so.lq), "w": tidy(so.w), "wq": tidy(so.wq), "utilization": tidy(so.util)});
        }
        data["simulation"] = json!({"l": tidy(so.l), "lq": tidy(so.lq), "w": tidy(so.w), "wq": tidy(so.wq), "utilization": tidy(so.util), "balked": tidy(so.balk), "served": so.served});
    }
    let mut r = Report::new("qa", &m.name, status, headline).section("solucion", "Solución", sol);
    if !st_blocks.is_empty() {
        r = r.section("probabilidades", "Probabilidades", st_blocks);
    }
    if !cap_blocks.is_empty() {
        r = r.section("capacidad", "Capacidad", cap_blocks);
    }
    if !sens_blocks.is_empty() {
        r = r.section("sensibilidad", "Sensibilidad", sens_blocks);
    }
    if !sim_blocks.is_empty() {
        r = r.section("simulacion", "Simulación", sim_blocks);
    }
    r = r.section("pasos", "Paso a paso", steps_v.map(|v| vec![steps(v)]).unwrap_or_default());
    Ok(r.data(data))
}

fn explain(st: &Sys, p: &Perf, u: &str, c: Option<[f64; 7]>) -> Vec<Step> {
    let mut v = vec![];
    let ex: f64 = st.pmf.iter().enumerate().map(|(k, p)| k as f64 * p).sum();
    let rho = st.lam * ex / (st.s as f64 * st.mu);
    v.push(step(
        "Datos",
        format!(
            "λ = {} y μ = {} por {u}, s = {}{}: intensidad ρ = λ{}/(s·μ) = {}.",
            fmt(tidy(st.lam)),
            fmt(tidy(st.mu)),
            st.s,
            if st.batch_one() { String::new() } else { format!(", lote medio E[X] = {}", fmt(tidy(ex))) },
            if st.batch_one() { "" } else { "·E[X]" },
            fmt(tidy(rho))
        ),
        vec![],
    ));
    if st.markov && st.plain() && st.qcap.is_none() && st.pop.is_none() {
        v.push(step(
            "Fórmulas de M/M/s",
            format!(
                "P0 = [Σₙ₌₀^(s−1) aⁿ/n! + aˢ/(s!(1 − ρ))]⁻¹ con a = λ/μ = {}; P0 = {}. Lq = P0·aˢ·ρ/(s!(1 − ρ)²) = {}; L = Lq + λ/μ = {}; Wq = Lq/λ = {}; W = Wq + 1/μ = {} (ley de Little).",
                fmt(tidy(st.lam / st.mu)),
                fmt(tidy(p.p0)),
                fmt(tidy(p.lq)),
                fmt(tidy(p.l)),
                fmt(tidy(p.wq)),
                fmt(tidy(p.w))
            ),
            vec![],
        ));
    } else if st.markov {
        let mut parts = vec![];
        parts.push(if st.pop.is_some() { "λ(n) = (N − n)·λ".to_string() } else { "λ(n) = λ".to_string() });
        if st.b != 0.0 {
            parts.push(format!("multiplicada por (s/(n+1))^b con b = {} cuando n ≥ s", fmt(st.b)));
        }
        let mut sv = "μ(n) = n·μ si n ≤ s".to_string();
        sv += &if st.a != 0.0 { format!(" y (n/s)^a·s·μ con a = {} si n > s", fmt(st.a)) } else { " y s·μ si n > s".into() };
        let lim = match (st.qcap, st.pop) {
            (Some(q), Some(n)) => format!(" Hay a lo sumo {} clientes (cola de {q} y población de {n}).", (q + st.s).min(n)),
            (Some(q), None) => format!(" Hay a lo sumo {} clientes en el sistema (K = Q + s).", q + st.s),
            (None, Some(n)) => format!(" Población de {n} clientes."),
            _ => String::new(),
        };
        v.push(step(
            "Cadena de Markov",
            format!(
                "Con {}; {}.{lim} {} Se resuelve el balance de cada corte entre n y n + 1: P(n+1)·μ(n+1) = Σᵢ≤ₙ P(i)·λ(i)·P(lote > n − i), y se normaliza para que las P(n) sumen 1.",
                parts.join(" "),
                sv,
                if st.batch_one() { "Cada llegada trae un cliente.".to_string() } else { format!("Cada llegada trae un lote de clientes (media {}); los que no caben se rechazan.", fmt(tidy(ex))) }
            ),
            vec![],
        ));
        v.push(step(
            "Medidas",
            format!(
                "L = Σ n·P(n) = {}, Lq = Σ (n − s)⁺·P(n) = {}. Tasa efectiva λₑ = Σ λ(n)·P(n)·(clientes que entran) = {}; W = L/λₑ = {} y Wq = Lq/λₑ = {} (ley de Little).",
                fmt(tidy(p.l)),
                fmt(tidy(p.lq)),
                fmt(tidy(p.lam_eff)),
                fmt(tidy(p.w)),
                fmt(tidy(p.wq))
            ),
            vec![],
        ));
    } else if p.exact {
        v.push(step(
            "Fórmula de Pollaczek-Khinchine",
            format!("Lq = (λ²σ² + ρ²)/(2(1 − ρ)) con σ² = varianza del servicio = {}: Lq = {}. Wq = Lq/λ, W = Wq + 1/μ, L = λ·W.", fmt(tidy(st.sv.variance())), fmt(tidy(p.lq))),
            vec![],
        ));
    } else {
        v.push(step(
            "Aproximación de G/G/s",
            format!(
                "Como en WinQSB: Lq ≈ Lq(M/M/s)·[Var(servicio)·μ² + Var(entre llegadas)·λ²]/2 = Lq(M/M/s)·(cₛ² + cₐ²)/2, con cₐ² = {} y cₛ² = {}. Lq ≈ {}; el resto sigue las fórmulas de M/M/s.",
                fmt(tidy(st.ia.variance() * st.lam * st.lam)),
                fmt(tidy(st.sv.variance() * st.mu * st.mu)),
                fmt(tidy(p.lq))
            ),
            vec![],
        ));
    }
    v.push(step("Desempeño", "Lb = Lq/Pw y Wb = Wq/Pw miden la cola cuando el sistema está ocupado (Pw = Σ P(n) con n ≥ s).", vec![perf_table(p, st.s, u, c).block()]));
    if c.is_some() {
        v.push(step("Costos", "Costo total = Cs·(L − Lq) + Ci·(s − L + Lq) + Cw·Wq·λₑ + Cu·(W − Wq)·λₑ + Cb·B + Cq·Q, donde B = clientes rechazados por unidad de tiempo.", vec![]));
    }
    v
}

// ------------------------------------------------------------------ archivos

/// Distribución en cuatro filas: nombre y tres parámetros (la discreta: cantidad de valores y "v/p, …").
fn dist_rows(w: &mut Writer, title: &str, d: &DistIn) {
    if d.distribution == Dist::Discrete {
        let vals = d.values.clone().unwrap_or_default();
        let n = vals.split([',', ';']).filter(|s| !s.trim().is_empty()).count();
        w.row(&[title.to_string(), "Discrete".into()]);
        w.row(&["Number of discrete values".to_string(), n.to_string()]);
        w.row(&["Discrete values (v/p,v/p,...)".to_string(), vals]);
        w.row(&["(Not used)".to_string(), String::new()]);
    } else {
        crate::its::dist_rows(w, title, d);
    }
}

fn read_dist(r: &mut Reader) -> Result<DistIn, ModError> {
    let l = r.line("la distribución")?;
    let d = Dist::parse(l.get(1)).filter(|d| !matches!(d, Dist::ThreeTime | Dist::Mixed)).unwrap_or(Dist::Constant);
    let rows: Vec<String> = (0..3).map(|_| r.line("los parámetros de la distribución").map(|x| x.get(1).trim().to_string())).collect::<Result<_, _>>()?;
    let mut out = DistIn { distribution: d, p1: None, p2: None, p3: None, values: None };
    if d == Dist::Discrete {
        out.values = Some(if rows[1].contains('/') { rows[1].clone() } else { rows[0].clone() });
    } else {
        out.p1 = parse_num(&rows[0]);
        out.p2 = parse_num(&rows[1]);
        out.p3 = parse_num(&rows[2]);
        // parámetro de posición en blanco = 0
        if out.p1.is_none() && matches!(d, Dist::Exponential | Dist::Erlang | Dist::Gamma | Dist::Weibull | Dist::Laplace | Dist::PowerFunction) {
            out.p1 = Some(0.0);
        }
    }
    Ok(out)
}

fn write_qa(m: &QaModel) -> Vec<u8> {
    let mut w = Writer::new();
    let general = m.problem_type == QaType::General;
    w.row(&["QA".to_string(), m.name.clone(), m.time_unit.clone(), if general { "1" } else { "0" }.into()]);
    w.row(&["Data Description", "ENTRY"]);
    let u = &m.time_unit;
    let item = |w: &mut Writer, l: String, v: String| {
        w.row(&[l, v]);
    };
    let blank0 = |x: f64| if x == 0.0 { String::new() } else { num(x) };
    let bnd = |b: Bnd| if b.0.is_finite() { num(b.0) } else { String::new() };
    item(&mut w, "Number of servers".into(), m.servers.to_string());
    if general {
        dist_rows(&mut w, &format!("Service time distribution (in {u})"), &m.service);
        item(&mut w, "Service pressure coefficient".into(), blank0(m.pressure));
        dist_rows(&mut w, &format!("Interarrival time distribution (in {u})"), &m.interarrival);
        item(&mut w, "Arrival discourage coefficient".into(), blank0(m.discourage));
        dist_rows(&mut w, "Batch (bulk) size distribution", &m.batch);
    } else {
        item(&mut w, format!("Service rate (per server per {u})"), num(m.service_rate));
        item(&mut w, format!("Customer arrival rate (per {u})"), num(m.arrival_rate));
    }
    item(&mut w, "Queue capacity (maximum waiting space)".into(), bnd(m.queue_capacity));
    item(&mut w, "Customer population".into(), bnd(m.population));
    item(&mut w, format!("Busy server cost per {u}"), blank0(m.busy_server_cost));
    item(&mut w, format!("Idle server cost per {u}"), blank0(m.idle_server_cost));
    item(&mut w, format!("Customer waiting cost per {u}"), blank0(m.waiting_cost));
    item(&mut w, format!("Customer being served cost per {u}"), blank0(m.service_cost));
    item(&mut w, "Cost of customer being balked".into(), blank0(m.balking_cost));
    item(&mut w, "Unit queue capacity cost".into(), blank0(m.capacity_cost));
    w.bytes()
}

fn read_qa(bytes: &[u8]) -> Result<QaModel, ModError> {
    let mut r = Reader::new(bytes);
    let h = r.line("la cabecera")?;
    if !h.get(0).trim().to_ascii_uppercase().starts_with("QA") {
        return Err(ModError::Format("no es un archivo de QA".into()));
    }
    let general = parse_num(h.get(3)).unwrap_or(0.0) as i64 == 1;
    let mut m: QaModel = serde_json::from_value(json!({"name": h.get(1).trim(), "problem_type": if general { "general" } else { "simple" }})).unwrap();
    if m.name.is_empty() {
        m.name = "Queuing Problem".into();
    }
    if !h.get(2).trim().is_empty() {
        m.time_unit = h.get(2).trim().to_string();
    }
    r.line("la fila de títulos")?;
    let val = |r: &mut Reader, what: &str| -> Result<String, ModError> { Ok(r.line(what)?.get(1).trim().to_string()) };
    let n0 = |s: &str| parse_num(s).unwrap_or(0.0);
    let bnd = |s: &str| parse_num(s).map(Bnd).unwrap_or(Bnd::INF);
    m.servers = n0(&val(&mut r, "el número de servidores")?).max(1.0) as u32;
    if general {
        m.service = read_dist(&mut r)?;
        m.pressure = n0(&val(&mut r, "el coeficiente de presión")?);
        m.interarrival = read_dist(&mut r)?;
        m.discourage = n0(&val(&mut r, "el coeficiente de desaliento")?);
        m.batch = read_dist(&mut r)?;
        if m.batch.distribution == Dist::Constant && m.batch.p1.is_none() {
            m.batch.p1 = Some(1.0);
        }
    } else {
        m.service_rate = n0(&val(&mut r, "la tasa de servicio")?);
        m.arrival_rate = n0(&val(&mut r, "la tasa de llegada")?);
    }
    m.queue_capacity = bnd(&val(&mut r, "la capacidad de la cola")?);
    m.population = bnd(&val(&mut r, "la población")?);
    let next = |r: &mut Reader| r.try_line().map(|l| n0(l.get(1))).unwrap_or(0.0);
    m.busy_server_cost = next(&mut r);
    m.idle_server_cost = next(&mut r);
    m.waiting_cost = next(&mut r);
    m.service_cost = next(&mut r);
    m.balking_cost = next(&mut r);
    m.capacity_cost = next(&mut r);
    Ok(m)
}
