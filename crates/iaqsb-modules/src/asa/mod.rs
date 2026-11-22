//! ASA · Muestreo de aceptación (equivalente al módulo ASA de WinQSB).
//!
//! Atributos: muestreo simple, doble, múltiple, secuencial (Wald), en cadena
//! (ChSP-1), continuo (CSP-1) y por saltos de lotes (SkSP-2), con distribución
//! binomial, de Poisson o hipergeométrica, errores de inspección y
//! rectificación. Variables: media de la muestra, fracción defectuosa por los
//! métodos k y M, y secuencial para variables. Curvas OC, AOQ, ATI, ASN y de
//! costo, riesgos α y β, AOQL, búsqueda del plan y proceso secuencial con datos.

mod attr;
mod var;

use iaqsb_core::winqsb::{num, parse_num, Reader, Writer};
use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    #[default]
    Single,
    Double,
    Multiple,
    Sequential,
    Chain,
    Continuous,
    SkipLot,
    Mean,
    KMethod,
    MMethod,
    SequentialVariable,
}

impl Plan {
    pub const ALL: [Plan; 11] = [Plan::Single, Plan::Double, Plan::Multiple, Plan::Sequential, Plan::Chain, Plan::Continuous, Plan::SkipLot, Plan::Mean, Plan::KMethod, Plan::MMethod, Plan::SequentialVariable];
    pub fn code(self) -> usize {
        Plan::ALL.iter().position(|p| *p == self).unwrap()
    }
    pub fn id(self) -> &'static str {
        match self {
            Plan::Single => "single",
            Plan::Double => "double",
            Plan::Multiple => "multiple",
            Plan::Sequential => "sequential",
            Plan::Chain => "chain",
            Plan::Continuous => "continuous",
            Plan::SkipLot => "skip_lot",
            Plan::Mean => "mean",
            Plan::KMethod => "k_method",
            Plan::MMethod => "m_method",
            Plan::SequentialVariable => "sequential_variable",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Plan::Single => "Muestreo simple (atributos)",
            Plan::Double => "Muestreo doble (atributos)",
            Plan::Multiple => "Muestreo múltiple (atributos)",
            Plan::Sequential => "Muestreo secuencial (atributos)",
            Plan::Chain => "Muestreo en cadena ChSP-1",
            Plan::Continuous => "Muestreo continuo CSP-1",
            Plan::SkipLot => "Muestreo por saltos de lotes SkSP-2",
            Plan::Mean => "Media de la muestra (variables)",
            Plan::KMethod => "Fracción defectuosa, método k (variables)",
            Plan::MMethod => "Fracción defectuosa, método M (variables)",
            Plan::SequentialVariable => "Muestreo secuencial (variables)",
        }
    }
    pub fn variable(self) -> bool {
        self.code() >= 7
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProbDist {
    #[default]
    Binomial,
    Hypergeometric,
    Poisson,
    Normal,
    /// «Program selection» de WinQSB (solo muestreo simple): elige según n/N, np y p.
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    #[default]
    Lower,
    Upper,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Stage {
    /// Tamaño de la muestra.
    pub n: f64,
    /// Número de aceptación.
    pub c: f64,
    /// Número de rechazo (en la última etapa, c + 1).
    #[serde(default)]
    pub r: Option<f64>,
}

fn five() -> f64 {
    5.0
}
fn ten() -> f64 {
    10.0
}
fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AsaModel {
    #[serde(default)]
    pub name: String,
    pub problem_type: Plan,
    // atributos
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stages: Vec<Stage>,
    /// Cadena: número de lotes anteriores (i); continuo y saltos: número de liberación (i).
    #[serde(default)]
    pub clearance: f64,
    /// Continuo y saltos: % de unidades o de lotes inspeccionados (f).
    #[serde(default)]
    pub fraction: f64,
    /// Calidad aceptable y rechazable (% defectuoso).
    #[serde(default)]
    pub aql: f64,
    #[serde(default)]
    pub rql: f64,
    #[serde(default = "five")]
    pub alpha: f64,
    #[serde(default = "ten")]
    pub beta: f64,
    #[serde(default)]
    pub lot_size: Option<f64>,
    #[serde(default)]
    pub distribution: ProbDist,
    #[serde(default)]
    pub sampling_cost: f64,
    #[serde(default)]
    pub inspection_cost: f64,
    #[serde(default)]
    pub producer_cost: f64,
    #[serde(default)]
    pub consumer_cost: f64,
    /// Errores de inspección (%): un bueno clasificado como malo / un malo como bueno.
    #[serde(default)]
    pub error_good: f64,
    #[serde(default)]
    pub error_bad: f64,
    /// Inspección rectificadora: los defectuosos encontrados se reemplazan por buenos
    /// (AOQ = Pa·p·(N − n)/N; si no, Pa·p·(N − n)/(N − n·p)).
    #[serde(default = "yes")]
    pub rectified: bool,
    /// Muestreo truncado: se deja de inspeccionar en cuanto se puede rechazar.
    #[serde(default)]
    pub curtailed: bool,
    // variables
    #[serde(default)]
    pub side: Side,
    #[serde(default)]
    pub sigma: f64,
    #[serde(default = "yes")]
    pub sigma_known: bool,
    #[serde(default)]
    pub n: f64,
    /// Media: límites de aceptación.
    #[serde(default)]
    pub xla: Option<f64>,
    #[serde(default)]
    pub xua: Option<f64>,
    /// Media aceptable (AQL) y rechazables (RQL) inferior y superior.
    #[serde(default)]
    pub x1: f64,
    #[serde(default)]
    pub x2l: Option<f64>,
    #[serde(default)]
    pub x2u: Option<f64>,
    #[serde(default)]
    pub lsl: Option<f64>,
    #[serde(default)]
    pub usl: Option<f64>,
    /// Método k: valor crítico; método M: máxima fracción defectuosa (%).
    #[serde(default)]
    pub k: f64,
    #[serde(default)]
    pub m: f64,
    /// Secuencial de variables: desplazamiento de los datos codificados.
    #[serde(default)]
    pub offset: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Analyze,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Criterion {
    /// Cumplir α y β (la menor muestra que los cumple).
    #[default]
    Risks,
    /// Menor desviación ponderada de α y β.
    Weighted,
    /// AOQL que no pase de un valor.
    Aoql,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AsaOptions {
    pub steps: bool,
    pub mode: Mode,
    /// Búsqueda: criterio, pesos de α y β, y AOQL máximo (%).
    pub criterion: Criterion,
    pub weight_alpha: f64,
    pub weight_beta: f64,
    pub aoql_limit: f64,
    /// Análisis «qué pasa si»: % defectuoso (atributos) o media (variables).
    pub what_if: Option<f64>,
    /// Rango de las curvas: % defectuoso (atributos, k, M) o media (media y secuencial de variables).
    pub from: Option<f64>,
    pub to: Option<f64>,
    pub points: u32,
    /// Proceso secuencial: datos observados (0/1 por unidad, o valores medidos).
    pub observations: String,
}

impl Default for AsaOptions {
    fn default() -> Self {
        AsaOptions { steps: true, mode: Mode::Analyze, criterion: Criterion::Risks, weight_alpha: 0.5, weight_beta: 0.5, aoql_limit: 2.0, what_if: None, from: None, to: None, points: 40, observations: String::new() }
    }
}

pub struct Asa;

impl TypedModule for Asa {
    type Model = AsaModel;
    type Options = AsaOptions;

    fn file_tag(&self) -> &'static str {
        "ASA"
    }

    fn spec(&self) -> ModuleSpec {
        let quality = || vec![Field::number("aql", "Calidad aceptable AQL (% defectuoso)"), Field::number("rql", "Calidad rechazable RQL o LTPD (% defectuoso)"), Field::number("alpha", "Riesgo del productor α (%)").default(5.0), Field::number("beta", "Riesgo del consumidor β (%)").default(10.0)];
        let lot = || {
            vec![
                Field::number("lot_size", "Tamaño del lote (N)").optional(),
                Field::choice("distribution", "Distribución", &[("binomial", "Binomial"), ("hypergeometric", "Hipergeométrica (usa N)"), ("poisson", "Poisson"), ("normal", "Normal (aproximación)"), ("auto", "Elegida por el programa (muestreo simple)")]).default("binomial"),
            ]
        };
        let costs = || {
            vec![
                Field::number("sampling_cost", "Costo por unidad muestreada"),
                Field::number("inspection_cost", "Costo por unidad inspeccionada (100 %)"),
                Field::number("producer_cost", "Costo del productor por defectuoso encontrado"),
                Field::number("consumer_cost", "Costo del consumidor por defectuoso que pasa"),
            ]
        };
        let errors = || {
            vec![
                Field::number("error_good", "Error: % de buenos clasificados como malos"),
                Field::number("error_bad", "Error: % de malos clasificados como buenos"),
                Field::boolean("rectified", "Inspección rectificadora (los defectuosos se reemplazan)", true),
            ]
        };
        let stages = |label: &str, rej: bool| {
            let mut cols = vec![Field::number("n", "Tamaño de muestra (n)"), Field::number("c", "Número de aceptación (c)")];
            if rej {
                cols.push(Field::number("r", "Número de rechazo (r)").optional().help("En la última etapa, vacío = c + 1"));
            }
            Widget::records(Some(label), "stages", cols, if rej { Some("Añadir etapa") } else { None })
        };
        let attr = |id: Plan, plan: Vec<Widget>, extra: Vec<Field>, with_dist: bool| {
            let mut q = quality();
            if id != Plan::Continuous {
                q.extend(lot().into_iter().filter(|f| with_dist || f.key != "distribution"));
            }
            q.extend(extra);
            let mut layout = plan;
            layout.push(Widget::params(Some("Calidad y riesgos"), q));
            layout.push(Widget::params(Some("Costos (opcionales)"), costs()));
            if id != Plan::Sequential {
                layout.push(Widget::params(Some("Errores de inspección"), errors()));
            }
            ProblemType { id: id.id().into(), label: id.label().into(), description: String::new(), dims: if id == Plan::Multiple { vec![Field::integer("stages", "Número de etapas", 3)] } else { vec![] }, layout }
        };
        let var_common = || vec![Field::choice("side", "Especificación", &[("lower", "Inferior"), ("upper", "Superior"), ("both", "Ambas")]).default("lower"), Field::number("sigma", "Desviación estándar (σ)"), Field::boolean("sigma_known", "σ conocida (si no, es la de la muestra)", true)];
        let mut types = vec![
            attr(Plan::Single, vec![stages("Plan", false)], vec![Field::boolean("curtailed", "Muestreo truncado (parar al poder rechazar)", false)], true),
            attr(Plan::Double, vec![stages("Plan (dos etapas)", true)], vec![Field::boolean("curtailed", "Muestreo truncado (parar al poder rechazar)", false)], true),
            attr(Plan::Multiple, vec![stages("Plan (etapas)", true)], vec![Field::boolean("curtailed", "Muestreo truncado (parar al poder rechazar)", false)], true),
            attr(Plan::Sequential, vec![], vec![], false),
            attr(Plan::Chain, vec![stages("Plan de muestreo", false), Widget::params(None, vec![Field::number("clearance", "Lotes anteriores (i)")])], vec![], true),
            attr(Plan::Continuous, vec![Widget::params(Some("Plan"), vec![Field::number("fraction", "% de unidades inspeccionadas (f)"), Field::number("clearance", "Número de liberación (i)")])], vec![], false),
            attr(Plan::SkipLot, vec![stages("Plan de referencia", false), Widget::params(None, vec![Field::number("fraction", "% de lotes inspeccionados (f)"), Field::number("clearance", "Número de liberación (i)")])], vec![], true),
        ];
        for t in types.iter_mut() {
            t.description = match t.id.as_str() {
                "single" => "Se toma una muestra de n y se acepta el lote si hay c defectuosos o menos",
                "double" => "Una primera muestra decide o pide una segunda",
                "multiple" => "Varias etapas, cada una con números de aceptación y rechazo",
                "sequential" => "Unidad por unidad hasta cruzar una de las rectas de Wald",
                "chain" => "Muestras pequeñas con c = 0 que miran los lotes anteriores",
                "continuous" => "Producción continua: 100 % hasta i unidades buenas, después una fracción f",
                _ => "Se inspecciona solo una fracción de los lotes después de i lotes aceptados",
            }
            .into();
        }
        let mut mean = var_common();
        mean.extend([
            Field::number("n", "Tamaño de muestra (n)"),
            Field::number("xla", "Límite de aceptación inferior (XLa)").optional().show_if("side", &["lower", "both"]),
            Field::number("xua", "Límite de aceptación superior (XUa)").optional().show_if("side", &["upper", "both"]),
            Field::number("x1", "Media aceptable (AQL)"),
            Field::number("x2l", "Media rechazable inferior (RQL)").optional().show_if("side", &["lower", "both"]),
            Field::number("x2u", "Media rechazable superior (RQL)").optional().show_if("side", &["upper", "both"]),
            Field::number("alpha", "Riesgo del productor α (%)").default(5.0),
            Field::number("beta", "Riesgo del consumidor β (%)").default(10.0),
        ]);
        let km = |m_method: bool| {
            let mut v = var_common();
            v.extend([
                Field::number("n", "Tamaño de muestra (n)"),
                if m_method { Field::number("m", "Máxima fracción defectuosa M (%)") } else { Field::number("k", "Valor crítico (k)") },
                Field::number("lsl", "Límite inferior de especificación (LSL)").optional().show_if("side", &["lower", "both"]),
                Field::number("usl", "Límite superior de especificación (USL)").optional().show_if("side", &["upper", "both"]),
            ]);
            v.extend(quality());
            v.push(Field::number("lot_size", "Tamaño del lote (N)").optional());
            v
        };
        let mut seqv = var_common();
        seqv.extend([
            Field::number("x1", "Media aceptable (AQL)"),
            Field::number("x2l", "Media rechazable inferior (RQL)").optional().show_if("side", &["lower", "both"]),
            Field::number("x2u", "Media rechazable superior (RQL)").optional().show_if("side", &["upper", "both"]),
            Field::number("alpha", "Riesgo del productor α (%)").default(5.0),
            Field::number("beta", "Riesgo del consumidor β (%)").default(10.0),
            Field::number("offset", "Desplazamiento de los datos codificados"),
        ]);
        let vt = |p: Plan, fields: Vec<Field>, desc: &str| ProblemType { id: p.id().into(), label: p.label().into(), description: desc.into(), dims: vec![], layout: vec![Widget::params(None, fields), Widget::params(Some("Costos (opcionales)"), costs())] };
        types.push(vt(Plan::Mean, mean, "Se acepta si la media de la muestra está del lado bueno del límite"));
        types.push(vt(Plan::KMethod, km(false), "Se acepta si (USL − media)/σ o (media − LSL)/σ es al menos k"));
        types.push(vt(Plan::MMethod, km(true), "Se acepta si la fracción defectuosa estimada no pasa de M"));
        types.push(vt(Plan::SequentialVariable, seqv, "Mediciones una a una hasta cruzar una recta de aceptación o rechazo"));
        ModuleSpec {
            id: "asa".into(),
            winqsb: "ASA".into(),
            name: "Muestreo de aceptación".into(),
            description: "Planes por atributos y por variables, curvas OC, AOQ, ATI, ASN y costos, y búsqueda del plan".into(),
            file_ext: "ASA".into(),
            problem_types: types,
            options: vec![
                Field::boolean("steps", "Mostrar paso a paso", true),
                Field::choice("mode", "Hacer", &[("analyze", "Analizar el plan"), ("search", "Buscar el plan (AQL, RQL, α, β)")]).default("analyze").show_if("problem_type", &["single", "double", "chain", "continuous", "skip_lot", "mean", "k_method", "m_method"]),
                Field::choice("criterion", "Criterio de búsqueda", &[("risks", "Cumplir α y β"), ("weighted", "Menor desviación ponderada de α y β"), ("aoql", "AOQL máximo")]).default("risks").show_if("mode", &["search"]),
                Field::number("weight_alpha", "Peso de α").default(0.5).show_if("criterion", &["weighted"]).show_if("mode", &["search"]),
                Field::number("weight_beta", "Peso de β").default(0.5).show_if("criterion", &["weighted"]).show_if("mode", &["search"]),
                Field::number("aoql_limit", "AOQL máximo (%)").default(2.0).show_if("criterion", &["aoql"]).show_if("mode", &["search"]),
                Field::number("what_if", "Qué pasa si: % defectuoso o media").optional(),
                Field::number("from", "Curvas desde").optional().help("% defectuoso o media"),
                Field::number("to", "Curvas hasta").optional(),
                Field::integer("points", "Puntos de las curvas", 40),
                Field::text("observations", "Datos para el proceso secuencial").help("0/1 por unidad (atributos) o mediciones, separados por comas").show_if("problem_type", &["sequential", "sequential_variable"]),
            ],
        }
    }

    fn template(&self, pt: &str, dims: &Value) -> Result<AsaModel, ModError> {
        let p: Plan = serde_json::from_value(Value::from(pt)).map_err(|_| ModError::Invalid(format!("tipo de problema desconocido: {pt}")))?;
        let mut m: AsaModel = serde_json::from_value(json!({"name": "Plan de muestreo", "problem_type": p, "aql": 1, "rql": 5, "lot_size": 1000})).unwrap();
        match p {
            Plan::Single | Plan::Chain | Plan::SkipLot => m.stages = vec![Stage { n: 50.0, c: if p == Plan::Chain { 0.0 } else { 1.0 }, r: None }],
            Plan::Double => m.stages = vec![Stage { n: 50.0, c: 1.0, r: Some(4.0) }, Stage { n: 100.0, c: 4.0, r: None }],
            Plan::Multiple => {
                let k = dim(dims, "stages", 3, 2, 20)?;
                m.stages = (0..k).map(|i| Stage { n: 20.0, c: i as f64, r: if i + 1 < k { Some(i as f64 + 3.0) } else { None } }).collect();
            }
            _ => {}
        }
        if matches!(p, Plan::Continuous | Plan::SkipLot) {
            m.fraction = 10.0;
            m.clearance = if p == Plan::SkipLot { 10.0 } else { 50.0 };
        }
        if p == Plan::Chain {
            m.clearance = 3.0;
        }
        if p.variable() {
            m.sigma = 1.0;
            m.n = 10.0;
            m.x1 = 10.0;
            m.x2l = Some(9.0);
            m.k = 2.0;
            m.m = 2.0;
            m.lsl = Some(7.0);
        }
        Ok(m)
    }

    fn solve(&self, m: &AsaModel, o: &AsaOptions) -> Result<Report, ModError> {
        let mut r = if m.problem_type.variable() { var::solve(m, o)? } else { attr::solve(m, o)? };
        if r.problem.is_empty() {
            r.problem = m.name.clone();
        }
        Ok(r)
    }

    fn read(&self, bytes: &[u8]) -> Result<AsaModel, ModError> {
        read_asa(bytes)
    }

    fn write(&self, m: &AsaModel) -> Result<Vec<u8>, ModError> {
        Ok(write_asa(m))
    }

    fn examples(&self) -> Vec<(&'static str, AsaModel)> {
        let ex = |s: &str| serde_json::from_str::<AsaModel>(s).unwrap();
        vec![
            ("muestreo_simple", ex(include_str!("ejemplos/simple.json"))),
            ("muestreo_doble", ex(include_str!("ejemplos/doble.json"))),
            ("secuencial_atributos", ex(include_str!("ejemplos/secuencial.json"))),
            ("continuo_csp1", ex(include_str!("ejemplos/csp1.json"))),
            ("media_variables", ex(include_str!("ejemplos/media.json"))),
            ("metodo_k", ex(include_str!("ejemplos/metodo_k.json"))),
        ]
    }
}

// ------------------------------------------------------------------ archivos

fn side_code(s: Side) -> &'static str {
    match s {
        Side::Lower => "0",
        Side::Upper => "1",
        Side::Both => "2",
    }
}

fn dist_name(d: ProbDist) -> &'static str {
    match d {
        ProbDist::Binomial => "Binomial",
        ProbDist::Hypergeometric => "Hypergeometric",
        ProbDist::Poisson => "Poisson",
        ProbDist::Normal => "Normal",
        ProbDist::Auto => "Program selection",
    }
}

fn write_asa(m: &AsaModel) -> Vec<u8> {
    let p = m.problem_type;
    let mut w = Writer::new();
    let yn = |b: bool| if b { "-1" } else { "0" }.to_string();
    w.row(&["ASA".to_string(), m.name.clone(), if p.variable() { "1" } else { "0" }.into(), p.code().to_string()]);
    w.row(&[yn(m.rectified), yn(m.curtailed), m.stages.len().max(1).to_string(), if m.sigma_known { "0".to_string() } else { "1".to_string() }, side_code(m.side).into()]);
    w.row(&[format!("{}DATA  ITEM", " ".repeat(20)), "ENTRY".into()]);
    let mut item = |l: &str, v: String| {
        w.row(&[l.to_string(), v]);
    };
    let o = |x: Option<f64>| x.map(num).unwrap_or_else(|| " ".into());
    let st = |k: usize| m.stages.get(k).cloned().unwrap_or(Stage { n: 0.0, c: 0.0, r: None });
    match p {
        Plan::Single => {
            item("Sample size (n)", num(st(0).n));
            item("Acceptance number (c)", num(st(0).c));
        }
        Plan::Double => {
            item("First sample size (n1)", num(st(0).n));
            item("First acceptance number (c1)", num(st(0).c));
            item("First rejection number (r1)", num(st(0).r.unwrap_or(st(1).c + 1.0)));
            item("Second sample size (n2)", num(st(1).n));
            item("Second acceptance number (c2)", num(st(1).c));
        }
        Plan::Multiple => {
            for k in 0..m.stages.len() {
                item(&format!("Sample size (n{})", k + 1), num(st(k).n));
                item(&format!("Acceptance number (c{})", k + 1), num(st(k).c));
                if k + 1 < m.stages.len() {
                    item(&format!("Rejection number (r{})", k + 1), num(st(k).r.unwrap_or(st(k).c + 1.0)));
                }
            }
        }
        Plan::Sequential => {}
        Plan::Chain => {
            item("Sample size (n)", num(st(0).n));
            item("Parameter for the number of previous lots (i)", num(m.clearance));
        }
        Plan::Continuous => {
            item("Percent (%) of units to be inspected (f)", num(m.fraction));
            item("Clearance number (i)", num(m.clearance));
        }
        Plan::SkipLot => {
            item("Percent (%) of lots to be inspected (f)", num(m.fraction));
            item("Clearance number (i)", num(m.clearance));
            item("Reference sampling plan sample size (n)", num(st(0).n));
            item("Reference sampling plan acceptance number (c)", num(st(0).c));
        }
        Plan::Mean => {
            item("Sample size (n)", num(m.n));
            match m.side {
                Side::Lower => {
                    item("Lower acceptance limit (XLa)", o(m.xla));
                    item("Lower specification limit (LSL)", o(m.lsl));
                    item("Good/acceptable average quality level (AQL/X1, > XLa)", num(m.x1));
                    item("Poor/rejectable average quality level (RQL/X2, < XLa)", o(m.x2l));
                }
                Side::Upper => {
                    item("Upper acceptance limit (XUa)", o(m.xua));
                    item("Upper specification limit (USL)", o(m.usl));
                    item("Good/acceptable average quality level (AQL/X1, < XUa)", num(m.x1));
                    item("Poor/rejectable average quality level (RQL/X2, > XUa)", o(m.x2u));
                }
                Side::Both => {
                    item("Lower acceptance limit (XLa)", o(m.xla));
                    item("Upper acceptance limit (XUa)", o(m.xua));
                    item("Lower specification limit (LSL)", o(m.lsl));
                    item("Upper specification limit (USL)", o(m.usl));
                    item("Good/acceptable average quality level (AQL/X1, XLa < X1 < XUa)", num(m.x1));
                    item("Lower poor/rejectable average quality level (RQL/X2L, < XLa)", o(m.x2l));
                    item("Upper poor/rejectable average quality level (RQL/X2U, > XUa)", o(m.x2u));
                }
            }
        }
        Plan::KMethod | Plan::MMethod => {
            item("Sample size (n)", num(m.n));
            item(if p == Plan::KMethod { "Critical value (k)" } else { "Maximum allowable fraction defective (M in %)" }, num(if p == Plan::KMethod { m.k } else { m.m }));
            match m.side {
                Side::Lower => item("Lower specification limit (LSL)", o(m.lsl)),
                Side::Upper => item("Upper specification limit (USL)", o(m.usl)),
                Side::Both => {
                    item("Lower specification limit (LSL)", o(m.lsl));
                    item("Upper specification limit (USL)", o(m.usl));
                }
            }
        }
        Plan::SequentialVariable => match m.side {
            Side::Lower => {
                item("Offset for the coded data", num(m.offset));
                item("Good/acceptable average quality level (AQL/XU)", num(m.x1));
                item("Poor/rejectable average quality level (RQL/XL)", o(m.x2l));
            }
            Side::Upper => {
                item("Offset for the coded data", num(m.offset));
                item("Good/acceptable average quality level (AQL/XL)", num(m.x1));
                item("Poor/rejectable average quality level (RQL/XU)", o(m.x2u));
            }
            Side::Both => {
                item("Lower poor/rejectable average quality level (RQL/XL)", o(m.x2l));
                item("Upper poor/rejectable average quality level (RQL/XU)", o(m.x2u));
            }
        },
    }
    if p != Plan::Mean && p != Plan::SequentialVariable {
        item("Acceptable quality level (AQL, in % defective)", num(m.aql));
        item("Rejectable quality level (RQL, LQL, or LTPD, in % defective)", num(m.rql));
    }
    if p.variable() {
        item(if m.sigma_known { "Population/process/universe standard deviation (sigma)" } else { "Sample standard deviation (sigma)" }, num(m.sigma));
    }
    item("Producer's risk level (alpha, type I error in %)", num(m.alpha));
    item("Consumer's risk level (beta, type II error in %)", num(m.beta));
    if p != Plan::Continuous {
        item("Lot size (N)", o(m.lot_size));
    }
    if !matches!(p, Plan::Sequential | Plan::Continuous) && !p.variable() {
        item("Probability distribution", dist_name(m.distribution).into());
    }
    if p != Plan::SequentialVariable {
        item("Unit sampling cost", num(m.sampling_cost));
        item("Unit inspection cost", num(m.inspection_cost));
        item("Unit producer's cost if unit is found defective", num(m.producer_cost));
        item("Unit consumer's cost if unit is found defective", num(m.consumer_cost));
    }
    if p != Plan::Sequential && !p.variable() {
        item("Inspection error: probability (%) of good item is classified as bad", num(m.error_good));
        item("Inspection error: probability (%) of bad item is classified as good", num(m.error_bad));
    }
    w.bytes()
}

fn read_asa(bytes: &[u8]) -> Result<AsaModel, ModError> {
    let mut r = Reader::new(bytes);
    let h = r.line("la cabecera")?;
    if !h.get(0).trim().to_ascii_uppercase().starts_with("ASA") {
        return Err(ModError::Format("no es un archivo de ASA".into()));
    }
    let code = parse_num(h.get(3)).unwrap_or(0.0) as usize;
    let p = *Plan::ALL.get(code).ok_or_else(|| ModError::Format(format!("plan de muestreo desconocido: {code}")))?;
    let mut m: AsaModel = serde_json::from_value(json!({"name": h.get(1).trim(), "problem_type": p})).unwrap();
    if m.name.is_empty() {
        m.name = "ASA Problem".into();
    }
    let l2 = r.line("los indicadores")?;
    let flag = |s: &str| {
        let t = s.trim().to_ascii_uppercase();
        t.starts_with('Y') || t.starts_with('T') || parse_num(&t).map(|v| v != 0.0).unwrap_or(false)
    };
    m.rectified = flag(l2.get(0));
    m.curtailed = flag(l2.get(1));
    let k = (parse_num(l2.get(2)).unwrap_or(1.0) as usize).max(1);
    let sk = l2.get(3).trim().to_ascii_uppercase();
    m.sigma_known = sk.starts_with('Y') || sk.starts_with('K') || parse_num(&sk).unwrap_or(0.0) == 0.0;
    let sd = l2.get(4).trim().to_ascii_uppercase();
    m.side = match sd.chars().next() {
        Some('U') | Some('1') => Side::Upper,
        Some('B') | Some('2') => Side::Both,
        _ => Side::Lower,
    };
    r.line("la fila de títulos")?;
    let v = |r: &mut Reader| -> Result<String, ModError> { Ok(r.line("un dato")?.get(1).trim().to_string()) };
    let n0 = |s: &str| parse_num(s).unwrap_or(0.0);
    match p {
        Plan::Single => {
            let n = n0(&v(&mut r)?);
            let c = n0(&v(&mut r)?);
            m.stages = vec![Stage { n, c, r: None }];
        }
        Plan::Double | Plan::Multiple => {
            let k = if p == Plan::Double { 2 } else { k };
            for s in 0..k {
                let n = n0(&v(&mut r)?);
                let c = n0(&v(&mut r)?);
                let rr = if s + 1 < k { Some(n0(&v(&mut r)?)) } else { None };
                m.stages.push(Stage { n, c, r: rr });
            }
        }
        Plan::Sequential => {}
        Plan::Chain => {
            let n = n0(&v(&mut r)?);
            m.clearance = n0(&v(&mut r)?);
            m.stages = vec![Stage { n, c: 0.0, r: None }];
        }
        Plan::Continuous => {
            m.fraction = n0(&v(&mut r)?);
            m.clearance = n0(&v(&mut r)?);
        }
        Plan::SkipLot => {
            m.fraction = n0(&v(&mut r)?);
            m.clearance = n0(&v(&mut r)?);
            let n = n0(&v(&mut r)?);
            let c = n0(&v(&mut r)?);
            m.stages = vec![Stage { n, c, r: None }];
        }
        Plan::Mean => {
            m.n = n0(&v(&mut r)?);
            match m.side {
                Side::Lower => {
                    m.xla = parse_num(&v(&mut r)?);
                    m.lsl = parse_num(&v(&mut r)?);
                    m.x1 = n0(&v(&mut r)?);
                    m.x2l = parse_num(&v(&mut r)?);
                }
                Side::Upper => {
                    m.xua = parse_num(&v(&mut r)?);
                    m.usl = parse_num(&v(&mut r)?);
                    m.x1 = n0(&v(&mut r)?);
                    m.x2u = parse_num(&v(&mut r)?);
                }
                Side::Both => {
                    m.xla = parse_num(&v(&mut r)?);
                    m.xua = parse_num(&v(&mut r)?);
                    m.lsl = parse_num(&v(&mut r)?);
                    m.usl = parse_num(&v(&mut r)?);
                    m.x1 = n0(&v(&mut r)?);
                    m.x2l = parse_num(&v(&mut r)?);
                    m.x2u = parse_num(&v(&mut r)?);
                }
            }
        }
        Plan::KMethod | Plan::MMethod => {
            m.n = n0(&v(&mut r)?);
            let x = n0(&v(&mut r)?);
            if p == Plan::KMethod {
                m.k = x
            } else {
                m.m = x
            }
            match m.side {
                Side::Lower => m.lsl = parse_num(&v(&mut r)?),
                Side::Upper => m.usl = parse_num(&v(&mut r)?),
                Side::Both => {
                    m.lsl = parse_num(&v(&mut r)?);
                    m.usl = parse_num(&v(&mut r)?);
                }
            }
        }
        Plan::SequentialVariable => {
            m.offset = n0(&v(&mut r)?);
            match m.side {
                Side::Lower => {
                    m.x1 = n0(&v(&mut r)?);
                    m.x2l = parse_num(&v(&mut r)?);
                }
                Side::Upper => {
                    m.x1 = n0(&v(&mut r)?);
                    m.x2u = parse_num(&v(&mut r)?);
                }
                Side::Both => {
                    m.x2l = parse_num(&v(&mut r)?);
                    m.x2u = parse_num(&v(&mut r)?);
                    m.x1 = (m.x2l.unwrap_or(0.0) + m.x2u.unwrap_or(0.0)) / 2.0;
                }
            }
        }
    }
    if p != Plan::Mean && p != Plan::SequentialVariable {
        m.aql = n0(&v(&mut r)?);
        m.rql = n0(&v(&mut r)?);
    }
    if p.variable() {
        m.sigma = n0(&v(&mut r)?);
    }
    m.alpha = n0(&v(&mut r)?);
    m.beta = n0(&v(&mut r)?);
    if p != Plan::Continuous {
        m.lot_size = parse_num(&v(&mut r)?).filter(|x| *x > 0.0);
    }
    if !matches!(p, Plan::Sequential | Plan::Continuous) && !p.variable() {
        let d = v(&mut r)?.to_ascii_uppercase();
        m.distribution = match d.get(..3).unwrap_or("") {
            "HYP" => ProbDist::Hypergeometric,
            "POI" => ProbDist::Poisson,
            "NOR" => ProbDist::Normal,
            "PRO" => ProbDist::Auto,
            _ => ProbDist::Binomial,
        };
    }
    if p != Plan::SequentialVariable {
        m.sampling_cost = r.try_line().map(|l| n0(l.get(1))).unwrap_or(0.0);
        m.inspection_cost = r.try_line().map(|l| n0(l.get(1))).unwrap_or(0.0);
        m.producer_cost = r.try_line().map(|l| n0(l.get(1))).unwrap_or(0.0);
        m.consumer_cost = r.try_line().map(|l| n0(l.get(1))).unwrap_or(0.0);
    }
    if p != Plan::Sequential && !p.variable() {
        m.error_good = r.try_line().map(|l| n0(l.get(1))).unwrap_or(0.0);
        m.error_bad = r.try_line().map(|l| n0(l.get(1))).unwrap_or(0.0);
    }
    Ok(m)
}
