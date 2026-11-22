//! QSS · Simulación de sistemas de colas (equivalente al módulo QSS de WinQSB).
//!
//! El sistema se arma con cuatro tipos de componentes, como en WinQSB:
//! poblaciones de clientes (C), servidores (S), colas (Q) y recolectores de
//! desechos (G). Cada componente indica sus seguidores con el formato
//! «Nombre/Probabilidad/TiempoDeTraslado», separados por comas; los servidores
//! sin seguidores son la salida del sistema.
//!
//! - 8 reglas de entrada (incluido el ensamble) y 8 de salida (incluido el
//!   desensamble), 10 disciplinas de cola, capacidades con bloqueo, lotes de
//!   llegada, atributos de prioridad y tiempos de servicio por tipo de cliente.
//! - Distribuciones en el formato de WinQSB: «Exp/0/1.2», «Normal/50/10»,
//!   «Discrete/3/100/.3/200/.5/300/.2»; servicio «Cliente/Distribución/...».
//!
//! Formato de archivo comprobado con los ejemplos originales QSS1–QSS3 y QSSGRAPH.

pub mod engine;

use crate::pert::dist::{Dist, Law};
use engine::{Link, Node};
use iaqsb_core::linear::Bnd;
use iaqsb_core::winqsb::{num, parse_num, Reader, Writer};
use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Población de clientes (C).
    #[default]
    Customer,
    /// Servidor (S).
    Server,
    /// Cola (Q).
    Queue,
    /// Recolector de desechos (G).
    Garbage,
}

impl Kind {
    fn letter(self) -> &'static str {
        match self {
            Kind::Customer => "C",
            Kind::Server => "S",
            Kind::Queue => "Q",
            Kind::Garbage => "G",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum InRule {
    #[default]
    Random,
    Probability,
    RoundRobin,
    Assembly,
    LongestQueue,
    ShortestQueue,
    MaxQueueCapacity,
    MinQueueCapacity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum OutRule {
    #[default]
    Random,
    Probability,
    RoundRobin,
    Disassembly,
    LongestQueue,
    ShortestQueue,
    MaxQueueCapacity,
    MinQueueCapacity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Discipline {
    #[default]
    Fifo,
    Lifo,
    Random,
    PriorityIndex,
    Spt,
    Lpt,
    MaxWorkDone,
    MinWorkDone,
    MostWait,
    EarliestEntering,
}

const IN_RULES: [(InRule, &str, &str, &str); 8] = [
    (InRule::Random, "random", "Random", "Al azar"),
    (InRule::Probability, "probability", "Probability", "Por probabilidad"),
    (InRule::RoundRobin, "round_robin", "RoundRobin", "Por turnos"),
    (InRule::Assembly, "assembly", "Assembly", "Ensamble (uno de cada cola)"),
    (InRule::LongestQueue, "longest_queue", "LongestQueue", "Cola más larga"),
    (InRule::ShortestQueue, "shortest_queue", "ShortestQueue", "Cola más corta"),
    (InRule::MaxQueueCapacity, "max_queue_capacity", "MaxQueueCapacity", "Cola de mayor capacidad"),
    (InRule::MinQueueCapacity, "min_queue_capacity", "MinQueueCapacity", "Cola de menor capacidad"),
];
const OUT_RULES: [(OutRule, &str, &str, &str); 8] = [
    (OutRule::Random, "random", "Random", "Al azar"),
    (OutRule::Probability, "probability", "Probability", "Por probabilidad"),
    (OutRule::RoundRobin, "round_robin", "RoundRobin", "Por turnos"),
    (OutRule::Disassembly, "disassembly", "DisAssembly", "Desensamble (a todas)"),
    (OutRule::LongestQueue, "longest_queue", "LongestQueue", "Cola más larga"),
    (OutRule::ShortestQueue, "shortest_queue", "ShortestQueue", "Cola más corta"),
    (OutRule::MaxQueueCapacity, "max_queue_capacity", "MaxQueueCapacity", "Cola de mayor capacidad"),
    (OutRule::MinQueueCapacity, "min_queue_capacity", "MinQueueCapacity", "Cola de menor capacidad"),
];
const DISCIPLINES: [(Discipline, &str, &str, &str); 10] = [
    (Discipline::Fifo, "fifo", "FIFO", "Primero en llegar (FIFO)"),
    (Discipline::Lifo, "lifo", "LIFO", "Último en llegar (LIFO)"),
    (Discipline::Random, "random", "Random", "Al azar"),
    (Discipline::PriorityIndex, "priority_index", "PriorityIndex", "Mayor atributo primero"),
    (Discipline::Spt, "spt", "SPT", "Menor tiempo de proceso (SPT)"),
    (Discipline::Lpt, "lpt", "LPT", "Mayor tiempo de proceso (LPT)"),
    (Discipline::MaxWorkDone, "max_work_done", "MaxWorkDone", "Más trabajo hecho"),
    (Discipline::MinWorkDone, "min_work_done", "MinWorkDone", "Menos trabajo hecho"),
    (Discipline::MostWait, "most_wait", "MostWait", "Más tiempo esperado"),
    (Discipline::EarliestEntering, "earliest_entering", "EarliestEntering", "Entró antes al sistema"),
];

/// Reconoce una regla por sus tres primeras letras (como WinQSB).
fn by_prefix<T: Copy>(s: &str, table: &[(T, &str, &str, &str)]) -> Option<T> {
    let t: String = s.trim().to_ascii_uppercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if t.is_empty() {
        return None;
    }
    if let Some(x) = table.iter().find(|x| x.2.eq_ignore_ascii_case(&t) || x.1.replace('_', "").eq_ignore_ascii_case(&t)) {
        return Some(x.0);
    }
    let p: String = t.chars().take(3).collect();
    // «MaxQueueCapacity» y «MinQueueCapacity» comparten prefijo con otras: se miran más letras
    let cands: Vec<&(T, &str, &str, &str)> = table.iter().filter(|x| x.2.to_ascii_uppercase().starts_with(&p)).collect();
    if cands.len() == 1 {
        return Some(cands[0].0);
    }
    cands.iter().find(|x| x.2.to_ascii_uppercase().starts_with(&t)).map(|x| x.0).or(cands.first().map(|x| x.0))
}

fn fifty() -> Option<Bnd> {
    None
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Component {
    pub name: String,
    pub kind: Kind,
    /// Seguidores: «Nombre/Probabilidad/TiempoDeTraslado», separados por comas.
    #[serde(default)]
    pub followers: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_rule: Option<InRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_rule: Option<OutRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discipline: Option<Discipline>,
    /// Capacidad de la cola (vacía = 50, como WinQSB; «M» = ilimitada).
    #[serde(default = "fifty", skip_serializing_if = "Option::is_none")]
    pub capacity: Option<Bnd>,
    /// Atributo de los clientes de la población (para la disciplina por prioridad).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribute: Option<f64>,
    /// Población: tiempo entre llegadas, p. ej. «Exp/0/1.2».
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub interarrival: String,
    /// Población: tamaño del lote (vacío = «Constant/1»).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub batch: String,
    /// Servidor: tiempo de servicio, «Distribución/...» o «Cliente/Distribución/...» separados por comas.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub service: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct QssModel {
    #[serde(default)]
    pub name: String,
    #[serde(default = "minute")]
    pub time_unit: String,
    pub components: Vec<Component>,
    /// Ubicación de cada componente en el modelo gráfico (fila, columna), si se dio.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub positions: Vec<[i64; 2]>,
    /// El archivo se guardó en forma de modelo gráfico.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub graphic: bool,
}

fn minute() -> String {
    "minuto".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct QssOptions {
    pub steps: bool,
    pub simulation_time: f64,
    pub warmup: f64,
    /// Máximo de observaciones (clientes que terminan); 0 = sin límite.
    pub max_observations: u64,
    pub seed: u64,
    /// Eventos que se muestran en el paso a paso.
    pub trace: u32,
}

impl Default for QssOptions {
    fn default() -> Self {
        QssOptions { steps: true, simulation_time: 1000.0, warmup: 0.0, max_observations: 0, seed: 27437, trace: 30 }
    }
}

// ------------------------------------------------------- textos de WinQSB

fn dist_name(s: &str) -> Option<Dist> {
    let t = s.trim();
    if t.is_empty() || t.chars().next().map(|c| !c.is_ascii_alphabetic()).unwrap_or(true) {
        return None;
    }
    Dist::parse(t).filter(|d| !matches!(d, Dist::ThreeTime | Dist::Mixed | Dist::Arbitrary))
}

/// «Exp/0/1.2», «Normal/50/10», «Discrete/3/100/.3/200/.5/300/.2».
pub fn parse_dist(tokens: &[&str], what: &str) -> Result<Law, ModError> {
    let d = tokens.first().and_then(|t| dist_name(t)).ok_or_else(|| ModError::Invalid(format!("{what}: «{}» no es una distribución (p. ej. Exp/0/2, Normal/50/10, Constant/5)", tokens.join("/"))))?;
    if d == Dist::Discrete {
        let rest: Vec<&str> = tokens[1..].iter().map(|s| s.trim()).collect();
        // con o sin la cantidad de valores al principio
        let vals = if rest.len() % 2 == 1 { &rest[1..] } else { &rest[..] };
        let pairs: Vec<String> = vals.chunks(2).filter(|c| c.len() == 2).map(|c| format!("{}/{}", c[0], c[1])).collect();
        return Law::new(d, [None, None, None], Some(&pairs.join(", ")), what);
    }
    let p = |k: usize| tokens.get(k).and_then(|s| parse_num(s));
    let mut ps = [p(1), p(2), p(3)];
    if ps[0].is_none() && matches!(d, Dist::Exponential | Dist::Erlang | Dist::Gamma | Dist::Weibull | Dist::Laplace | Dist::PowerFunction) {
        ps[0] = Some(0.0);
    }
    Law::new(d, ps, None, what)
}

/// Seguidores «Nombre/Prob/Traslado, ...».
fn parse_followers(s: &str) -> Vec<(String, Option<f64>, f64)> {
    s.split(',')
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(|item| {
            let t: Vec<&str> = item.split('/').collect();
            (t[0].trim().to_string(), t.get(1).and_then(|x| parse_num(x)), t.get(2).and_then(|x| parse_num(x)).unwrap_or(0.0))
        })
        .collect()
}

/// Tiempos de servicio «Cliente/Dist/...» o «Dist/...», separados por comas.
fn parse_service(s: &str, what: &str) -> Result<Vec<(Option<String>, Law)>, ModError> {
    let mut out = vec![];
    for item in s.split(',').map(str::trim).filter(|x| !x.is_empty()) {
        let t: Vec<&str> = item.split('/').collect();
        if t.len() >= 2 && dist_name(t[1]).is_some() && !t[0].trim().is_empty() {
            out.push((Some(t[0].trim().to_string()), parse_dist(&t[1..], &format!("{what} ({})", t[0].trim()))?));
        } else {
            out.push((None, parse_dist(&t, what)?));
        }
    }
    Ok(out)
}

/// Valida el modelo y arma la red para el motor.
pub fn build(m: &QssModel) -> Result<Vec<Node>, ModError> {
    let n = m.components.len();
    if n == 0 {
        return invalid("el sistema no tiene componentes");
    }
    for (i, c) in m.components.iter().enumerate() {
        if c.name.trim().is_empty() {
            return invalid(format!("el componente {} no tiene nombre", i + 1));
        }
        if let Some(j) = (0..i).find(|&j| m.components[j].name.trim().eq_ignore_ascii_case(c.name.trim())) {
            return invalid(format!("el nombre «{}» se repite (componentes {} y {})", c.name, j + 1, i + 1));
        }
    }
    let find = |s: &str| (0..n).find(|&j| m.components[j].name.trim().eq_ignore_ascii_case(s.trim()));
    let mut nodes = vec![];
    for c in &m.components {
        let mut follow = vec![];
        for (name, p, t) in parse_followers(&c.followers) {
            let j = find(&name).ok_or_else(|| ModError::Invalid(format!("{}: el seguidor «{name}» no existe", c.name)))?;
            if m.components[j].name.trim().eq_ignore_ascii_case(c.name.trim()) {
                return invalid(format!("{}: un componente no puede seguirse a sí mismo (usa una cola intermedia)", c.name));
            }
            if let Some(p) = p {
                if !(0.0..=1.0).contains(&p) {
                    return invalid(format!("{}: la probabilidad hacia {name} debe estar entre 0 y 1", c.name));
                }
            }
            if !(t >= 0.0) {
                return invalid(format!("{}: el tiempo de traslado hacia {name} no puede ser negativo", c.name));
            }
            follow.push(Link { to: j, prob: p, transfer: t });
        }
        // probabilidades: las que falten se reparten lo que sobra
        if follow.iter().any(|l| l.prob.is_some()) {
            let given: f64 = follow.iter().filter_map(|l| l.prob).sum();
            let missing = follow.iter().filter(|l| l.prob.is_none()).count();
            if missing == 0 && (given - 1.0).abs() > 1e-6 && c.output_rule.map(|r| r == OutRule::Probability).unwrap_or(true) && c.kind != Kind::Queue {
                return invalid(format!("{}: las probabilidades de sus seguidores suman {}, no 1", c.name, fmt(tidy(given))));
            }
            if missing > 0 {
                let share = ((1.0 - given) / missing as f64).max(0.0);
                for l in follow.iter_mut() {
                    if l.prob.is_none() {
                        l.prob = Some(share);
                    }
                }
            }
        }
        let has_p = follow.iter().any(|l| l.prob.is_some());
        let what = |x: &str| format!("{}: {x}", c.name);
        let (ia, batch, service) = match c.kind {
            Kind::Customer => {
                if c.interarrival.trim().is_empty() {
                    return invalid(format!("{}: indica el tiempo entre llegadas (p. ej. Exp/0/2)", c.name));
                }
                let t: Vec<&str> = c.interarrival.split('/').collect();
                let ia = parse_dist(&t, &what("tiempo entre llegadas"))?;
                if !(ia.mean() > 0.0) {
                    return invalid(format!("{}: el tiempo medio entre llegadas debe ser positivo", c.name));
                }
                let b = if c.batch.trim().is_empty() { None } else { Some(parse_dist(&c.batch.split('/').collect::<Vec<_>>(), &what("tamaño del lote"))?) };
                if follow.is_empty() {
                    return invalid(format!("{}: una población debe conectarse a una o más colas", c.name));
                }
                (Some(ia), b, vec![])
            }
            Kind::Server => {
                if c.service.trim().is_empty() {
                    return invalid(format!("{}: indica el tiempo de servicio (p. ej. Exp/0/1.5 o Cliente/Normal/5/1)", c.name));
                }
                (None, None, parse_service(&c.service, &what("tiempo de servicio"))?)
            }
            Kind::Queue => {
                if follow.is_empty() {
                    return invalid(format!("{}: una cola debe conectarse a uno o más servidores", c.name));
                }
                (None, None, vec![])
            }
            Kind::Garbage => {
                if !follow.is_empty() {
                    return invalid(format!("{}: un recolector de desechos no tiene seguidores", c.name));
                }
                (None, None, vec![])
            }
        };
        let cap = match c.kind {
            Kind::Queue => match c.capacity {
                None => Some(50),
                Some(b) if !b.0.is_finite() => None,
                Some(b) => {
                    if b.0 < 0.0 || b.0.fract() != 0.0 {
                        return invalid(format!("{}: la capacidad debe ser un entero no negativo o M", c.name));
                    }
                    Some(b.0 as usize)
                }
            },
            _ => None,
        };
        nodes.push(Node {
            name: c.name.trim().to_string(),
            kind: c.kind,
            follow,
            preds: vec![],
            in_rule: c.input_rule.unwrap_or_default(),
            out_rule: c.output_rule.unwrap_or(if has_p { OutRule::Probability } else { OutRule::Random }),
            disc: c.discipline.unwrap_or_default(),
            cap,
            attr: c.attribute.unwrap_or(0.0),
            ia,
            batch,
            service,
        });
    }
    for i in 0..n {
        for k in 0..nodes[i].follow.len() {
            let j = nodes[i].follow[k].to;
            if !nodes[j].preds.contains(&i) {
                nodes[j].preds.push(i);
            }
        }
    }
    if !nodes.iter().any(|x| x.kind == Kind::Customer) {
        return invalid("hace falta al menos una población de clientes (tipo C)");
    }
    for nd in &nodes {
        if nd.kind == Kind::Server && nd.in_rule == InRule::Assembly && nd.preds.iter().filter(|&&p| nodes[p].kind == Kind::Queue).count() < 2 {
            return invalid(format!("{}: el ensamble necesita dos o más colas antes del servidor", nd.name));
        }
    }
    Ok(nodes)
}

// ------------------------------------------------------------------ módulo

pub struct Qss;

fn dist_help() -> String {
    crate::pert::dist::ALL
        .iter()
        .filter(|d| !matches!(d, Dist::ThreeTime | Dist::Mixed | Dist::Arbitrary | Dist::Discrete))
        .map(|d| format!("{}: {}", d.winqsb(), d.params().join(", ")))
        .collect::<Vec<_>>()
        .join(" · ")
}

impl TypedModule for Qss {
    type Model = QssModel;
    type Options = QssOptions;

    fn file_tag(&self) -> &'static str {
        "QSS"
    }

    fn spec(&self) -> ModuleSpec {
        let ins: Vec<(&str, &str)> = IN_RULES.iter().map(|x| (x.1, x.3)).collect();
        let outs: Vec<(&str, &str)> = OUT_RULES.iter().map(|x| (x.1, x.3)).collect();
        let discs: Vec<(&str, &str)> = DISCIPLINES.iter().map(|x| (x.1, x.3)).collect();
        let cols = vec![
            Field::text("name", "Componente"),
            Field::choice("kind", "Tipo", &[("customer", "C · Población de clientes"), ("server", "S · Servidor"), ("queue", "Q · Cola"), ("garbage", "G · Recolector de desechos")]),
            Field::text("followers", "Seguidores").help("Nombre/Probabilidad/Traslado, separados por comas; un servidor sin seguidores es la salida").show_if("kind", &["customer", "server", "queue"]),
            Field::choice("input_rule", "Regla de entrada", &ins).optional().show_if("kind", &["server"]).help("Cómo elige el servidor entre varias colas"),
            Field::choice("output_rule", "Regla de salida", &outs).optional().show_if("kind", &["customer", "server"]).help("Cómo elige entre varios seguidores (con probabilidades: por probabilidad)"),
            Field::choice("discipline", "Disciplina", &discs).optional().show_if("kind", &["queue"]),
            Field::bound("capacity", "Capacidad", Value::from(50)).optional().show_if("kind", &["queue"]).help("Vacía = 50; M = ilimitada; 0 = sin espera"),
            Field::number("attribute", "Atributo").optional().show_if("kind", &["customer"]).help("Prioridad para la disciplina «Mayor atributo primero»"),
            Field::text("interarrival", "Tiempo entre llegadas").show_if("kind", &["customer"]).help("Distribución/p1/p2/p3, p. ej. Exp/0/2"),
            Field::text("batch", "Tamaño del lote").show_if("kind", &["customer"]).help("Vacío = Constant/1"),
            Field::text("service", "Tiempo de servicio").show_if("kind", &["server"]).help("Distribución/... o Cliente/Distribución/..., separados por comas"),
        ];
        ModuleSpec {
            id: "qss".into(),
            winqsb: "QSS".into(),
            name: "Simulación de colas".into(),
            description: "Sistemas de colas de una o varias etapas con poblaciones, servidores, colas y recolectores, simulados por eventos discretos".into(),
            file_ext: "QSS".into(),
            problem_types: vec![ProblemType {
                id: "network".into(),
                label: "Sistema de colas".into(),
                description: format!("Distribuciones: {}. Discreta: Discrete/n/v1/p1/v2/p2…", dist_help()),
                dims: vec![Field::integer("components", "Número de componentes", 4)],
                layout: vec![Widget::params(None, vec![Field::text("time_unit", "Unidad de tiempo")]), Widget::records(Some("Componentes"), "components", cols, Some("Añadir componente"))],
            }],
            options: vec![
                Field::boolean("steps", "Mostrar paso a paso", true),
                Field::number("simulation_time", "Tiempo simulado").default(1000.0),
                Field::number("warmup", "Inicio de la recolección de datos"),
                Field::integer("max_observations", "Máximo de observaciones", 0).help("0 = sin límite (M)"),
                Field::integer("seed", "Semilla", 27437),
                Field::integer("trace", "Eventos en el paso a paso", 30),
            ],
        }
    }

    fn template(&self, _pt: &str, dims: &Value) -> Result<QssModel, ModError> {
        let n = dim(dims, "components", 4, 1, 500)?;
        let base: [(&str, Kind, &str, &str, &str); 4] = [("Clientes", Kind::Customer, "Cola", "Exp/0/5", ""), ("Cola", Kind::Queue, "Servidor", "", ""), ("Servidor", Kind::Server, "", "", "Exp/0/4"), ("Desechos", Kind::Garbage, "", "", "")];
        let components = (0..n)
            .map(|i| {
                let (nm, k, nx, ia, sv) = base.get(i).cloned().unwrap_or(("", Kind::Queue, "", "", ""));
                Component {
                    name: if nm.is_empty() { format!("Componente{}", i + 1) } else { nm.to_string() },
                    kind: k,
                    followers: nx.into(),
                    input_rule: None,
                    output_rule: None,
                    discipline: None,
                    capacity: None,
                    attribute: None,
                    interarrival: ia.into(),
                    batch: String::new(),
                    service: sv.into(),
                }
            })
            .collect();
        Ok(QssModel { name: "Sistema de colas".into(), time_unit: "minuto".into(), components, positions: vec![], graphic: false })
    }

    fn solve(&self, m: &QssModel, o: &QssOptions) -> Result<Report, ModError> {
        solve(m, o)
    }

    fn read(&self, bytes: &[u8]) -> Result<QssModel, ModError> {
        read_qss(bytes)
    }

    fn write(&self, m: &QssModel) -> Result<Vec<u8>, ModError> {
        Ok(write_qss(m))
    }

    fn examples(&self) -> Vec<(&'static str, QssModel)> {
        let ex = |s: &str| serde_json::from_str::<QssModel>(s).unwrap();
        vec![
            ("banco_dos_cajeros", ex(include_str!("ejemplos/banco.json"))),
            ("taller_en_serie", ex(include_str!("ejemplos/taller.json"))),
            ("mm1_comprobacion", ex(include_str!("ejemplos/mm1.json"))),
            ("ensamble_dos_lineas", ex(include_str!("ejemplos/ensamble.json"))),
        ]
    }
}

fn solve(m: &QssModel, o: &QssOptions) -> Result<Report, ModError> {
    let nodes = build(m)?;
    if !(o.simulation_time > 0.0) {
        return invalid("indica un tiempo de simulación positivo");
    }
    if o.warmup >= o.simulation_time {
        return invalid("el inicio de la recolección debe ser menor que el tiempo simulado");
    }
    let so = engine::simulate(&nodes, o.simulation_time, o.warmup, (o.max_observations > 0).then_some(o.max_observations), o.seed, if o.steps { o.trace as usize } else { 0 });
    let span = (so.end - o.warmup).max(1e-12);
    let u = &m.time_unit;
    let n = nodes.len();
    let avg = |s: &engine::Stat| if s.n_t > 0 { s.sum_t / s.n_t as f64 } else { 0.0 };
    // clientes
    let mut ct = Table::new(
        Some("Análisis de clientes"),
        &["Población", "Llegaron", "No entraron", "Terminaron", "A desechos", &format!("Tiempo medio en el sistema ({u})"), &format!("Mínimo ({u})"), &format!("Máximo ({u})"), &format!("Espera media ({u})"), &format!("Proceso medio ({u})"), "En el sistema (promedio)"],
    )
    .left(0);
    let mut tot = engine::CustStat::default();
    let mut data_c = vec![];
    let mut first = true;
    for i in 0..n {
        if nodes[i].kind != Kind::Customer {
            continue;
        }
        let c = &so.cust[i];
        let a = |x: f64| if c.finished > 0 { tidy(x / c.finished as f64) } else { 0.0 };
        ct.row(cells![&nodes[i].name, c.arrived as f64, c.balked as f64, c.finished as f64, c.garbage as f64, a(c.sum_sys), tidy(c.min_sys), tidy(c.max_sys), a(c.sum_wait), a(c.sum_work), tidy(c.in_sys_area / span)]);
        data_c.push(json!({"population": nodes[i].name, "arrived": c.arrived, "balked": c.balked, "finished": c.finished, "garbage": c.garbage, "time_in_system": a(c.sum_sys), "wait": a(c.sum_wait), "in_system": tidy(c.in_sys_area / span)}));
        tot.arrived += c.arrived;
        tot.balked += c.balked;
        tot.garbage += c.garbage;
        tot.sum_sys += c.sum_sys;
        tot.sum_wait += c.sum_wait;
        tot.sum_work += c.sum_work;
        tot.in_sys_area += c.in_sys_area;
        tot.max_sys = tot.max_sys.max(c.max_sys);
        if c.finished > 0 {
            tot.min_sys = if first { c.min_sys } else { tot.min_sys.min(c.min_sys) };
            first = false;
        }
        tot.finished += c.finished;
    }
    let pops = nodes.iter().filter(|x| x.kind == Kind::Customer).count();
    let ta = |x: f64| if tot.finished > 0 { tidy(x / tot.finished as f64) } else { 0.0 };
    if pops > 1 {
        ct.row_tone(cells!["Total", tot.arrived as f64, tot.balked as f64, tot.finished as f64, tot.garbage as f64, ta(tot.sum_sys), tidy(tot.min_sys), tidy(tot.max_sys), ta(tot.sum_wait), ta(tot.sum_work), tidy(tot.in_sys_area / span)], Tone::Total);
    }
    // servidores, colas y recolectores
    let mut svt = Table::new(Some("Análisis de servidores"), &["Servidor", "Atendidos", "Utilización (%)", "Bloqueado (%)", "Ocioso (%)", &format!("Servicio medio ({u})"), &format!("Servicio máximo ({u})")]).left(0);
    let mut qt = Table::new(Some("Análisis de colas"), &["Cola", "Capacidad", "Entraron", "Longitud media", "Longitud máxima", &format!("Espera media ({u})"), &format!("Espera máxima ({u})")]).left(0);
    let mut gt = Table::new(Some("Análisis de recolectores de desechos"), &["Recolector", "Recolectados", &format!("Tiempo medio en el sistema ({u})"), &format!("Máximo ({u})")]).left(0);
    let mut util_pts = vec![];
    let mut cats = vec![];
    let mut data_el = vec![];
    for i in 0..n {
        let s = &so.stats[i];
        match nodes[i].kind {
            Kind::Server => {
                let ut = s.area / span;
                let bl = s.blocked_area / span;
                svt.row(cells![&nodes[i].name, s.count as f64, tidy(100.0 * ut), tidy(100.0 * bl), tidy(100.0 * (1.0 - ut - bl).max(0.0)), tidy(avg(s)), tidy(s.max_t)]);
                cats.push(nodes[i].name.clone());
                util_pts.push([util_pts.len() as f64, 100.0 * ut]);
                data_el.push(json!({"name": nodes[i].name, "kind": "server", "served": s.count, "utilization": tidy(ut), "blocked": tidy(bl), "service_time": tidy(avg(s))}));
            }
            Kind::Queue => {
                qt.row(cells![&nodes[i].name, nodes[i].cap.map(|c| c.to_string()).unwrap_or("M".into()), s.count as f64, tidy(s.area / span), s.max_level, tidy(avg(s)), tidy(s.max_t)]);
                data_el.push(json!({"name": nodes[i].name, "kind": "queue", "entered": s.count, "average_length": tidy(s.area / span), "max_length": s.max_level, "average_wait": tidy(avg(s))}));
            }
            Kind::Garbage => {
                gt.row(cells![&nodes[i].name, s.count as f64, tidy(avg(s)), tidy(s.max_t)]);
                data_el.push(json!({"name": nodes[i].name, "kind": "garbage", "collected": s.count, "time_in_system": tidy(avg(s))}));
            }
            Kind::Customer => {}
        }
    }
    let wsys = ta(tot.sum_sys);
    let lsys = tot.in_sys_area / span;
    let mut sol = vec![
        metrics(vec![
            metric_main(format!("Tiempo medio en el sistema ({u})"), wsys),
            metric("Clientes en el sistema (promedio)", tidy(lsys)),
            metric("Clientes que terminaron", tot.finished as f64),
            metric(format!("Producción por {u}"), tidy(tot.finished as f64 / span)),
        ]),
        text(format!(
            "Simulación de {} {u} (datos desde {}){}, semilla {}.",
            fmt(tidy(so.end)),
            fmt(o.warmup),
            if so.end < o.simulation_time { format!("; se detuvo al reunir {} observaciones", o.max_observations) } else { String::new() },
            o.seed
        )),
        ct.block(),
    ];
    if !svt.rows.is_empty() {
        sol.push(svt.block());
    }
    if !qt.rows.is_empty() {
        sol.push(qt.block());
    }
    if !gt.rows.is_empty() {
        sol.push(gt.block());
    }
    // diagrama
    let pos: Option<Vec<(f64, f64)>> = (m.positions.len() == n).then(|| m.positions.iter().map(|p| (p[1] as f64, p[0] as f64)).collect());
    let coords: Vec<(f64, f64)> = match pos {
        Some(p) => {
            let (x0, x1) = p.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, q| (a.0.min(q.0), a.1.max(q.0)));
            let (y0, y1) = p.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, q| (a.0.min(q.1), a.1.max(q.1)));
            p.iter().map(|q| (if x1 > x0 { (q.0 - x0) / (x1 - x0) } else { 0.5 }, if y1 > y0 { 0.1 + 0.8 * (q.1 - y0) / (y1 - y0) } else { 0.5 })).collect()
        }
        None => layered(&nodes),
    };
    let mut nn = vec![];
    for i in 0..n {
        let s = &so.stats[i];
        let note = match nodes[i].kind {
            Kind::Server => format!("S · {} %", fmt(tidy(100.0 * s.area / span))),
            Kind::Queue => format!("Q · Lq {}", fmt(tidy(s.area / span))),
            Kind::Customer => format!("C · {} llegadas", s.count),
            Kind::Garbage => format!("G · {}", s.count),
        };
        nn.push(NetNode {
            id: format!("e{i}"),
            label: nodes[i].name.clone(),
            x: coords[i].0,
            y: coords[i].1,
            note: Some(note),
            tone: match nodes[i].kind {
                Kind::Server => Tone::Accent,
                Kind::Queue => Tone::Highlight,
                Kind::Garbage => Tone::Bad,
                Kind::Customer => Tone::Normal,
            },
        });
    }
    let mut edges = vec![];
    for i in 0..n {
        for l in &nodes[i].follow {
            let mut lab = vec![];
            if let Some(p) = l.prob {
                if nodes[i].follow.len() > 1 {
                    lab.push(fmt(tidy(p)));
                }
            }
            if l.transfer > 0.0 {
                lab.push(format!("⏱ {}", fmt(l.transfer)));
            }
            edges.push(NetEdge { from: format!("e{i}"), to: format!("e{}", l.to), label: (!lab.is_empty()).then(|| lab.join(" · ")), tone: Tone::Normal });
        }
    }
    let mut graphs = vec![Block::Network(Network { title: Some("Sistema simulado".into()), directed: true, nodes: nn, edges })];
    if !util_pts.is_empty() {
        let mut ch = Chart::new(Some("Utilización de los servidores"), "Servidor", "Utilización (%)").series(Series::new("Utilización", SeriesKind::Bar, util_pts).tone(Tone::Accent));
        ch.categories = Some(cats);
        ch.y_zero = true;
        graphs.push(ch.block());
    }
    let mut st = vec![];
    if o.steps {
        let mut tt = Table::new(Some("Primeros eventos"), &[&format!("Tiempo ({u})"), "Evento", "Componente", "Cliente"]).left(1).left(2).left(3);
        for (t, w, e, c) in &so.trace {
            tt.row(cells![tidy(*t), w, e, c]);
        }
        st.push(steps(vec![
            step(
                "Eventos discretos",
                "La simulación salta de un evento al siguiente: llega un lote de una población, termina un servicio o termina un traslado. Cada evento cambia el estado (colas y servidores) y programa eventos futuros con tiempos tomados de sus distribuciones.",
                vec![],
            ),
            step(
                "Recorrido de los clientes",
                "Una población manda cada cliente a una de sus colas según la regla de salida; si todas están llenas, el cliente no entra. Un servidor libre toma un cliente de sus colas según la regla de entrada (con ensamble, uno de cada cola) y la disciplina de la cola. Al terminar, lo pasa a un seguidor con lugar; si no hay, queda bloqueado. Un servidor sin seguidores es la salida.",
                vec![tt.block()],
            ),
            step(
                "Estadísticas",
                "Las longitudes de cola y la utilización se ponderan por el tiempo; las esperas y los tiempos en el sistema se promedian por cliente. Solo cuentan los datos después del inicio de la recolección; para resultados más estables, simula más tiempo o compara varias semillas.",
                vec![],
            ),
        ]));
    }
    Ok(Report::new("qss", &m.name, Status::Solved, format!("{} clientes terminaron; tiempo medio en el sistema {} {u}.", tot.finished, fmt(wsys)))
        .section("solucion", "Solución", sol)
        .section("red", "Red", graphs)
        .section("pasos", "Paso a paso", st)
        .data(json!({"time_in_system": wsys, "in_system": tidy(lsys), "completed": tot.finished, "arrived": tot.arrived, "balked": tot.balked, "garbage": tot.garbage, "end_time": tidy(so.end), "customers": data_c, "elements": data_el})))
}

/// Disposición por capas desde las poblaciones.
fn layered(nodes: &[Node]) -> Vec<(f64, f64)> {
    let n = nodes.len();
    let mut depth = vec![usize::MAX; n];
    let mut q: VecDeque<usize> = (0..n).filter(|&i| nodes[i].kind == Kind::Customer).collect();
    for &i in &q {
        depth[i] = 0;
    }
    while let Some(i) = q.pop_front() {
        for l in &nodes[i].follow {
            if depth[l.to] == usize::MAX {
                depth[l.to] = depth[i] + 1;
                q.push_back(l.to);
            }
        }
    }
    let maxd = depth.iter().filter(|d| **d != usize::MAX).max().copied().unwrap_or(0);
    for d in depth.iter_mut() {
        if *d == usize::MAX {
            *d = maxd + 1;
        }
    }
    let maxd = *depth.iter().max().unwrap();
    let mut out = vec![(0.0, 0.0); n];
    for lvl in 0..=maxd {
        let at: Vec<usize> = (0..n).filter(|&i| depth[i] == lvl).collect();
        for (k, &i) in at.iter().enumerate() {
            out[i] = (if maxd == 0 { 0.5 } else { lvl as f64 / maxd as f64 }, (k as f64 + 1.0) / (at.len() as f64 + 1.0));
        }
    }
    out
}

// ------------------------------------------------------------------ archivos

fn read_qss(bytes: &[u8]) -> Result<QssModel, ModError> {
    let mut r = Reader::new(bytes);
    let h = r.line("la cabecera")?;
    if !h.get(0).trim().to_ascii_uppercase().starts_with("QSS") {
        return Err(ModError::Format("no es un archivo de QSS".into()));
    }
    let n = parse_num(h.get(3)).unwrap_or(0.0) as usize;
    let mut m = QssModel {
        name: Some(h.get(1).trim()).filter(|s| !s.is_empty()).unwrap_or("Queuing System").to_string(),
        time_unit: Some(h.get(2).trim()).filter(|s| !s.is_empty()).unwrap_or("minute").to_string(),
        components: vec![],
        positions: vec![],
        graphic: h.get(4).trim().to_ascii_uppercase().starts_with('G'),
    };
    r.line("la fila de títulos")?;
    let opt = |s: &str| Some(s.trim().to_string()).filter(|x| !x.is_empty());
    for i in 0..n.min(100_000) {
        let l = r.line(&format!("el componente {}", i + 1))?;
        let kind = match l.get(1).trim().chars().next().map(|c| c.to_ascii_uppercase()) {
            Some('C') => Kind::Customer,
            Some('S') => Kind::Server,
            Some('Q') => Kind::Queue,
            Some('G') => Kind::Garbage,
            _ => return Err(ModError::Format(format!("componente {}: tipo «{}» desconocido (C, S, Q o G)", i + 1, l.get(1).trim()))),
        };
        let cap = l.get(6).trim();
        m.components.push(Component {
            name: l.get(0).trim().to_string(),
            kind,
            followers: l.get(2).trim().to_string(),
            input_rule: opt(l.get(3)).and_then(|s| by_prefix(&s, &IN_RULES)),
            output_rule: opt(l.get(4)).and_then(|s| by_prefix(&s, &OUT_RULES)),
            discipline: opt(l.get(5)).and_then(|s| by_prefix(&s, &DISCIPLINES)),
            capacity: if cap.is_empty() { None } else if cap.eq_ignore_ascii_case("M") { Some(Bnd::INF) } else { parse_num(cap).map(Bnd) },
            attribute: parse_num(l.get(7)),
            interarrival: l.get(8).trim().to_string(),
            batch: l.get(9).trim().to_string(),
            service: l.get(10).trim().to_string(),
        });
    }
    // 0 = sin ubicaciones; 1 o -1 = siguen las ubicaciones del modelo gráfico
    if let Some(l) = r.try_line() {
        let flag = parse_num(l.get(0)).unwrap_or(0.0);
        if flag != 0.0 {
            let mut pos = vec![[0i64, 0i64]; m.components.len()];
            let mut got = 0;
            while let Some(l) = r.try_line() {
                let k = parse_num(l.get(0)).unwrap_or(0.0) as usize;
                if k >= 1 && k <= pos.len() {
                    pos[k - 1] = [parse_num(l.get(1)).unwrap_or(0.0) as i64, parse_num(l.get(2)).unwrap_or(0.0) as i64];
                    got += 1;
                }
            }
            if got > 0 {
                m.positions = pos;
            }
        }
    }
    Ok(m)
}

fn write_qss(m: &QssModel) -> Vec<u8> {
    let mut w = Writer::new();
    let graphic = m.graphic && m.positions.len() == m.components.len();
    w.row(&["QSS".to_string(), m.name.clone(), m.time_unit.clone(), m.components.len().to_string(), if graphic { "Graphic" } else { "Matrix" }.into()]);
    w.row(&[
        "Component Name",
        "Type (C/S/Q/G)",
        "Immediate Follower (Name / Prob / TransferTime, separated by ',')",
        "Input Rule",
        "Output Rule",
        "Queue Discipline",
        "Queue Capacity",
        "Attribute Value",
        "Interarrival Time Distribution",
        "Batch Size Distribution",
        "Service Time Distribution",
    ]);
    let b = |s: &str| if s.trim().is_empty() { " ".to_string() } else { s.to_string() };
    for c in &m.components {
        let k = c.kind;
        w.row(&[
            c.name.clone(),
            k.letter().into(),
            b(&c.followers),
            c.input_rule.map(|r| IN_RULES.iter().find(|x| x.0 == r).unwrap().2.to_string()).unwrap_or(" ".into()),
            c.output_rule.map(|r| OUT_RULES.iter().find(|x| x.0 == r).unwrap().2.to_string()).unwrap_or(" ".into()),
            c.discipline.map(|r| DISCIPLINES.iter().find(|x| x.0 == r).unwrap().2.to_string()).unwrap_or(" ".into()),
            c.capacity.map(|x| if x.0.is_finite() { num(x.0) } else { "M".into() }).unwrap_or(" ".into()),
            c.attribute.map(num).unwrap_or(" ".into()),
            b(&c.interarrival),
            if k == Kind::Customer && c.batch.trim().is_empty() { "Constant/1".into() } else { b(&c.batch) },
            b(&c.service),
        ]);
    }
    if m.positions.len() == m.components.len() && !m.positions.is_empty() {
        w.row(&["-1"]);
        for (i, p) in m.positions.iter().enumerate() {
            w.row(&[(i + 1).to_string(), p[0].to_string(), p[1].to_string()]);
        }
    } else {
        w.row(&["0"]);
    }
    w.bytes()
}
