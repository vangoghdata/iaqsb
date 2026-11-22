//! JOB · Programación de trabajos (equivalente al módulo JOB de WinQSB).
//!
//! - Taller (job shop): programación por despacho con quince reglas de
//!   prioridad (y regla de desempate), todas las reglas o generación aleatoria.
//! - Flujo (flow shop): Johnson, Palmer, CDS, Gupta, Dannenbring, Hundal y
//!   Rajgopal, Ho y Chang (mejora por intercambios), NEH, todas, aleatoria y
//!   enumeración completa.
//! - Las 18 medidas de desempeño de WinQSB (Cmax, flujo, espera, retraso,
//!   tardanza, adelanto, utilización, WIP, costos) y diagramas de Gantt.

use iaqsb_core::winqsb::{num, parse_num, Reader, Writer};
use iaqsb_core::*;
use rand::{RngExt, SeedableRng};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    #[default]
    JobShop,
    FlowShop,
}

fn one() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Job {
    pub name: String,
    /// Taller: operaciones "tiempo/máquina" en orden, separadas por comas (p. ej. "12/1, 5/M3").
    #[serde(default)]
    pub operations: String,
    #[serde(default)]
    pub ready: f64,
    #[serde(default)]
    pub due: Option<f64>,
    #[serde(default = "one")]
    pub weight: f64,
    /// Índice de prioridad (menor = más urgente).
    #[serde(default)]
    pub priority: f64,
    /// Costo por unidad de tiempo esperando.
    #[serde(default)]
    pub idle_cost: f64,
    /// Costo por unidad de tiempo en proceso.
    #[serde(default)]
    pub busy_cost: f64,
    #[serde(default)]
    pub late_cost: f64,
    #[serde(default)]
    pub early_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Machine {
    pub name: String,
    #[serde(default)]
    pub ready: f64,
    #[serde(default)]
    pub idle_cost: f64,
    #[serde(default)]
    pub busy_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JobModel {
    #[serde(default)]
    pub name: String,
    pub problem_type: JobType,
    #[serde(default = "minute")]
    pub time_unit: String,
    pub jobs: Vec<Job>,
    pub machines: Vec<Machine>,
    /// Flujo: tiempos de proceso, trabajo × máquina (en el orden de las máquinas).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub times: Vec<Vec<f64>>,
}

fn minute() -> String {
    "minuto".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    Fcfs,
    Lcfs,
    #[default]
    Spt,
    Lpt,
    Edd,
    Cr,
    Slack,
    SlackPerOp,
    Mwkr,
    Lwkr,
    Mopnr,
    Lopnr,
    Wspt,
    PriorityIndex,
    Random,
}

impl Rule {
    pub const ALL: [Rule; 15] = [
        Rule::Fcfs,
        Rule::Lcfs,
        Rule::Spt,
        Rule::Lpt,
        Rule::Edd,
        Rule::Cr,
        Rule::Slack,
        Rule::SlackPerOp,
        Rule::Mwkr,
        Rule::Lwkr,
        Rule::Mopnr,
        Rule::Lopnr,
        Rule::Wspt,
        Rule::PriorityIndex,
        Rule::Random,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Rule::Fcfs => "fcfs",
            Rule::Lcfs => "lcfs",
            Rule::Spt => "spt",
            Rule::Lpt => "lpt",
            Rule::Edd => "edd",
            Rule::Cr => "cr",
            Rule::Slack => "slack",
            Rule::SlackPerOp => "slack_per_op",
            Rule::Mwkr => "mwkr",
            Rule::Lwkr => "lwkr",
            Rule::Mopnr => "mopnr",
            Rule::Lopnr => "lopnr",
            Rule::Wspt => "wspt",
            Rule::PriorityIndex => "priority_index",
            Rule::Random => "random",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Rule::Fcfs => "FCFS (primero en llegar)",
            Rule::Lcfs => "LCFS (último en llegar)",
            Rule::Spt => "SPT (menor tiempo de proceso)",
            Rule::Lpt => "LPT (mayor tiempo de proceso)",
            Rule::Edd => "EDD (fecha de entrega más próxima)",
            Rule::Cr => "CR (razón crítica)",
            Rule::Slack => "Holgura mínima",
            Rule::SlackPerOp => "S/ROP (holgura por operación restante)",
            Rule::Mwkr => "MWKR (más trabajo restante)",
            Rule::Lwkr => "LWKR (menos trabajo restante)",
            Rule::Mopnr => "MOPNR (más operaciones restantes)",
            Rule::Lopnr => "LOPNR (menos operaciones restantes)",
            Rule::Wspt => "WSPT (tiempo de proceso / peso)",
            Rule::PriorityIndex => "Índice de prioridad",
            Rule::Random => "Al azar",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum FlowMethod {
    #[default]
    Cds,
    Johnson,
    Palmer,
    Gupta,
    Dannenbring,
    HundalRajgopal,
    HoChang,
    Neh,
    AllHeuristics,
    Random,
    Enumeration,
}

impl FlowMethod {
    pub const HEUR: [FlowMethod; 8] = [FlowMethod::Cds, FlowMethod::Johnson, FlowMethod::Palmer, FlowMethod::Gupta, FlowMethod::Dannenbring, FlowMethod::HundalRajgopal, FlowMethod::HoChang, FlowMethod::Neh];
    pub fn label(self) -> &'static str {
        match self {
            FlowMethod::Cds => "CDS (Campbell, Dudek y Smith)",
            FlowMethod::Johnson => "Johnson",
            FlowMethod::Palmer => "Palmer",
            FlowMethod::Gupta => "Gupta",
            FlowMethod::Dannenbring => "Dannenbring (acceso rápido)",
            FlowMethod::HundalRajgopal => "Hundal y Rajgopal",
            FlowMethod::HoChang => "Ho y Chang (mejora por intercambios)",
            FlowMethod::Neh => "NEH (Nawaz, Enscore y Ham)",
            FlowMethod::AllHeuristics => "Todas las heurísticas",
            FlowMethod::Random => "Generación aleatoria",
            FlowMethod::Enumeration => "Enumeración completa",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ShopMethod {
    #[default]
    Rule,
    AllRules,
    Random,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub enum Criterion {
    #[default]
    #[serde(rename = "cmax")]
    Cmax,
    #[serde(rename = "mc")]
    Mc,
    #[serde(rename = "mf")]
    Mf,
    #[serde(rename = "fmax")]
    Fmax,
    #[serde(rename = "mw")]
    Mw,
    #[serde(rename = "wmax")]
    Wmax,
    #[serde(rename = "ml")]
    Ml,
    #[serde(rename = "lmax")]
    Lmax,
    #[serde(rename = "mt")]
    Mt,
    #[serde(rename = "tmax")]
    Tmax,
    #[serde(rename = "me")]
    Me,
    #[serde(rename = "emax")]
    Emax,
    #[serde(rename = "nt")]
    Nt,
    #[serde(rename = "mu")]
    Mu,
    #[serde(rename = "wip")]
    Wip,
    #[serde(rename = "tjc")]
    Tjc,
    #[serde(rename = "tmc")]
    Tmc,
    #[serde(rename = "tc")]
    Tc,
}

pub const CRITERIA: [(Criterion, &str, &str); 18] = [
    (Criterion::Cmax, "cmax", "Cmax: tiempo total (makespan)"),
    (Criterion::Mc, "mc", "MC: terminación media ponderada"),
    (Criterion::Mf, "mf", "MF: flujo medio ponderado"),
    (Criterion::Fmax, "fmax", "Fmax: flujo máximo"),
    (Criterion::Mw, "mw", "MW: espera media ponderada"),
    (Criterion::Wmax, "wmax", "Wmax: espera máxima"),
    (Criterion::Ml, "ml", "ML: retraso medio ponderado"),
    (Criterion::Lmax, "lmax", "Lmax: retraso máximo"),
    (Criterion::Mt, "mt", "MT: tardanza media ponderada"),
    (Criterion::Tmax, "tmax", "Tmax: tardanza máxima"),
    (Criterion::Me, "me", "ME: adelanto medio ponderado"),
    (Criterion::Emax, "emax", "Emax: adelanto máximo"),
    (Criterion::Nt, "nt", "NT: trabajos tardíos"),
    (Criterion::Mu, "mu", "MU: utilización media de las máquinas (se maximiza)"),
    (Criterion::Wip, "wip", "WIP: trabajo en proceso medio"),
    (Criterion::Tjc, "tjc", "TJC: costo total de los trabajos"),
    (Criterion::Tmc, "tmc", "TMC: costo total de las máquinas"),
    (Criterion::Tc, "tc", "TC: costo total"),
];

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct JobOptions {
    pub steps: bool,
    pub criterion: Criterion,
    pub shop_method: ShopMethod,
    pub rule: Rule,
    pub tie_breaker: Rule,
    pub flow_method: FlowMethod,
    pub schedules: u32,
    pub seed: u64,
}

impl Default for JobOptions {
    fn default() -> Self {
        JobOptions { steps: true, criterion: Criterion::Cmax, shop_method: ShopMethod::Rule, rule: Rule::Spt, tie_breaker: Rule::Fcfs, flow_method: FlowMethod::Cds, schedules: 1000, seed: 27437 }
    }
}

// ---------------------------------------------------------------- datos y programa

#[derive(Clone)]
struct Data {
    names: Vec<String>,
    mnames: Vec<String>,
    /// Operaciones de cada trabajo: (máquina, tiempo).
    ops: Vec<Vec<(usize, f64)>>,
    ready: Vec<f64>,
    due: Vec<Option<f64>>,
    weight: Vec<f64>,
    prio: Vec<f64>,
    jcost: Vec<[f64; 4]>,
    mready: Vec<f64>,
    mcost: Vec<[f64; 2]>,
}

/// Programa: para cada trabajo, (máquina, inicio, fin) de cada operación.
#[derive(Clone)]
pub struct Sched {
    ops: Vec<Vec<(usize, f64, f64)>>,
}

#[derive(Clone, Debug, Default)]
pub struct Measures {
    pub v: [f64; 18],
}

fn measures(d: &Data, s: &Sched) -> Measures {
    let n = d.names.len();
    let nm = d.mnames.len();
    let wsum: f64 = d.weight.iter().sum::<f64>().max(1e-12);
    let mut c = vec![0.0; n];
    let mut f = vec![0.0; n];
    let mut w = vec![0.0; n];
    let mut l = vec![0.0; n];
    let mut proc_t = vec![0.0; n];
    for j in 0..n {
        let p: f64 = s.ops[j].iter().map(|o| o.2 - o.1).sum();
        proc_t[j] = p;
        c[j] = s.ops[j].iter().map(|o| o.2).fold(d.ready[j], f64::max);
        f[j] = c[j] - d.ready[j];
        w[j] = f[j] - p;
        l[j] = d.due[j].map(|dd| c[j] - dd).unwrap_or(0.0);
    }
    let cmax = c.iter().cloned().fold(0.0, f64::max);
    let wmean = |v: &[f64]| (0..n).map(|j| d.weight[j] * v[j]).sum::<f64>() / wsum;
    let mx = |v: &[f64]| v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let t: Vec<f64> = l.iter().map(|x| x.max(0.0)).collect();
    let e: Vec<f64> = l.iter().map(|x| (-x).max(0.0)).collect();
    let mut busy = vec![0.0; nm];
    for j in 0..n {
        for o in &s.ops[j] {
            busy[o.0] += o.2 - o.1;
        }
    }
    let mu = (0..nm).map(|k| if cmax > d.mready[k] { busy[k] / (cmax - d.mready[k]) } else { 0.0 }).sum::<f64>() / nm.max(1) as f64;
    let wip = if cmax > 0.0 { f.iter().sum::<f64>() / cmax } else { 0.0 };
    let tjc: f64 = (0..n).map(|j| d.jcost[j][0] * w[j] + d.jcost[j][1] * proc_t[j] + d.jcost[j][2] * t[j] + d.jcost[j][3] * e[j]).sum();
    let tmc: f64 = (0..nm).map(|k| d.mcost[k][0] * (cmax - d.mready[k] - busy[k]).max(0.0) + d.mcost[k][1] * busy[k]).sum();
    let nt = t.iter().filter(|x| **x > 1e-9).count() as f64;
    Measures { v: [cmax, wmean(&c), wmean(&f), mx(&f), wmean(&w), mx(&w), wmean(&l), mx(&l), wmean(&t), mx(&t), wmean(&e), mx(&e), nt, mu, wip, tjc, tmc, tjc + tmc] }
}

fn score(m: &Measures, c: Criterion) -> f64 {
    let i = CRITERIA.iter().position(|x| x.0 == c).unwrap();
    if c == Criterion::Mu {
        -m.v[i]
    } else {
        m.v[i]
    }
}

/// Programa de un orden de trabajos en un taller de flujo (permutación).
fn flow_sched(d: &Data, order: &[usize]) -> Sched {
    let nm = d.mnames.len();
    let mut mfree = d.mready.clone();
    let mut ops = vec![vec![]; d.names.len()];
    for &j in order {
        let mut t = d.ready[j];
        for (k, &(m, p)) in d.ops[j].iter().enumerate() {
            let _ = k;
            let st = t.max(mfree[m]);
            let en = st + p;
            ops[j].push((m, st, en));
            mfree[m] = en;
            t = en;
        }
    }
    let _ = nm;
    Sched { ops }
}

fn makespan(d: &Data, order: &[usize]) -> f64 {
    measures(d, &flow_sched(d, order)).v[0]
}

/// Regla de Johnson para dos "máquinas" con tiempos a y b.
fn johnson(a: &[f64], b: &[f64]) -> Vec<usize> {
    let n = a.len();
    let mut first: Vec<usize> = (0..n).filter(|&j| a[j] < b[j]).collect();
    let mut last: Vec<usize> = (0..n).filter(|&j| a[j] >= b[j]).collect();
    first.sort_by(|&x, &y| a[x].partial_cmp(&a[y]).unwrap().then(x.cmp(&y)));
    last.sort_by(|&x, &y| b[y].partial_cmp(&b[x]).unwrap().then(x.cmp(&y)));
    first.extend(last);
    first
}

fn flow_heuristic(d: &Data, meth: FlowMethod) -> Vec<usize> {
    let n = d.names.len();
    let m = d.mnames.len();
    let p = |j: usize, k: usize| d.ops[j][k].1;
    let sort_by_key = |key: &dyn Fn(usize) -> f64| {
        let mut o: Vec<usize> = (0..n).collect();
        o.sort_by(|&x, &y| key(x).partial_cmp(&key(y)).unwrap().then(x.cmp(&y)));
        o
    };
    match meth {
        FlowMethod::Johnson => {
            let half = m / 2;
            let (a, b): (Vec<f64>, Vec<f64>) = if m == 2 {
                ((0..n).map(|j| p(j, 0)).collect(), (0..n).map(|j| p(j, 1)).collect())
            } else if m == 3 {
                ((0..n).map(|j| p(j, 0) + p(j, 1)).collect(), (0..n).map(|j| p(j, 1) + p(j, 2)).collect())
            } else {
                ((0..n).map(|j| (0..half).map(|k| p(j, k)).sum()).collect(), (0..n).map(|j| (m - half..m).map(|k| p(j, k)).sum()).collect())
            };
            johnson(&a, &b)
        }
        FlowMethod::Palmer => sort_by_key(&|j| -(0..m).map(|k| (2.0 * (k + 1) as f64 - m as f64 - 1.0) * p(j, k)).sum::<f64>()),
        FlowMethod::HundalRajgopal => {
            let cands = [0.0, 1.0, -1.0].map(|shift: f64| sort_by_key(&|j| -(0..m).map(|k| (2.0 * (k + 1) as f64 - m as f64 - 1.0 + shift) * p(j, k)).sum::<f64>()));
            cands.into_iter().min_by(|x, y| makespan(d, x).partial_cmp(&makespan(d, y)).unwrap()).unwrap()
        }
        FlowMethod::Cds => {
            let mut best: Option<(f64, Vec<usize>)> = None;
            for k in 1..m.max(2) {
                let a: Vec<f64> = (0..n).map(|j| (0..k.min(m)).map(|q| p(j, q)).sum()).collect();
                let b: Vec<f64> = (0..n).map(|j| (m.saturating_sub(k)..m).map(|q| p(j, q)).sum()).collect();
                let o = johnson(&a, &b);
                let c = makespan(d, &o);
                if best.as_ref().map(|x| c < x.0 - 1e-12).unwrap_or(true) {
                    best = Some((c, o));
                }
            }
            best.unwrap().1
        }
        FlowMethod::Gupta => sort_by_key(&|j| {
            let e = if p(j, 0) < p(j, m - 1) { 1.0 } else { -1.0 };
            let mn = if m >= 2 { (0..m - 1).map(|k| p(j, k) + p(j, k + 1)).fold(f64::INFINITY, f64::min) } else { p(j, 0) };
            e / mn.max(1e-12)
        }),
        FlowMethod::Dannenbring => {
            let a: Vec<f64> = (0..n).map(|j| (0..m).map(|k| (m - k) as f64 * p(j, k)).sum()).collect();
            let b: Vec<f64> = (0..n).map(|j| (0..m).map(|k| (k + 1) as f64 * p(j, k)).sum()).collect();
            johnson(&a, &b)
        }
        FlowMethod::HoChang => {
            let mut o = flow_heuristic(d, FlowMethod::Cds);
            let mut cur = makespan(d, &o);
            loop {
                let mut improved = false;
                for i in 0..n {
                    for j in i + 1..n {
                        o.swap(i, j);
                        let c = makespan(d, &o);
                        if c < cur - 1e-12 {
                            cur = c;
                            improved = true;
                        } else {
                            o.swap(i, j);
                        }
                    }
                }
                if !improved {
                    break;
                }
            }
            o
        }
        FlowMethod::Neh => {
            let mut by_total: Vec<usize> = (0..n).collect();
            by_total.sort_by(|&x, &y| {
                let tx: f64 = (0..m).map(|k| p(x, k)).sum();
                let ty: f64 = (0..m).map(|k| p(y, k)).sum();
                ty.partial_cmp(&tx).unwrap().then(x.cmp(&y))
            });
            let mut seq: Vec<usize> = vec![];
            for &j in &by_total {
                let mut best = (f64::INFINITY, 0);
                for pos in 0..=seq.len() {
                    let mut t = seq.clone();
                    t.insert(pos, j);
                    let c = makespan(d, &t);
                    if c < best.0 - 1e-12 {
                        best = (c, pos);
                    }
                }
                seq.insert(best.1, j);
            }
            seq
        }
        _ => (0..n).collect(),
    }
}

/// Clave de prioridad (menor = primero) de la regla para la operación siguiente del trabajo j.
fn rule_key(rule: Rule, d: &Data, j: usize, k: usize, t: f64, arrived: f64, rnd: f64) -> f64 {
    let p = d.ops[j][k].1;
    let rem_work: f64 = d.ops[j][k..].iter().map(|o| o.1).sum();
    let rem_ops = (d.ops[j].len() - k) as f64;
    let due = d.due[j].unwrap_or(f64::INFINITY);
    let slack = due - t - rem_work;
    match rule {
        Rule::Fcfs => arrived,
        Rule::Lcfs => -arrived,
        Rule::Spt => p,
        Rule::Lpt => -p,
        Rule::Edd => due,
        Rule::Cr => {
            if rem_work > 0.0 {
                (due - t) / rem_work
            } else {
                f64::INFINITY
            }
        }
        Rule::Slack => slack,
        Rule::SlackPerOp => slack / rem_ops,
        Rule::Mwkr => -rem_work,
        Rule::Lwkr => rem_work,
        Rule::Mopnr => -rem_ops,
        Rule::Lopnr => rem_ops,
        Rule::Wspt => p / d.weight[j].max(1e-12),
        Rule::PriorityIndex => d.prio[j],
        Rule::Random => rnd,
    }
}

/// Despacho sin retrasos innecesarios: cuando una máquina puede empezar,
/// elige entre las operaciones disponibles con la regla (y el desempate).
fn dispatch(d: &Data, rule: Rule, tie: Rule, rng: &mut rand_chacha::ChaCha8Rng, trace: &mut Vec<(f64, String, String, String)>, trace_n: usize) -> Sched {
    let n = d.names.len();
    let nm = d.mnames.len();
    let mut next = vec![0usize; n];
    let mut jfree = d.ready.clone();
    let mut mfree = d.mready.clone();
    let mut ops: Vec<Vec<(usize, f64, f64)>> = vec![vec![]; n];
    let rnd: Vec<f64> = (0..n).map(|_| rng.random()).collect();
    let rnd2: Vec<f64> = (0..n).map(|_| rng.random()).collect();
    loop {
        // operación disponible que puede empezar antes
        let mut best_t = f64::INFINITY;
        let mut best_m = usize::MAX;
        for j in 0..n {
            if next[j] < d.ops[j].len() {
                let m = d.ops[j][next[j]].0;
                let st = jfree[j].max(mfree[m]);
                if st < best_t - 1e-12 || (st <= best_t + 1e-12 && m < best_m) {
                    best_t = st;
                    best_m = m;
                }
            }
        }
        if best_m == usize::MAX {
            break;
        }
        let t = best_t;
        let m = best_m;
        let cands: Vec<usize> = (0..n).filter(|&j| next[j] < d.ops[j].len() && d.ops[j][next[j]].0 == m && jfree[j] <= t + 1e-12).collect();
        let key = |j: usize, r: Rule, rv: &[f64]| rule_key(r, d, j, next[j], t, jfree[j], rv[j]);
        let pick = *cands
            .iter()
            .min_by(|&&a, &&b| key(a, rule, &rnd).partial_cmp(&key(b, rule, &rnd)).unwrap_or(std::cmp::Ordering::Equal).then(key(a, tie, &rnd2).partial_cmp(&key(b, tie, &rnd2)).unwrap_or(std::cmp::Ordering::Equal)).then(a.cmp(&b)))
            .unwrap();
        let p = d.ops[pick][next[pick]].1;
        if trace.len() < trace_n {
            trace.push((t, d.mnames[m].clone(), cands.iter().map(|&j| d.names[j].clone()).collect::<Vec<_>>().join(", "), d.names[pick].clone()));
        }
        ops[pick].push((m, t, t + p));
        jfree[pick] = t + p;
        mfree[m] = t + p;
        next[pick] += 1;
    }
    let _ = nm;
    Sched { ops }
}

fn build(m: &JobModel) -> Result<Data, ModError> {
    let n = m.jobs.len();
    let nm = m.machines.len();
    if n == 0 || nm == 0 {
        return invalid("hacen falta trabajos y máquinas");
    }
    check_names("trabajos", &m.jobs.iter().map(|x| &x.name).collect::<Vec<_>>())?;
    check_names("máquinas", &m.machines.iter().map(|x| &x.name).collect::<Vec<_>>())?;
    let mnames: Vec<String> = m.machines.iter().map(|x| x.name.clone()).collect();
    let mut ops = vec![];
    for (j, job) in m.jobs.iter().enumerate() {
        let mut v = vec![];
        match m.problem_type {
            JobType::FlowShop => {
                for k in 0..nm {
                    let p = m.times.get(j).and_then(|r| r.get(k)).copied().unwrap_or(0.0);
                    if p < 0.0 {
                        return invalid(format!("{}: los tiempos no pueden ser negativos", job.name));
                    }
                    v.push((k, p));
                }
            }
            JobType::JobShop => {
                for part in job.operations.split([',', ';']).map(str::trim).filter(|s| !s.is_empty()) {
                    let part = part.trim_matches('"');
                    let (a, b) = part.split_once('/').ok_or_else(|| ModError::Invalid(format!("{}: «{part}» debe tener la forma tiempo/máquina (p. ej. 12.5/3)", job.name)))?;
                    let p = parse_num(a).ok_or_else(|| ModError::Invalid(format!("{}: «{a}» no es un tiempo", job.name)))?;
                    if p < 0.0 {
                        return invalid(format!("{}: los tiempos no pueden ser negativos", job.name));
                    }
                    let b = b.trim();
                    let k = mnames
                        .iter()
                        .position(|x| x.eq_ignore_ascii_case(b))
                        .or_else(|| b.parse::<usize>().ok().filter(|k| *k >= 1 && *k <= nm).map(|k| k - 1))
                        .ok_or_else(|| ModError::Invalid(format!("{}: la máquina «{b}» no existe", job.name)))?;
                    v.push((k, p));
                }
                if v.is_empty() {
                    return invalid(format!("{}: no tiene operaciones", job.name));
                }
            }
        }
        ops.push(v);
    }
    Ok(Data {
        names: m.jobs.iter().map(|x| x.name.clone()).collect(),
        mnames,
        ops,
        ready: m.jobs.iter().map(|x| x.ready).collect(),
        due: m.jobs.iter().map(|x| x.due).collect(),
        weight: m.jobs.iter().map(|x| x.weight).collect(),
        prio: m.jobs.iter().map(|x| x.priority).collect(),
        jcost: m.jobs.iter().map(|x| [x.idle_cost, x.busy_cost, x.late_cost, x.early_cost]).collect(),
        mready: m.machines.iter().map(|x| x.ready).collect(),
        mcost: m.machines.iter().map(|x| [x.idle_cost, x.busy_cost]).collect(),
    })
}

fn gantt(d: &Data, s: &Sched, by_machine: bool, unit: &str) -> Block {
    let tones = [Tone::Accent, Tone::Highlight, Tone::Good, Tone::Normal, Tone::Bad, Tone::Muted];
    let rows: Vec<GanttRow> = if by_machine {
        (0..d.mnames.len())
            .map(|k| {
                let mut bars: Vec<GanttBar> = vec![];
                for j in 0..d.names.len() {
                    for o in &s.ops[j] {
                        if o.0 == k && o.2 > o.1 {
                            bars.push(GanttBar { start: o.1, end: o.2, label: Some(d.names[j].clone()), tone: tones[j % tones.len()] });
                        }
                    }
                }
                bars.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
                GanttRow { label: d.mnames[k].clone(), bars }
            })
            .collect()
    } else {
        (0..d.names.len())
            .map(|j| GanttRow {
                label: d.names[j].clone(),
                bars: s.ops[j].iter().filter(|o| o.2 > o.1).map(|o| GanttBar { start: o.1, end: o.2, label: Some(d.mnames[o.0].clone()), tone: tones[o.0 % tones.len()] }).collect(),
            })
            .collect()
    };
    Block::Gantt(Gantt { title: Some(if by_machine { "Gantt por máquina".into() } else { "Gantt por trabajo".into() }), unit: unit.into(), rows, markers: vec![], legend: vec![] })
}

fn heap_permutations(n: usize, f: &mut dyn FnMut(&[usize])) {
    let mut a: Vec<usize> = (0..n).collect();
    let mut c = vec![0usize; n];
    f(&a);
    let mut i = 0;
    while i < n {
        if c[i] < i {
            if i % 2 == 0 {
                a.swap(0, i);
            } else {
                a.swap(c[i], i);
            }
            f(&a);
            c[i] += 1;
            i = 0;
        } else {
            c[i] = 0;
            i += 1;
        }
    }
}

pub struct JobMod;

impl TypedModule for JobMod {
    type Model = JobModel;
    type Options = JobOptions;

    fn file_tag(&self) -> &'static str {
        "SCH"
    }

    fn spec(&self) -> ModuleSpec {
        let job_cols = |shop: bool| {
            let mut v = vec![Field::text("name", "Trabajo")];
            if shop {
                v.push(Field::list("operations", "Operaciones (tiempo/máquina)").help("En orden, p. ej. 12/1, 5/3 o 12/Torno"));
            }
            v.extend([
                Field::number("ready", "Disponible desde"),
                Field::number("due", "Fecha de entrega").optional(),
                Field::number("weight", "Peso").default(1.0),
                Field::number("priority", "Índice de prioridad"),
                Field::number("idle_cost", "Costo esperando"),
                Field::number("busy_cost", "Costo en proceso"),
                Field::number("late_cost", "Costo por retraso"),
                Field::number("early_cost", "Costo por adelanto"),
            ]);
            v
        };
        let mach_cols = vec![Field::text("name", "Máquina"), Field::number("ready", "Disponible desde"), Field::number("idle_cost", "Costo ociosa"), Field::number("busy_cost", "Costo ocupada")];
        let mut jobs_axis = Axis::new("jobs", "Trabajo").add("Añadir trabajo");
        for f in job_cols(false).into_iter().skip(1) {
            jobs_axis = jobs_axis.extra(f);
        }
        let mut mach_axis = Axis::new("machines", "Máquina").add("Añadir máquina");
        for f in mach_cols.clone().into_iter().skip(1) {
            mach_axis = mach_axis.extra(f);
        }
        let crit: Vec<(&str, &str)> = CRITERIA.iter().map(|c| (c.1, c.2)).collect();
        let rules: Vec<(&str, &str)> = Rule::ALL.iter().map(|r| (r.id(), r.label())).collect();
        let flows = [
            ("cds", FlowMethod::Cds),
            ("johnson", FlowMethod::Johnson),
            ("palmer", FlowMethod::Palmer),
            ("gupta", FlowMethod::Gupta),
            ("dannenbring", FlowMethod::Dannenbring),
            ("hundal_rajgopal", FlowMethod::HundalRajgopal),
            ("ho_chang", FlowMethod::HoChang),
            ("neh", FlowMethod::Neh),
            ("all_heuristics", FlowMethod::AllHeuristics),
            ("random", FlowMethod::Random),
            ("enumeration", FlowMethod::Enumeration),
        ];
        let flow_opts: Vec<(&str, &str)> = flows.iter().map(|(id, f)| (*id, f.label())).collect();
        ModuleSpec {
            id: "job".into(),
            winqsb: "JOB".into(),
            name: "Programación de trabajos".into(),
            description: "Taller con 15 reglas de despacho y flujo con Johnson, CDS, Palmer, NEH y más; 18 medidas de desempeño".into(),
            file_ext: "JOB".into(),
            problem_types: vec![
                ProblemType {
                    id: "job_shop".into(),
                    label: "Taller (job shop)".into(),
                    description: "Cada trabajo tiene su propia ruta de máquinas".into(),
                    dims: vec![Field::integer("jobs", "Número de trabajos", 4), Field::integer("machines", "Número de máquinas", 3)],
                    layout: vec![
                        Widget::params(None, vec![Field::text("time_unit", "Unidad de tiempo")]),
                        Widget::records(Some("Trabajos"), "jobs", job_cols(true), Some("Añadir trabajo")),
                        Widget::records(Some("Máquinas"), "machines", mach_cols, Some("Añadir máquina")),
                    ],
                },
                ProblemType {
                    id: "flow_shop".into(),
                    label: "Flujo (flow shop)".into(),
                    description: "Todos los trabajos pasan por las máquinas en el mismo orden".into(),
                    dims: vec![Field::integer("jobs", "Número de trabajos", 5), Field::integer("machines", "Número de máquinas", 3)],
                    layout: vec![Widget::params(None, vec![Field::text("time_unit", "Unidad de tiempo")]), Widget::matrix(Some("Tiempos de proceso (trabajo × máquina)"), "times", jobs_axis, mach_axis, Field::number("p", "Tiempo"))],
                },
            ],
            options: vec![
                Field::boolean("steps", "Mostrar paso a paso", true),
                Field::choice("criterion", "Criterio", &crit).default("cmax"),
                Field::choice("shop_method", "Método", &[("rule", "Regla de despacho"), ("all_rules", "Todas las reglas"), ("random", "Generación aleatoria")]).comparable().default("rule").show_if("problem_type", &["job_shop"]),
                Field::choice("rule", "Regla principal", &rules).comparable().default("spt").show_if("problem_type", &["job_shop"]).show_if("shop_method", &["rule", "all_rules"]),
                Field::choice("tie_breaker", "Desempate", &rules).default("fcfs").show_if("problem_type", &["job_shop"]).show_if("shop_method", &["rule", "all_rules"]),
                Field::choice("flow_method", "Método", &flow_opts).comparable().default("cds").show_if("problem_type", &["flow_shop"]),
                Field::integer("schedules", "Programas aleatorios", 1000).show_if("shop_method", &["random"]),
                Field::integer("seed", "Semilla", 27437),
            ],
        }
    }

    fn template(&self, pt: &str, dims: &Value) -> Result<JobModel, ModError> {
        let t: JobType = serde_json::from_value(Value::from(pt)).map_err(|_| ModError::Invalid(format!("tipo de problema desconocido: {pt}")))?;
        let n = dim(dims, "jobs", 4, 1, 500)?;
        let nm = dim(dims, "machines", 3, 1, 100)?;
        let jobs = (1..=n).map(|i| serde_json::from_value(json!({"name": format!("Trabajo{i}"), "operations": if t == JobType::JobShop { (1..=nm).map(|k| format!("1/{k}")).collect::<Vec<_>>().join(", ") } else { String::new() }})).unwrap()).collect();
        let machines = (1..=nm).map(|k| serde_json::from_value(json!({"name": format!("Máquina{k}")})).unwrap()).collect();
        Ok(JobModel { name: "Programación nueva".into(), problem_type: t, time_unit: "minuto".into(), jobs, machines, times: if t == JobType::FlowShop { vec![vec![1.0; nm]; n] } else { vec![] } })
    }

    fn solve(&self, m: &JobModel, o: &JobOptions) -> Result<Report, ModError> {
        solve(m, o)
    }

    fn read(&self, bytes: &[u8]) -> Result<JobModel, ModError> {
        read_job(bytes)
    }

    fn write(&self, m: &JobModel) -> Result<Vec<u8>, ModError> {
        Ok(write_job(m))
    }

    fn examples(&self) -> Vec<(&'static str, JobModel)> {
        let ex = |s: &str| serde_json::from_str::<JobModel>(s).unwrap();
        vec![("taller_cuatro_trabajos", ex(include_str!("ejemplos/taller.json"))), ("flujo_johnson", ex(include_str!("ejemplos/johnson.json"))), ("flujo_cinco_maquinas", ex(include_str!("ejemplos/flujo.json")))]
    }
}

fn solve(m: &JobModel, o: &JobOptions) -> Result<Report, ModError> {
    let d = build(m)?;
    let n = d.names.len();
    let u = &m.time_unit;
    let crit_i = CRITERIA.iter().position(|c| c.0 == o.criterion).unwrap();
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(o.seed);
    let mut trace = vec![];
    let mut comparison: Vec<(String, Measures)> = vec![];
    let mut steps_v = vec![];
    let (sched, method_txt) = match m.problem_type {
        JobType::JobShop => match o.shop_method {
            ShopMethod::Rule => {
                let s = dispatch(&d, o.rule, o.tie_breaker, &mut rng, &mut trace, 30);
                for r in Rule::ALL {
                    let mut tmp = vec![];
                    let mut rr = rand_chacha::ChaCha8Rng::seed_from_u64(o.seed);
                    comparison.push((r.label().to_string(), measures(&d, &dispatch(&d, r, o.tie_breaker, &mut rr, &mut tmp, 0))));
                }
                (s, format!("regla {} con desempate {}", o.rule.label(), o.tie_breaker.label()))
            }
            ShopMethod::AllRules => {
                let mut best: Option<(f64, Sched, Rule)> = None;
                for r in Rule::ALL {
                    let mut tmp = vec![];
                    let mut rr = rand_chacha::ChaCha8Rng::seed_from_u64(o.seed);
                    let s = dispatch(&d, r, o.tie_breaker, &mut rr, &mut tmp, 0);
                    let ms = measures(&d, &s);
                    let sc = score(&ms, o.criterion);
                    comparison.push((r.label().to_string(), ms));
                    if best.as_ref().map(|b| sc < b.0 - 1e-12).unwrap_or(true) {
                        best = Some((sc, s, r));
                    }
                }
                let (_, s, r) = best.unwrap();
                let mut rr = rand_chacha::ChaCha8Rng::seed_from_u64(o.seed);
                dispatch(&d, r, o.tie_breaker, &mut rr, &mut trace, 30);
                (s, format!("la mejor de las 15 reglas: {}", r.label()))
            }
            ShopMethod::Random => {
                let k = o.schedules.clamp(1, 1_000_000);
                let mut best: Option<(f64, Sched)> = None;
                for _ in 0..k {
                    let mut tmp = vec![];
                    let s = dispatch(&d, Rule::Random, Rule::Random, &mut rng, &mut tmp, 0);
                    let sc = score(&measures(&d, &s), o.criterion);
                    if best.as_ref().map(|b| sc < b.0 - 1e-12).unwrap_or(true) {
                        best = Some((sc, s));
                    }
                }
                (best.unwrap().1, format!("el mejor de {k} programas aleatorios"))
            }
        },
        JobType::FlowShop => {
            let eval = |ord: &[usize]| {
                let s = flow_sched(&d, ord);
                let ms = measures(&d, &s);
                (score(&ms, o.criterion), s, ms)
            };
            for h in FlowMethod::HEUR {
                let ord = flow_heuristic(&d, h);
                comparison.push((format!("{} ({})", h.label(), ord.iter().map(|&j| d.names[j].as_str()).collect::<Vec<_>>().join("-")), eval(&ord).2));
            }
            match o.flow_method {
                FlowMethod::AllHeuristics => {
                    let mut best: Option<(f64, Sched, FlowMethod)> = None;
                    for h in FlowMethod::HEUR {
                        let (sc, s, _) = eval(&flow_heuristic(&d, h));
                        if best.as_ref().map(|b| sc < b.0 - 1e-12).unwrap_or(true) {
                            best = Some((sc, s, h));
                        }
                    }
                    let (_, s, h) = best.unwrap();
                    (s, format!("la mejor heurística: {}", h.label()))
                }
                FlowMethod::Random => {
                    let k = o.schedules.clamp(1, 1_000_000);
                    let mut best: Option<(f64, Sched)> = None;
                    let mut ord: Vec<usize> = (0..n).collect();
                    for _ in 0..k {
                        for i in (1..n).rev() {
                            let j = rng.random_range(0..=i);
                            ord.swap(i, j);
                        }
                        let (sc, s, _) = eval(&ord);
                        if best.as_ref().map(|b| sc < b.0 - 1e-12).unwrap_or(true) {
                            best = Some((sc, s));
                        }
                    }
                    (best.unwrap().1, format!("la mejor de {k} secuencias aleatorias"))
                }
                FlowMethod::Enumeration => {
                    if n > 10 {
                        return invalid("la enumeración completa admite hasta 10 trabajos (10! secuencias); usa otro método");
                    }
                    let mut best: Option<(f64, Vec<usize>)> = None;
                    heap_permutations(n, &mut |p| {
                        let sc = score(&measures(&d, &flow_sched(&d, p)), o.criterion);
                        if best.as_ref().map(|b| sc < b.0 - 1e-12).unwrap_or(true) {
                            best = Some((sc, p.to_vec()));
                        }
                    });
                    let ord = best.unwrap().1;
                    (flow_sched(&d, &ord), format!("enumeración de las {} secuencias (óptimo)", (1..=n).product::<usize>()))
                }
                h => {
                    let ord = flow_heuristic(&d, h);
                    if o.steps {
                        steps_v.extend(flow_steps(&d, h, &ord));
                    }
                    (flow_sched(&d, &ord), h.label().to_string())
                }
            }
        }
    };
    let ms = measures(&d, &sched);
    // tabla de desempeño
    let mut pt = Table::new(Some("Medidas de desempeño"), &["Medida", "Valor"]).left(0);
    for (i, c) in CRITERIA.iter().enumerate() {
        let v = if c.0 == Criterion::Mu { 100.0 * ms.v[i] } else { ms.v[i] };
        let label = if c.0 == Criterion::Mu { "MU: utilización media de las máquinas (%)" } else { c.2.split(" (se").next().unwrap() };
        pt.row_tone(cells![label, tidy(v)], if i == crit_i { Tone::Highlight } else { Tone::Normal });
    }
    // programa por trabajo
    let mut jt = Table::new(Some("Programa de los trabajos"), &["Trabajo", "Operación", "Máquina", &format!("Inicio ({u})"), "Fin", "Fecha de entrega", "Retraso"]).left(0).left(2);
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| sched.ops[a].first().map(|x| x.1).unwrap_or(0.0).partial_cmp(&sched.ops[b].first().map(|x| x.1).unwrap_or(0.0)).unwrap().then(a.cmp(&b)));
    for &j in &order {
        let cj = sched.ops[j].iter().map(|x| x.2).fold(d.ready[j], f64::max);
        for (k, op) in sched.ops[j].iter().enumerate() {
            let last = k + 1 == sched.ops[j].len();
            jt.row_tone(
                cells![if k == 0 { d.names[j].clone() } else { String::new() }, k + 1, &d.mnames[op.0], tidy(op.1), tidy(op.2), if last { d.due[j].map(tidy) } else { None }, if last { d.due[j].map(|dd| tidy(cj - dd)) } else { None }],
                if last && d.due[j].map(|dd| cj > dd + 1e-9).unwrap_or(false) { Tone::Accent } else { Tone::Normal },
            );
        }
    }
    let mut ct = Table::new(Some(if m.problem_type == JobType::JobShop { "Comparación de las reglas" } else { "Comparación de las heurísticas" }), &["Método", "Cmax", "MF", "MT", "Tmax", "NT", CRITERIA[crit_i].1]).left(0);
    let best_cmp = comparison.iter().map(|c| score(&c.1, o.criterion)).fold(f64::INFINITY, f64::min);
    for (name, mm) in &comparison {
        ct.row_tone(cells![name, tidy(mm.v[0]), tidy(mm.v[2]), tidy(mm.v[8]), tidy(mm.v[9]), mm.v[12], tidy(mm.v[crit_i])], if (score(mm, o.criterion) - best_cmp).abs() < 1e-9 { Tone::Highlight } else { Tone::Normal });
    }
    let mut st = vec![];
    if o.steps {
        if m.problem_type == JobType::JobShop {
            let mut tt = Table::new(Some("Decisiones de despacho"), &[&format!("Tiempo ({u})"), "Máquina", "Trabajos disponibles", "Elegido"]).left(1).left(2).left(3);
            for (t, mch, c, pk) in &trace {
                tt.row(cells![tidy(*t), mch, c, pk]);
            }
            steps_v.push(step(
                "Despacho",
                "Se busca la operación que puede empezar antes; en esa máquina compiten los trabajos ya disponibles y la regla elige uno (el desempate decide si empatan). Se repite hasta programar todas las operaciones.",
                vec![tt.block()],
            ));
        }
        steps_v.push(step("Medidas de desempeño", "Para cada trabajo: terminación C, flujo F = C − disponible, espera = F − proceso, retraso L = C − entrega, tardanza = máx(L, 0) y adelanto = máx(−L, 0). Las medias se ponderan por el peso de cada trabajo.", vec![pt.clone().block()]));
        if !comparison.is_empty() {
            steps_v.push(step("Comparar", "Con los mismos datos, cada método da un programa distinto: compara el criterio elegido.", vec![ct.clone().block()]));
        }
        st.push(steps(steps_v));
    }
    let headline = format!("{}: {} = {} (Cmax = {} {u}).", capitalize(&method_txt), CRITERIA[crit_i].1.to_uppercase(), fmt(tidy(if o.criterion == Criterion::Mu { 100.0 * ms.v[crit_i] } else { ms.v[crit_i] })), fmt(tidy(ms.v[0])));
    let seq: Vec<&str> = order.iter().map(|&j| d.names[j].as_str()).collect();
    let mut sol = vec![metrics(vec![metric_main(CRITERIA[crit_i].2.split(':').next().unwrap(), tidy(if o.criterion == Criterion::Mu { 100.0 * ms.v[crit_i] } else { ms.v[crit_i] })), metric("Cmax", tidy(ms.v[0])), metric("Trabajos tardíos", ms.v[12])])];
    if m.problem_type == JobType::FlowShop {
        sol.push(text(format!("Secuencia: {}", seq.join(" → "))));
    }
    sol.push(pt.block());
    sol.push(jt.block());
    Ok(Report::new("job", &m.name, if matches!(o.flow_method, FlowMethod::Enumeration) && m.problem_type == JobType::FlowShop { Status::Optimal } else { Status::Feasible }, headline)
        .section("solucion", "Solución", sol)
        .section("gantt", "Gantt", vec![gantt(&d, &sched, true, u), gantt(&d, &sched, false, u)])
        .section("comparacion", "Comparación", if comparison.is_empty() { vec![] } else { vec![ct.block()] })
        .section("pasos", "Paso a paso", st)
        .data(json!({
            "sequence": seq,
            "measures": CRITERIA.iter().enumerate().map(|(i, c)| (c.1.to_string(), json!(tidy(ms.v[i])))).collect::<serde_json::Map<_, _>>(),
            "schedule": (0..n).map(|j| json!({"job": d.names[j], "operations": sched.ops[j].iter().map(|o| json!({"machine": d.mnames[o.0], "start": tidy(o.1), "end": tidy(o.2)})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        })))
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn flow_steps(d: &Data, h: FlowMethod, ord: &[usize]) -> Vec<Step> {
    let n = d.names.len();
    let m = d.mnames.len();
    let p = |j: usize, k: usize| d.ops[j][k].1;
    let seq = ord.iter().map(|&j| d.names[j].as_str()).collect::<Vec<_>>().join(" → ");
    match h {
        FlowMethod::Johnson => {
            let desc = if m == 2 {
                "Regla de Johnson (óptima con dos máquinas): se toma el menor tiempo de todos; si es de la máquina 1, el trabajo va lo antes posible; si es de la máquina 2, lo más tarde posible. Se retira y se repite.".to_string()
            } else {
                format!("Con {m} máquinas se forman dos máquinas ficticias (sumando la primera y la segunda mitad de los tiempos) y se aplica la regla de Johnson.")
            };
            vec![step("Regla de Johnson", desc, vec![]), step("Secuencia", seq, vec![])]
        }
        FlowMethod::Cds => {
            let mut t = Table::new(Some("Subproblemas de CDS"), &["k", "Secuencia de Johnson", "Cmax"]).left(1);
            for k in 1..m.max(2) {
                let a: Vec<f64> = (0..n).map(|j| (0..k.min(m)).map(|q| p(j, q)).sum()).collect();
                let b: Vec<f64> = (0..n).map(|j| (m.saturating_sub(k)..m).map(|q| p(j, q)).sum()).collect();
                let o = johnson(&a, &b);
                t.row(cells![k, o.iter().map(|&j| d.names[j].as_str()).collect::<Vec<_>>().join(" → "), tidy(makespan(d, &o))]);
            }
            vec![step("CDS", "Para k = 1 … m − 1 se suman los tiempos de las primeras k máquinas y de las últimas k, se aplica Johnson a esas dos máquinas ficticias y se queda la secuencia con menor Cmax.", vec![t.block()]), step("Secuencia", seq, vec![])]
        }
        FlowMethod::Palmer => vec![step("Índice de pendiente de Palmer", "Sⱼ = Σₖ (2k − m − 1)·pⱼₖ: los trabajos cuyos tiempos crecen a lo largo de las máquinas van primero (orden decreciente de Sⱼ).", vec![]), step("Secuencia", seq, vec![])],
        FlowMethod::Gupta => vec![step("Índice de Gupta", "sⱼ = eⱼ / mínₖ(pⱼₖ + pⱼ,ₖ₊₁), con eⱼ = 1 si el primer tiempo es menor que el último y −1 si no; orden creciente de sⱼ.", vec![]), step("Secuencia", seq, vec![])],
        FlowMethod::Dannenbring => vec![step("Acceso rápido de Dannenbring", "Dos máquinas ficticias con pesos: aⱼ = Σₖ (m − k + 1)·pⱼₖ y bⱼ = Σₖ k·pⱼₖ; se aplica Johnson.", vec![]), step("Secuencia", seq, vec![])],
        FlowMethod::HundalRajgopal => vec![step("Hundal y Rajgopal", "Extiende a Palmer: calcula tres índices de pendiente (con pesos 2k − m − 1, 2k − m y 2k − m − 2) y se queda la secuencia con menor Cmax.", vec![]), step("Secuencia", seq, vec![])],
        FlowMethod::HoChang => vec![step("Mejora por intercambios", "Parte de la secuencia de CDS e intercambia pares de trabajos mientras baje el Cmax (en la línea de Ho y Chang, que reducen los huecos entre trabajos consecutivos).", vec![]), step("Secuencia", seq, vec![])],
        FlowMethod::Neh => vec![step("NEH", "Ordena los trabajos por tiempo total decreciente y los inserta uno a uno en la posición de la secuencia parcial que da menor Cmax.", vec![]), step("Secuencia", seq, vec![])],
        _ => vec![],
    }
}

// ------------------------------------------------------------------ archivos

fn write_job(m: &JobModel) -> Vec<u8> {
    let mut w = Writer::new();
    let n = m.jobs.len();
    let nm = m.machines.len();
    let ops: Vec<Vec<String>> = m
        .jobs
        .iter()
        .enumerate()
        .map(|(j, job)| match m.problem_type {
            JobType::FlowShop => (0..nm).map(|k| format!("{}/{}", num(m.times.get(j).and_then(|r| r.get(k)).copied().unwrap_or(0.0)), k + 1)).collect(),
            JobType::JobShop => job
                .operations
                .split([',', ';'])
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| {
                    // la máquina se guarda por número, como WinQSB
                    match s.split_once('/') {
                        Some((a, b)) => {
                            let b = b.trim();
                            let k = m.machines.iter().position(|x| x.name.eq_ignore_ascii_case(b)).map(|k| (k + 1).to_string()).unwrap_or_else(|| b.to_string());
                            format!("{}/{}", a.trim(), k)
                        }
                        None => s.to_string(),
                    }
                })
                .collect(),
        })
        .collect();
    let maxops = ops.iter().map(|o| o.len()).max().unwrap_or(0).max(1);
    let ty = if m.problem_type == JobType::FlowShop { "1" } else { "0" };
    w.row(&["SCH".to_string(), m.name.clone(), m.time_unit.clone(), n.to_string(), nm.to_string(), maxops.to_string(), ty.into()]);
    let mut h = vec!["Job Number".to_string(), "Job Name".to_string()];
    h.extend((1..=maxops).map(|k| format!("Operation {k}")));
    h.extend(["Ready Time", "Due Date", "Weight", "Priority Index", "Idle Cost", "Busy Cost", "Late Cost", "Early Cost"].map(String::from));
    w.row(&h);
    for (j, job) in m.jobs.iter().enumerate() {
        let mut r = vec![(j + 1).to_string(), job.name.clone()];
        r.extend((0..maxops).map(|k| ops[j].get(k).map(|s| format!("\"{s}\"")).unwrap_or_else(|| "\"\"".into())));
        r.extend([num(job.ready), job.due.map(num).unwrap_or_else(|| " ".into()), num(job.weight), num(job.priority), num(job.idle_cost), num(job.busy_cost), num(job.late_cost), num(job.early_cost)]);
        w.row(&r);
    }
    w.row(&["Machine Number", "Machine Name", "Ready Time", "Idle Cost", "Busy Cost"]);
    for (k, mc) in m.machines.iter().enumerate() {
        w.row(&[(k + 1).to_string(), mc.name.clone(), num(mc.ready), num(mc.idle_cost), num(mc.busy_cost)]);
    }
    w.bytes()
}

fn read_job(bytes: &[u8]) -> Result<JobModel, ModError> {
    let mut r = Reader::new(bytes);
    let h = r.line("la cabecera")?;
    if !h.get(0).trim().to_ascii_uppercase().starts_with("SCH") {
        return Err(ModError::Format("no es un archivo de programación de trabajos".into()));
    }
    let name = Some(h.get(1).trim()).filter(|s| !s.is_empty()).unwrap_or("Scheduling Problem").to_string();
    let unit = Some(h.get(2).trim()).filter(|s| !s.is_empty()).unwrap_or("minute").to_string();
    let n = parse_num(h.get(3)).unwrap_or(0.0) as usize;
    let nm = parse_num(h.get(4)).unwrap_or(0.0) as usize;
    let mut maxops = parse_num(h.get(5)).unwrap_or(0.0) as usize;
    if maxops < 1 {
        maxops = nm;
    }
    let flow = parse_num(h.get(6)).unwrap_or(0.0) as i64 != 0;
    if n == 0 || nm == 0 || n > 100_000 || nm > 10_000 {
        return Err(ModError::Format("el número de trabajos o de máquinas no es válido".into()));
    }
    r.line("la fila de títulos de los trabajos")?;
    let mut jobs = vec![];
    let mut times = vec![];
    for j in 0..n {
        let l = r.line(&format!("el trabajo {}", j + 1))?;
        let nm_j = Some(l.get(1).trim()).filter(|s| !s.is_empty()).map(str::to_string).unwrap_or(format!("Job {}", j + 1));
        let ops: Vec<String> = (0..maxops).map(|k| l.get(2 + k).trim().trim_matches('"').trim().to_string()).filter(|s| !s.is_empty()).collect();
        let b = 2 + maxops;
        let g = |i: usize| parse_num(l.get(b + i));
        if flow {
            let mut row = vec![0.0; nm];
            for (k, s) in ops.iter().enumerate() {
                let (a, mach) = s.split_once('/').unwrap_or((s.as_str(), ""));
                let kk = mach.trim().parse::<usize>().ok().filter(|x| *x >= 1 && *x <= nm).map(|x| x - 1).unwrap_or(k.min(nm - 1));
                row[kk] = parse_num(a).unwrap_or(0.0);
            }
            times.push(row);
        }
        jobs.push(Job {
            name: nm_j,
            operations: if flow { String::new() } else { ops.join(", ") },
            ready: g(0).unwrap_or(0.0),
            due: g(1),
            weight: g(2).unwrap_or(1.0),
            priority: g(3).unwrap_or(0.0),
            idle_cost: g(4).unwrap_or(0.0),
            busy_cost: g(5).unwrap_or(0.0),
            late_cost: g(6).unwrap_or(0.0),
            early_cost: g(7).unwrap_or(0.0),
        });
    }
    r.line("la fila de títulos de las máquinas")?;
    let mut machines = vec![];
    for k in 0..nm {
        let l = r.line(&format!("la máquina {}", k + 1))?;
        machines.push(Machine {
            name: Some(l.get(1).trim()).filter(|s| !s.is_empty()).map(str::to_string).unwrap_or(format!("Machine {}", k + 1)),
            ready: parse_num(l.get(2)).unwrap_or(0.0),
            idle_cost: parse_num(l.get(3)).unwrap_or(0.0),
            busy_cost: parse_num(l.get(4)).unwrap_or(0.0),
        });
    }
    Ok(JobModel { name, problem_type: if flow { JobType::FlowShop } else { JobType::JobShop }, time_unit: unit, jobs, machines, times })
}
