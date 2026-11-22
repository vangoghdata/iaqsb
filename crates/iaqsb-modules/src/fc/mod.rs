//! FC · Pronósticos y regresión lineal (equivalente al módulo FC de WinQSB).
//!
//! - Series de tiempo: los doce métodos de WinQSB (promedio simple, móviles,
//!   ponderados, con tendencia, suavizamiento exponencial simple, doble, con
//!   tendencia y adaptativo, regresión con el tiempo y Holt-Winters aditivo y
//!   multiplicativo), con CFE, MAD, MSE, MAPE y señal de rastreo, y búsqueda de
//!   los mejores parámetros.
//! - Regresión lineal múltiple: coeficientes con su error estándar, t y p,
//!   ANOVA, correlaciones, residuos y predicción con intervalos.

mod regress;
mod series;

use iaqsb_core::winqsb::{num, parse_num, Reader, Writer};
use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum FcType {
    #[default]
    Series,
    Regression,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Obs {
    #[serde(default)]
    pub name: String,
    pub value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Named {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FcModel {
    #[serde(default)]
    pub name: String,
    pub problem_type: FcType,
    #[serde(default = "month")]
    pub time_unit: String,
    /// Serie de tiempo (un valor por período).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data: Vec<Obs>,
    /// Regresión: variables (columnas) y observaciones (filas).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<Named>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub observations: Vec<Named>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<Vec<f64>>,
}

fn month() -> String {
    "mes".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    SimpleAverage,
    MovingAverage,
    WeightedMovingAverage,
    MovingAverageTrend,
    #[default]
    SingleExponential,
    SingleExponentialTrend,
    DoubleExponential,
    DoubleExponentialTrend,
    AdaptiveExponential,
    LinearTrend,
    HoltWintersAdditive,
    HoltWintersMultiplicative,
}

impl Method {
    pub const ALL: [Method; 12] = [
        Method::SimpleAverage,
        Method::MovingAverage,
        Method::WeightedMovingAverage,
        Method::MovingAverageTrend,
        Method::SingleExponential,
        Method::SingleExponentialTrend,
        Method::DoubleExponential,
        Method::DoubleExponentialTrend,
        Method::AdaptiveExponential,
        Method::LinearTrend,
        Method::HoltWintersAdditive,
        Method::HoltWintersMultiplicative,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Method::SimpleAverage => "simple_average",
            Method::MovingAverage => "moving_average",
            Method::WeightedMovingAverage => "weighted_moving_average",
            Method::MovingAverageTrend => "moving_average_trend",
            Method::SingleExponential => "single_exponential",
            Method::SingleExponentialTrend => "single_exponential_trend",
            Method::DoubleExponential => "double_exponential",
            Method::DoubleExponentialTrend => "double_exponential_trend",
            Method::AdaptiveExponential => "adaptive_exponential",
            Method::LinearTrend => "linear_trend",
            Method::HoltWintersAdditive => "holt_winters_additive",
            Method::HoltWintersMultiplicative => "holt_winters_multiplicative",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Method::SimpleAverage => "Promedio simple (SA)",
            Method::MovingAverage => "Promedio móvil (MA)",
            Method::WeightedMovingAverage => "Promedio móvil ponderado (WMA)",
            Method::MovingAverageTrend => "Promedio móvil con tendencia (MAT)",
            Method::SingleExponential => "Suavizamiento exponencial simple (SES)",
            Method::SingleExponentialTrend => "Suavizamiento exponencial con tendencia (SEST, Holt)",
            Method::DoubleExponential => "Suavizamiento exponencial doble (DES)",
            Method::DoubleExponentialTrend => "Suavizamiento exponencial doble con tendencia (DEST)",
            Method::AdaptiveExponential => "Suavizamiento exponencial adaptativo (AES)",
            Method::LinearTrend => "Regresión lineal con el tiempo (LR)",
            Method::HoltWintersAdditive => "Holt-Winters aditivo (HWA)",
            Method::HoltWintersMultiplicative => "Holt-Winters multiplicativo (HWM)",
        }
    }
    pub fn short(self) -> &'static str {
        match self {
            Method::SimpleAverage => "SA",
            Method::MovingAverage => "MA",
            Method::WeightedMovingAverage => "WMA",
            Method::MovingAverageTrend => "MAT",
            Method::SingleExponential => "SES",
            Method::SingleExponentialTrend => "SEST",
            Method::DoubleExponential => "DES",
            Method::DoubleExponentialTrend => "DEST",
            Method::AdaptiveExponential => "AES",
            Method::LinearTrend => "LR",
            Method::HoltWintersAdditive => "HWA",
            Method::HoltWintersMultiplicative => "HWM",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Criterion {
    #[default]
    Mse,
    Mad,
    Mape,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct FcOptions {
    pub steps: bool,
    pub method: Method,
    /// Períodos a pronosticar después de los datos.
    pub horizon: u32,
    /// Períodos del promedio móvil.
    pub periods: u32,
    /// Pesos del promedio ponderado, del más antiguo al más reciente.
    pub weights: String,
    pub alpha: f64,
    pub beta: f64,
    pub gamma: f64,
    /// Longitud del ciclo estacional (Holt-Winters).
    pub cycle: u32,
    /// Valores iniciales conocidos (F(0), T(0) o F'(0); a y b de la regresión).
    pub initial_level: Option<f64>,
    pub initial_trend: Option<f64>,
    /// Buscar los mejores parámetros (α, β, γ o períodos).
    pub search: bool,
    pub criterion: Criterion,
    /// Regresión: variable dependiente (nombre o número; vacío = la última).
    pub dependent: String,
    /// Nivel de significancia (%).
    pub significance: f64,
    /// Valores de las variables independientes para predecir, separados por comas.
    pub predict: String,
}

impl Default for FcOptions {
    fn default() -> Self {
        FcOptions {
            steps: true,
            method: Method::SingleExponential,
            horizon: 3,
            periods: 3,
            weights: String::new(),
            alpha: 0.3,
            beta: 0.2,
            gamma: 0.2,
            cycle: 4,
            initial_level: None,
            initial_trend: None,
            search: false,
            criterion: Criterion::Mse,
            dependent: String::new(),
            significance: 5.0,
            predict: String::new(),
        }
    }
}

pub struct Fc;

impl TypedModule for Fc {
    type Model = FcModel;
    type Options = FcOptions;

    fn file_tag(&self) -> &'static str {
        "FC"
    }

    fn spec(&self) -> ModuleSpec {
        let methods: Vec<(&str, &str)> = Method::ALL.iter().map(|m| (m.id(), m.label())).collect();
        let ser = &["series"];
        let only = |f: Field, ms: &[Method]| f.show_if("problem_type", ser).show_if("method", &ms.iter().map(|m| m.id()).collect::<Vec<_>>());
        use Method::*;
        ModuleSpec {
            id: "fc".into(),
            winqsb: "FC".into(),
            name: "Pronósticos".into(),
            description: "Doce métodos de series de tiempo y regresión lineal múltiple".into(),
            file_ext: "FCC".into(),
            problem_types: vec![
                ProblemType {
                    id: "series".into(),
                    label: "Serie de tiempo".into(),
                    description: "Pronosticar los próximos períodos a partir de los datos históricos".into(),
                    dims: vec![Field::integer("periods", "Número de períodos", 12), Field::text("time_unit", "Unidad de tiempo").default("mes")],
                    layout: vec![
                        Widget::params(None, vec![Field::text("time_unit", "Unidad de tiempo")]),
                        Widget::records(Some("Datos históricos"), "data", vec![Field::text("name", "Período"), Field::number("value", "Dato").optional()], Some("Añadir período")),
                    ],
                },
                ProblemType {
                    id: "regression".into(),
                    label: "Regresión lineal".into(),
                    description: "Relación lineal entre una variable dependiente y las demás".into(),
                    dims: vec![Field::integer("variables", "Número de variables", 2), Field::integer("observations", "Número de observaciones", 10)],
                    layout: vec![Widget::matrix(
                        Some("Observaciones"),
                        "values",
                        Axis::new("observations", "Observación").add("Añadir observación"),
                        Axis::new("variables", "Variable").add("Añadir variable"),
                        Field::number("v", "Valor"),
                    )],
                },
            ],
            options: vec![
                Field::boolean("steps", "Mostrar paso a paso", true),
                Field::choice("method", "Método", &methods).comparable().default("single_exponential").show_if("problem_type", ser),
                Field::integer("horizon", "Períodos a pronosticar", 3).show_if("problem_type", ser),
                only(Field::integer("periods", "Períodos del promedio (m)", 3), &[MovingAverage, WeightedMovingAverage, MovingAverageTrend]),
                only(Field::text("weights", "Pesos (del más antiguo al más reciente)").help("Ej.: 1, 2, 3; vacío = iguales"), &[WeightedMovingAverage]),
                only(Field::number("alpha", "α (suavizamiento)").default(0.3), &[SingleExponential, SingleExponentialTrend, DoubleExponential, DoubleExponentialTrend, AdaptiveExponential, HoltWintersAdditive, HoltWintersMultiplicative]),
                only(Field::number("beta", "β (tendencia o ajuste)").default(0.2), &[SingleExponentialTrend, AdaptiveExponential, HoltWintersAdditive, HoltWintersMultiplicative]),
                only(Field::number("gamma", "γ (estacionalidad)").default(0.2), &[HoltWintersAdditive, HoltWintersMultiplicative]),
                only(Field::integer("cycle", "Longitud del ciclo (c)", 4), &[HoltWintersAdditive, HoltWintersMultiplicative]),
                only(
                    Field::number("initial_level", "Valor inicial F(0) o intercepto a (opcional)").optional(),
                    &[SingleExponential, SingleExponentialTrend, DoubleExponential, DoubleExponentialTrend, AdaptiveExponential, LinearTrend, HoltWintersAdditive, HoltWintersMultiplicative],
                ),
                only(
                    Field::number("initial_trend", "Valor inicial T(0), F'(0) o pendiente b (opcional)").optional(),
                    &[SingleExponentialTrend, DoubleExponential, DoubleExponentialTrend, LinearTrend, HoltWintersAdditive, HoltWintersMultiplicative],
                ),
                only(
                    Field::boolean("search", "Buscar los mejores parámetros", false),
                    &[MovingAverage, WeightedMovingAverage, MovingAverageTrend, SingleExponential, SingleExponentialTrend, DoubleExponential, DoubleExponentialTrend, AdaptiveExponential, HoltWintersAdditive, HoltWintersMultiplicative],
                ),
                Field::choice("criterion", "Criterio de búsqueda", &[("mse", "MSE"), ("mad", "MAD"), ("mape", "MAPE")]).default("mse").show_if("problem_type", ser).show_if("search", &["true"]),
                Field::text("dependent", "Variable dependiente").help("Nombre o número; vacío = la última").show_if("problem_type", &["regression"]),
                Field::number("significance", "Nivel de significancia (%)").default(5.0).show_if("problem_type", &["regression"]),
                Field::text("predict", "Predecir con (valores de las independientes)").help("Separados por comas, en orden").show_if("problem_type", &["regression"]),
            ],
        }
    }

    fn template(&self, pt: &str, dims: &Value) -> Result<FcModel, ModError> {
        let t: FcType = serde_json::from_value(Value::from(pt)).map_err(|_| ModError::Invalid(format!("tipo de problema desconocido: {pt}")))?;
        let mut m = FcModel { name: "Pronóstico nuevo".into(), problem_type: t, time_unit: dim_str(dims, "time_unit", "mes"), data: vec![], variables: vec![], observations: vec![], values: vec![] };
        match t {
            FcType::Series => {
                let n = dim(dims, "periods", 12, 1, 5000)?;
                m.data = (1..=n).map(|i| Obs { name: i.to_string(), value: None }).collect();
            }
            FcType::Regression => {
                let v = dim(dims, "variables", 2, 2, 50)?;
                let n = dim(dims, "observations", 10, 1, 10000)?;
                m.variables = (1..=v).map(|i| Named { name: if i == v { "Y".into() } else { format!("X{i}") } }).collect();
                m.observations = (1..=n).map(|i| Named { name: i.to_string() }).collect();
                m.values = vec![vec![0.0; v]; n];
            }
        }
        Ok(m)
    }

    fn solve(&self, m: &FcModel, o: &FcOptions) -> Result<Report, ModError> {
        let mut r = match m.problem_type {
            FcType::Series => series::solve(m, o)?,
            FcType::Regression => regress::solve(m, o)?,
        };
        if r.problem.is_empty() {
            r.problem = m.name.clone();
        }
        Ok(r)
    }

    fn read(&self, bytes: &[u8]) -> Result<FcModel, ModError> {
        let mut r = Reader::new(bytes);
        let h = r.line("la cabecera")?;
        if !h.get(0).trim().to_ascii_uppercase().starts_with("FC") {
            return Err(ModError::Format("no es un archivo de FC".into()));
        }
        let name = Some(h.get(1).trim()).filter(|s| !s.is_empty()).unwrap_or("FC Problem").to_string();
        let t = if parse_num(h.get(2)).unwrap_or(0.0) as i64 == 1 { FcType::Regression } else { FcType::Series };
        let mut m = FcModel { name, problem_type: t, time_unit: "month".into(), data: vec![], variables: vec![], observations: vec![], values: vec![] };
        match t {
            FcType::Series => {
                let u = h.get(3).trim();
                if !u.is_empty() {
                    m.time_unit = u.to_string();
                }
                let n = parse_num(h.get(4)).unwrap_or(0.0) as usize;
                if n == 0 || n > 1_000_000 {
                    return Err(ModError::Format("el número de períodos no es válido".into()));
                }
                r.line("la fila de títulos")?;
                for i in 0..n {
                    let l = r.line(&format!("el período {}", i + 1))?;
                    let label = l.get(0).trim();
                    m.data.push(Obs { name: if label.is_empty() { (i + 1).to_string() } else { label.to_string() }, value: parse_num(l.get(1)) });
                }
            }
            FcType::Regression => {
                let v = parse_num(h.get(3)).unwrap_or(0.0) as usize;
                let n = parse_num(h.get(4)).unwrap_or(0.0) as usize;
                if v == 0 || n == 0 || v > 1000 || n > 1_000_000 {
                    return Err(ModError::Format("el tamaño del problema no es válido".into()));
                }
                let head = r.line("los nombres de las variables")?;
                m.variables = (0..v).map(|j| Named { name: Some(head.get(j + 1).trim()).filter(|s| !s.is_empty()).map(str::to_string).unwrap_or(format!("Factor{}", j + 1)) }).collect();
                for i in 0..n {
                    let l = r.line(&format!("la observación {}", i + 1))?;
                    m.observations.push(Named { name: Some(l.get(0).trim()).filter(|s| !s.is_empty()).map(str::to_string).unwrap_or((i + 1).to_string()) });
                    m.values.push((0..v).map(|j| parse_num(l.get(j + 1)).unwrap_or(0.0)).collect());
                }
            }
        }
        Ok(m)
    }

    fn write(&self, m: &FcModel) -> Result<Vec<u8>, ModError> {
        let mut w = Writer::new();
        match m.problem_type {
            FcType::Series => {
                w.row(&["FC".to_string(), m.name.clone(), "0".into(), m.time_unit.clone(), m.data.len().to_string()]);
                w.row(&["Number", "Historical Data"]);
                for (i, d) in m.data.iter().enumerate() {
                    w.row(&[if d.name.trim().is_empty() { (i + 1).to_string() } else { d.name.clone() }, d.value.map(num).unwrap_or_else(|| " ".into())]);
                }
            }
            FcType::Regression => {
                let v = m.variables.len();
                w.row(&["FC".to_string(), m.name.clone(), "1".into(), v.to_string(), m.observations.len().to_string()]);
                let mut h = vec!["Observation".to_string()];
                h.extend(m.variables.iter().map(|x| x.name.clone()));
                w.row(&h);
                for (i, o) in m.observations.iter().enumerate() {
                    let mut r = vec![o.name.clone()];
                    r.extend((0..v).map(|j| num(m.values.get(i).and_then(|x| x.get(j)).copied().unwrap_or(0.0))));
                    w.row(&r);
                }
            }
        }
        Ok(w.bytes())
    }

    fn examples(&self) -> Vec<(&'static str, FcModel)> {
        let ex = |s: &str| serde_json::from_str::<FcModel>(s).unwrap();
        vec![
            ("ventas_mensuales", ex(include_str!("ejemplos/ventas.json"))),
            ("estacional_trimestral", ex(include_str!("ejemplos/estacional.json"))),
            ("regresion_ventas", ex(include_str!("ejemplos/regresion.json"))),
        ]
    }
}
