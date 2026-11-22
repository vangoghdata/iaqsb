//! QCC · Gráficos de control de calidad (equivalente al módulo QCC de WinQSB).
//!
//! 21 gráficos para variables (X̄, R, S, S², X, mediana, rango medio, CUSUM,
//! tendencia, media móvil geométrica y simple, modificados y de aceptación) y
//! 15 para atributos (P, nP, C, u, U de deméritos, CUSUM, medias móviles y
//! estandarizados), las 14 reglas de fuera de control configurables, curva OC,
//! capacidad del proceso, histograma con prueba ji cuadrada, gráfico de
//! probabilidad y análisis de Pareto.
//!
//! Formato de archivo comprobado con los ejemplos originales VARIABLE,
//! PROBPLOT, C_CHART, P_CHART y PARETO.

pub mod chart;
pub mod consts;
pub mod tools;

use iaqsb_core::winqsb::{num, parse_num, Reader, Writer};
use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum QccType {
    #[default]
    Variable,
    Attribute,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Characteristic {
    pub name: String,
    #[serde(default)]
    pub lsl: Option<f64>,
    #[serde(default)]
    pub target: Option<f64>,
    #[serde(default)]
    pub usl: Option<f64>,
    /// Peso de demérito (gráfico U).
    #[serde(default)]
    pub demerit: Option<f64>,
}

/// Una fila de datos: observación (variables, formato vertical) o subgrupo (atributos).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
pub struct DataRow {
    /// Número de la fila (observación o subgrupo).
    #[serde(default, rename = "name")]
    pub id: String,
    /// Variables: subgrupo al que pertenece la observación.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subgroup: Option<f64>,
    /// Atributos: tamaño del subgrupo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub date: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub time: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub disabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<f64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub comment: String,
}

/// Regla de fuera de control (columnas de WinQSB).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Rule {
    pub description: String,
    /// n de m puntos.
    #[serde(default)]
    pub n_of: f64,
    #[serde(default)]
    pub m_points: f64,
    /// 1 = sobre el LCS, −1 = bajo el LCI.
    #[serde(default)]
    pub off_limit: f64,
    /// Desviaciones desde la línea central (+ arriba, − abajo).
    #[serde(default)]
    pub sigma_off: f64,
    /// Seguidos sobre (1) o bajo (−1) la línea central.
    #[serde(default)]
    pub row_cl: f64,
    /// Seguidos sobre (1) o bajo (−1) la mediana del gráfico.
    #[serde(default)]
    pub row_median: f64,
    /// Seguidos subiendo (1) o bajando (−1).
    #[serde(default)]
    pub row_updown: f64,
    /// Salto entre dos puntos seguidos, en desviaciones (+ sube, − baja).
    #[serde(default)]
    pub jump: f64,
}

pub fn default_rules() -> Vec<Rule> {
    let r = |d: &str, n: f64, m: f64, v: [f64; 6]| Rule { description: d.into(), n_of: n, m_points: m, off_limit: v[0], sigma_off: v[1], row_cl: v[2], row_median: v[3], row_updown: v[4], jump: v[5] };
    vec![
        r("Single Point Above UCL", 1.0, 1.0, [1.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        r("Single Point Below LCL", 1.0, 1.0, [-1.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        r("2 of 3 Points Above 2 Sigma", 2.0, 3.0, [0.0, 2.0, 0.0, 0.0, 0.0, 0.0]),
        r("2 of 3 Points Below 2 Sigma", 2.0, 3.0, [0.0, -2.0, 0.0, 0.0, 0.0, 0.0]),
        r("4 of 5 Points Above 1 Sigma", 4.0, 5.0, [0.0, 1.0, 0.0, 0.0, 0.0, 0.0]),
        r("4 of 5 Points Below 1 Sigma", 4.0, 5.0, [0.0, -1.0, 0.0, 0.0, 0.0, 0.0]),
        r("8 Points in a Row Above CL", 8.0, 8.0, [0.0, 0.0, 1.0, 0.0, 0.0, 0.0]),
        r("8 Points in a Row Below CL", 8.0, 8.0, [0.0, 0.0, -1.0, 0.0, 0.0, 0.0]),
        r("8 Points in a Row Above Median", 8.0, 8.0, [0.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
        r("8 Points in a Row Below Median", 8.0, 8.0, [0.0, 0.0, 0.0, -1.0, 0.0, 0.0]),
        r("8 Points in a Row Up", 8.0, 8.0, [0.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
        r("8 Points in a Row Down", 8.0, 8.0, [0.0, 0.0, 0.0, 0.0, -1.0, 0.0]),
        r("Single Point Jumps Up 4 Sigma", 1.0, 1.0, [0.0, 0.0, 0.0, 0.0, 0.0, 4.0]),
        r("Single Point Jumps Down 4 Sigma", 1.0, 1.0, [0.0, 0.0, 0.0, 0.0, 0.0, -4.0]),
    ]
}

fn rules_default() -> Vec<Rule> {
    default_rules()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct QccModel {
    #[serde(default)]
    pub name: String,
    pub problem_type: QccType,
    /// Variables: el archivo usa el formato horizontal (una fila por subgrupo).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub horizontal: bool,
    pub characteristics: Vec<Characteristic>,
    pub rows: Vec<DataRow>,
    /// Valores: una fila por observación o subgrupo, una columna por característica.
    pub data: Vec<Vec<Option<f64>>>,
    #[serde(default)]
    pub causes: Vec<String>,
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default = "rules_default")]
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChartKind {
    Xbar,
    R,
    S,
    Variance,
    X,
    Median,
    Midrange,
    CusumXbar,
    CusumX,
    CusumR,
    CusumS,
    TrendXbar,
    TrendX,
    GmaXbar,
    GmaX,
    MaXbar,
    MaX,
    ModifiedXbar,
    ModifiedX,
    AcceptanceXbar,
    AcceptanceX,
    P,
    Np,
    C,
    U,
    Demerits,
    CusumP,
    CusumC,
    GmaP,
    GmaC,
    MaP,
    MaC,
    StdP,
    StdNp,
    StdC,
    StdU,
}

pub const CHARTS: [(ChartKind, &str, &str); 36] = [
    (ChartKind::Xbar, "xbar", "X̄ (media)"),
    (ChartKind::R, "r", "R (rango)"),
    (ChartKind::S, "s", "S (desviación estándar)"),
    (ChartKind::Variance, "variance", "S² (varianza)"),
    (ChartKind::X, "x", "X (individuales)"),
    (ChartKind::Median, "median", "X̃ (mediana)"),
    (ChartKind::Midrange, "midrange", "M (rango medio)"),
    (ChartKind::CusumXbar, "cusum_xbar", "CUSUM de la media"),
    (ChartKind::CusumX, "cusum_x", "CUSUM de individuales"),
    (ChartKind::CusumR, "cusum_r", "CUSUM del rango"),
    (ChartKind::CusumS, "cusum_s", "CUSUM de la desviación"),
    (ChartKind::TrendXbar, "trend_xbar", "Tendencia de la media"),
    (ChartKind::TrendX, "trend_x", "Tendencia de individuales"),
    (ChartKind::GmaXbar, "gma_xbar", "Media móvil geométrica (EWMA) de la media"),
    (ChartKind::GmaX, "gma_x", "Media móvil geométrica (EWMA) de individuales"),
    (ChartKind::MaXbar, "ma_xbar", "Media móvil de la media"),
    (ChartKind::MaX, "ma_x", "Media móvil de individuales"),
    (ChartKind::ModifiedXbar, "modified_xbar", "Modificado para la media"),
    (ChartKind::ModifiedX, "modified_x", "Modificado para individuales"),
    (ChartKind::AcceptanceXbar, "acceptance_xbar", "De aceptación para la media"),
    (ChartKind::AcceptanceX, "acceptance_x", "De aceptación para individuales"),
    (ChartKind::P, "p", "P (proporción defectuosa)"),
    (ChartKind::Np, "np", "nP (número de defectuosos)"),
    (ChartKind::C, "c", "C (número de defectos)"),
    (ChartKind::U, "u", "u (defectos por unidad)"),
    (ChartKind::Demerits, "demerits", "U (deméritos por unidad)"),
    (ChartKind::CusumP, "cusum_p", "CUSUM de P"),
    (ChartKind::CusumC, "cusum_c", "CUSUM de C"),
    (ChartKind::GmaP, "gma_p", "EWMA de P"),
    (ChartKind::GmaC, "gma_c", "EWMA de C"),
    (ChartKind::MaP, "ma_p", "Media móvil de P"),
    (ChartKind::MaC, "ma_c", "Media móvil de C"),
    (ChartKind::StdP, "std_p", "P estandarizado"),
    (ChartKind::StdNp, "std_np", "nP estandarizado"),
    (ChartKind::StdC, "std_c", "C estandarizado"),
    (ChartKind::StdU, "std_u", "u estandarizado"),
];

impl ChartKind {
    pub fn label(self) -> &'static str {
        CHARTS.iter().find(|c| c.0 == self).unwrap().2
    }
    pub fn attribute(self) -> bool {
        CHARTS.iter().position(|c| c.0 == self).unwrap() >= 21
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Limits {
    /// k desviaciones desde la línea central.
    #[default]
    Sigma,
    /// Probabilidad (%) sobre el LCS y bajo el LCI.
    Probability,
    /// Valores dados.
    Preset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum MeanMethod {
    #[default]
    Auto,
    /// Media de las medias de los subgrupos.
    MeanOfMeans,
    /// Media general de todas las observaciones.
    Grand,
    /// Media de las medianas de los subgrupos.
    MeanOfMedians,
    /// Mediana de las medianas (o de las observaciones).
    Median,
    Preset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum SigmaMethod {
    #[default]
    Auto,
    /// R̄/d2.
    Rbar,
    /// R̃/d4 (mediana de los rangos).
    Rmedian,
    /// S̄/c4.
    Sbar,
    /// Desviación de todas las observaciones.
    Overall,
    /// Rango móvil medio / 1.128 (individuales).
    MovingRange,
    Preset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum FitDist {
    #[default]
    Normal,
    Lognormal,
    Exponential,
    Uniform,
    Gamma,
    Weibull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ParetoSource {
    #[default]
    Auto,
    Characteristics,
    Causes,
    Actions,
    Rules,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct QccOptions {
    pub steps: bool,
    /// Gráfico (vacío = el más adecuado a los datos).
    pub chart: Option<ChartKind>,
    /// Característica (1 = la primera).
    pub characteristic: u32,
    pub limits: Limits,
    pub k_upper: f64,
    pub k_lower: f64,
    pub prob_upper: f64,
    pub prob_lower: f64,
    pub ucl: Option<f64>,
    pub lcl: Option<f64>,
    pub mean_method: MeanMethod,
    pub mean: Option<f64>,
    pub sigma_method: SigmaMethod,
    pub sigma: Option<f64>,
    /// EWMA: peso λ; media móvil: amplitud.
    pub lambda: f64,
    pub span: u32,
    /// CUSUM: α y β (%) y cambio a detectar Δ (en desviaciones).
    pub alpha: f64,
    pub beta: f64,
    pub delta: f64,
    /// Modificado: α (%) y fracción defectuosa aceptable δ (%); de aceptación: γ (%) y β (%).
    pub mod_alpha: f64,
    pub mod_delta: f64,
    pub acc_gamma: f64,
    pub acc_beta: f64,
    pub warning2: bool,
    pub warning1: bool,
    pub check_rules: bool,
    pub histogram: bool,
    pub cells: u32,
    /// Juntar los datos más allá de ±3 desviaciones en las celdas de los extremos.
    pub combine_3sigma: bool,
    pub significance: f64,
    pub distribution: FitDist,
    pub probability_plot: bool,
    pub pareto: bool,
    pub pareto_source: ParetoSource,
}

impl Default for QccOptions {
    fn default() -> Self {
        QccOptions {
            steps: true,
            chart: None,
            characteristic: 1,
            limits: Limits::Sigma,
            k_upper: 3.0,
            k_lower: 3.0,
            prob_upper: 0.135,
            prob_lower: 0.135,
            ucl: None,
            lcl: None,
            mean_method: MeanMethod::Auto,
            mean: None,
            sigma_method: SigmaMethod::Auto,
            sigma: None,
            lambda: 0.2,
            span: 3,
            alpha: 0.27,
            beta: 10.0,
            delta: 1.0,
            mod_alpha: 0.27,
            mod_delta: 1.0,
            acc_gamma: 5.0,
            acc_beta: 10.0,
            warning2: true,
            warning1: false,
            check_rules: true,
            histogram: true,
            cells: 0,
            combine_3sigma: false,
            significance: 0.05,
            distribution: FitDist::Normal,
            probability_plot: true,
            pareto: true,
            pareto_source: ParetoSource::Auto,
        }
    }
}

pub struct Qcc;

impl TypedModule for Qcc {
    type Model = QccModel;
    type Options = QccOptions;

    fn file_tag(&self) -> &'static str {
        "QCC"
    }

    fn spec(&self) -> ModuleSpec {
        let chars_axis = |attr: bool| {
            let mut a = Axis::new("characteristics", "Característica").add("Añadir característica");
            a = a.extra(Field::number("lsl", "LSL").optional()).extra(Field::number("target", "Objetivo").optional()).extra(Field::number("usl", "USL").optional());
            if attr {
                a = a.extra(Field::number("demerit", "Peso de demérito").optional());
            }
            a
        };
        let rows_axis = |attr: bool| {
            let mut a = Axis::new("rows", if attr { "Subgrupo" } else { "Observación" }).add(if attr { "Añadir subgrupo" } else { "Añadir observación" });
            a = if attr { a.extra(Field::number("size", "Tamaño")) } else { a.extra(Field::number("subgroup", "Subgrupo")) };
            a.extra(Field::boolean("disabled", "Deshabilitado", false)).extra(Field::number("cause", "Causa").optional()).extra(Field::number("action", "Acción").optional()).extra(Field::text("comment", "Comentario").optional())
        };
        let rules = Widget::records(
            Some("Reglas de fuera de control"),
            "rules",
            vec![
                Field::text("description", "Regla"),
                Field::number("n_of", "n de"),
                Field::number("m_points", "m puntos"),
                Field::number("off_limit", "Fuera de LCS/LCI (1/−1)"),
                Field::number("sigma_off", "Desviaciones de la LC"),
                Field::number("row_cl", "Seguidos sobre/bajo la LC (1/−1)"),
                Field::number("row_median", "Seguidos sobre/bajo la mediana (1/−1)"),
                Field::number("row_updown", "Seguidos subiendo/bajando (1/−1)"),
                Field::number("jump", "Salto (desviaciones)"),
            ],
            Some("Añadir regla"),
        );
        let var_ids: Vec<&str> = CHARTS.iter().filter(|c| !c.0.attribute()).map(|c| c.1).collect();
        let att_ids: Vec<&str> = CHARTS.iter().filter(|c| c.0.attribute()).map(|c| c.1).collect();
        let all: Vec<(&str, &str)> = CHARTS.iter().map(|c| (c.1, c.2)).collect();
        let _ = (&var_ids, &att_ids);
        let cusum = ["cusum_xbar", "cusum_x", "cusum_r", "cusum_s", "cusum_p", "cusum_c"];
        ModuleSpec {
            id: "qcc".into(),
            winqsb: "QCC".into(),
            name: "Gráficos de control de calidad".into(),
            description: "36 gráficos de control, reglas, curva OC, capacidad, histograma, ji cuadrada, gráfico de probabilidad y Pareto".into(),
            file_ext: "QCC".into(),
            problem_types: vec![
                ProblemType {
                    id: "variable".into(),
                    label: "Datos por variables".into(),
                    description: "Mediciones (diámetro, peso…) en subgrupos; cada observación indica su subgrupo".into(),
                    dims: vec![Field::integer("characteristics", "Características", 1), Field::integer("subgroups", "Subgrupos", 20), Field::integer("size", "Tamaño de los subgrupos", 5)],
                    layout: vec![Widget::matrix(Some("Datos"), "data", rows_axis(false), chars_axis(false), Field::number("value", "Valor").optional()), rules.clone()],
                },
                ProblemType {
                    id: "attribute".into(),
                    label: "Datos por atributos".into(),
                    description: "Defectuosos (número o proporción) o defectos por subgrupo".into(),
                    dims: vec![Field::integer("characteristics", "Características", 1), Field::integer("subgroups", "Subgrupos", 20), Field::integer("size", "Tamaño de los subgrupos", 100)],
                    layout: vec![Widget::matrix(Some("Datos"), "data", rows_axis(true), chars_axis(true), Field::number("value", "Valor").optional()), rules],
                },
            ],
            options: vec![
                Field::boolean("steps", "Mostrar paso a paso", true),
                Field::choice("chart", "Gráfico de control", &all).optional().help("Vacío = el más adecuado a los datos"),
                Field::integer("characteristic", "Característica (número)", 1),
                Field::choice("limits", "Límites de control", &[("sigma", "k desviaciones"), ("probability", "Probabilidad"), ("preset", "Valores dados")]).default("sigma"),
                Field::number("k_upper", "k del LCS").default(3.0).show_if("limits", &["sigma"]),
                Field::number("k_lower", "k del LCI").default(3.0).show_if("limits", &["sigma"]),
                Field::number("prob_upper", "% sobre el LCS").default(0.135).show_if("limits", &["probability"]),
                Field::number("prob_lower", "% bajo el LCI").default(0.135).show_if("limits", &["probability"]),
                Field::number("ucl", "LCS").optional().show_if("limits", &["preset"]),
                Field::number("lcl", "LCI").optional().show_if("limits", &["preset"]),
                Field::choice("mean_method", "Línea central", &[("auto", "Automática"), ("mean_of_means", "Media de las medias"), ("grand", "Media general"), ("mean_of_medians", "Media de las medianas"), ("median", "Mediana"), ("preset", "Valor dado")]).default("auto"),
                Field::number("mean", "Línea central dada").optional().show_if("mean_method", &["preset"]),
                Field::choice("sigma_method", "Desviación estándar", &[("auto", "Automática"), ("rbar", "R̄/d2"), ("rmedian", "R̃/d4"), ("sbar", "S̄/c4"), ("overall", "Desviación general"), ("moving_range", "Rango móvil/1.128"), ("preset", "Valor dado")]).default("auto"),
                Field::number("sigma", "Desviación dada").optional().show_if("sigma_method", &["preset"]),
                Field::number("lambda", "Peso λ (EWMA)").default(0.2).show_if("chart", &["gma_xbar", "gma_x", "gma_p", "gma_c"]),
                Field::integer("span", "Amplitud de la media móvil", 3).show_if("chart", &["ma_xbar", "ma_x", "ma_p", "ma_c"]),
                Field::number("alpha", "CUSUM: α (%)").default(0.27).show_if("chart", &cusum),
                Field::number("beta", "CUSUM: β (%)").default(10.0).show_if("chart", &cusum),
                Field::number("delta", "CUSUM: cambio a detectar Δ (desviaciones)").default(1.0).show_if("chart", &cusum),
                Field::number("mod_alpha", "Modificado: α (%)").default(0.27).show_if("chart", &["modified_xbar", "modified_x"]),
                Field::number("mod_delta", "Modificado: fracción defectuosa aceptable δ (%)").default(1.0).show_if("chart", &["modified_xbar", "modified_x"]),
                Field::number("acc_gamma", "Aceptación: fracción defectuosa a rechazar γ (%)").default(5.0).show_if("chart", &["acceptance_xbar", "acceptance_x"]),
                Field::number("acc_beta", "Aceptación: β (%)").default(10.0).show_if("chart", &["acceptance_xbar", "acceptance_x"]),
                Field::boolean("warning2", "Mostrar líneas de 2σ", true),
                Field::boolean("warning1", "Mostrar líneas de 1σ", false),
                Field::boolean("check_rules", "Revisar las reglas de fuera de control", true),
                Field::boolean("histogram", "Histograma y prueba ji cuadrada", true),
                Field::integer("cells", "Celdas del histograma", 0).help("0 = mín(20, √n)"),
                Field::boolean("combine_3sigma", "Juntar los datos más allá de ±3σ en las celdas extremas", false),
                Field::number("significance", "Nivel de significancia").default(0.05),
                Field::choice("distribution", "Distribución a probar", &[("normal", "Normal"), ("lognormal", "Lognormal"), ("exponential", "Exponencial"), ("uniform", "Uniforme"), ("gamma", "Gamma"), ("weibull", "Weibull")]).default("normal"),
                Field::boolean("probability_plot", "Gráfico de probabilidad", true),
                Field::boolean("pareto", "Análisis de Pareto", true),
                Field::choice("pareto_source", "Pareto de", &[("auto", "Automático"), ("characteristics", "Características"), ("causes", "Causas"), ("actions", "Acciones"), ("rules", "Reglas violadas")]).default("auto").show_if("pareto", &["true"]),
            ],
        }
    }

    fn template(&self, pt: &str, dims: &Value) -> Result<QccModel, ModError> {
        let t: QccType = serde_json::from_value(Value::from(pt)).map_err(|_| ModError::Invalid(format!("tipo de problema desconocido: {pt}")))?;
        let k = dim(dims, "characteristics", 1, 1, 50)?;
        let m = dim(dims, "subgroups", 20, 1, 5000)?;
        let j = dim(dims, "size", if t == QccType::Attribute { 100 } else { 5 }, 1, 100_000)?;
        let characteristics = (1..=k).map(|i| Characteristic { name: format!("Característica {i}"), lsl: None, target: None, usl: None, demerit: (t == QccType::Attribute).then_some(1.0) }).collect();
        let (rows, data) = if t == QccType::Variable {
            let n = m * j.min(100);
            ((0..n).map(|i| DataRow { id: (i + 1).to_string(), subgroup: Some((i / j.min(100) + 1) as f64), ..Default::default() }).collect(), vec![vec![None; k]; n])
        } else {
            ((0..m).map(|i| DataRow { id: (i + 1).to_string(), size: Some(j as f64), ..Default::default() }).collect(), vec![vec![None; k]; m])
        };
        Ok(QccModel {
            name: "Control de calidad".into(),
            problem_type: t,
            horizontal: false,
            characteristics,
            rows,
            data,
            causes: (1..=5).map(|i| format!("Causa {i}")).collect(),
            actions: (1..=5).map(|i| format!("Acción {i}")).collect(),
            rules: default_rules(),
        })
    }

    fn solve(&self, m: &QccModel, o: &QccOptions) -> Result<Report, ModError> {
        tools::solve(m, o)
    }

    fn read(&self, bytes: &[u8]) -> Result<QccModel, ModError> {
        read_qcc(bytes)
    }

    fn write(&self, m: &QccModel) -> Result<Vec<u8>, ModError> {
        Ok(write_qcc(m))
    }

    fn examples(&self) -> Vec<(&'static str, QccModel)> {
        let ex = |s: &str| serde_json::from_str::<QccModel>(s).unwrap();
        vec![
            ("anillos_de_piston", ex(include_str!("ejemplos/anillos.json"))),
            ("latas_de_jugo_p", ex(include_str!("ejemplos/latas.json"))),
            ("tarjetas_de_circuito_c", ex(include_str!("ejemplos/tarjetas.json"))),
            ("defectos_pareto", ex(include_str!("ejemplos/pareto.json"))),
        ]
    }
}

// ------------------------------------------------------------------ archivos

fn opt_text(x: Option<f64>) -> String {
    x.map(num).unwrap_or_default()
}

fn flag(s: &str) -> bool {
    let t = s.trim().to_ascii_uppercase();
    t.starts_with('Y') || t.starts_with('T') || parse_num(&t).map(|v| v != 0.0).unwrap_or(false)
}

fn read_qcc(bytes: &[u8]) -> Result<QccModel, ModError> {
    let mut r = Reader::new(bytes);
    let h = r.line("la cabecera")?;
    if !h.get(0).trim().to_ascii_uppercase().starts_with("QCC") {
        return Err(ModError::Format("no es un archivo de QCC".into()));
    }
    let attr = parse_num(h.get(2)).unwrap_or(0.0) as i64 == 1;
    let horizontal = !attr && parse_num(h.get(3)).unwrap_or(0.0) as i64 == 1;
    let l2 = r.line("las dimensiones")?;
    let g = |i: usize| parse_num(l2.get(i)).unwrap_or(0.0).max(0.0) as usize;
    let (k, m, j, u, v, w) = (g(0).max(1), g(1), g(2), g(3), g(4), g(5));
    let l3 = r.line("el tamaño de la tabla")?;
    let nrows = parse_num(l3.get(0)).unwrap_or(0.0).max(1.0) as usize;
    let _ncols = parse_num(l3.get(1)).unwrap_or(0.0) as usize;
    let head = r.line("la fila de títulos")?;
    let mut model = QccModel {
        name: Some(h.get(1).trim()).filter(|s| !s.is_empty()).unwrap_or("QCC Problem").to_string(),
        problem_type: if attr { QccType::Attribute } else { QccType::Variable },
        horizontal,
        characteristics: vec![],
        rows: vec![],
        data: vec![],
        causes: vec![],
        actions: vec![],
        rules: vec![],
    };
    let num_opt = |s: &str| parse_num(s);
    let mut names: Vec<String> = vec![];
    if attr {
        // Subgroup, Date, Time, Size, char1..k, Disabled, Cause, Action, Comment
        names = (0..k).map(|c| head.get(4 + c).trim().to_string()).collect();
        for _ in 0..nrows.saturating_sub(1) {
            let l = r.line("un subgrupo")?;
            model.rows.push(DataRow {
                id: l.get(0).trim().to_string(),
                subgroup: None,
                size: num_opt(l.get(3)),
                date: l.get(1).trim().to_string(),
                time: l.get(2).trim().to_string(),
                disabled: flag(l.get(4 + k)),
                cause: num_opt(l.get(5 + k)),
                action: num_opt(l.get(6 + k)),
                comment: l.get(7 + k).trim().to_string(),
            });
            model.data.push((0..k).map(|c| num_opt(l.get(4 + c))).collect());
        }
    } else if !horizontal {
        // Number, Date, Time, Subgroup, char1..k, Disabled, Cause, Action, Comment
        names = (0..k).map(|c| head.get(4 + c).trim().to_string()).collect();
        for _ in 0..nrows.saturating_sub(1) {
            let l = r.line("una observación")?;
            model.rows.push(DataRow {
                id: l.get(0).trim().to_string(),
                subgroup: num_opt(l.get(3)),
                size: None,
                date: l.get(1).trim().to_string(),
                time: l.get(2).trim().to_string(),
                disabled: flag(l.get(4 + k)),
                cause: num_opt(l.get(5 + k)),
                action: num_opt(l.get(6 + k)),
                comment: l.get(7 + k).trim().to_string(),
            });
            model.data.push((0..k).map(|c| num_opt(l.get(4 + c))).collect());
        }
    } else {
        // Subgroup, Characteristic, Date, Time, Obs1..j, Disabled, Cause, Action, Comment (k filas por subgrupo)
        let mut obs: Vec<(f64, usize, Vec<Option<f64>>, DataRow)> = vec![];
        let mut char_names: Vec<String> = vec![];
        for _ in 0..nrows.saturating_sub(1) {
            let l = r.line("un subgrupo")?;
            let sg = num_opt(l.get(0)).unwrap_or((obs.len() / k + 1) as f64);
            let cname = l.get(1).trim().to_string();
            let ci = match char_names.iter().position(|c| *c == cname) {
                Some(i) => i,
                None => {
                    char_names.push(cname);
                    char_names.len() - 1
                }
            };
            let vals: Vec<Option<f64>> = (0..j).map(|o| num_opt(l.get(4 + o))).collect();
            let info = DataRow { date: l.get(2).trim().to_string(), time: l.get(3).trim().to_string(), disabled: flag(l.get(4 + j)), cause: num_opt(l.get(5 + j)), action: num_opt(l.get(6 + j)), comment: l.get(7 + j).trim().to_string(), ..Default::default() };
            obs.push((sg, ci, vals, info));
        }
        names = char_names.clone();
        let kk = names.len().max(1);
        // a formato vertical: una fila por (subgrupo, observación)
        let mut sgs: Vec<f64> = vec![];
        for o in &obs {
            if !sgs.contains(&o.0) {
                sgs.push(o.0);
            }
        }
        let mut id = 0;
        for sg in sgs {
            for t in 0..j {
                id += 1;
                let mut vals = vec![None; kk];
                let mut info = DataRow::default();
                for o in obs.iter().filter(|o| o.0 == sg) {
                    vals[o.1] = o.2.get(t).cloned().flatten();
                    if o.1 == 0 {
                        info = o.3.clone();
                    }
                }
                info.id = id.to_string();
                info.subgroup = Some(sg);
                model.rows.push(info);
                model.data.push(vals);
            }
        }
    }
    let _ = m;
    // nombres de las características
    let mut cn: Vec<String> = vec![];
    for _ in 0..k {
        if let Some(l) = r.try_line() {
            cn.push(l.get(1).trim().to_string());
        }
    }
    for _ in 0..u {
        if let Some(l) = r.try_line() {
            model.causes.push(l.get(1).trim().to_string());
        }
    }
    for _ in 0..v {
        if let Some(l) = r.try_line() {
            model.actions.push(l.get(1).trim().to_string());
        }
    }
    for _ in 0..w {
        if let Some(l) = r.try_line() {
            let f = |i: usize| parse_num(l.get(i)).unwrap_or(0.0);
            let rule = Rule { description: l.get(1).trim().to_string(), n_of: f(2), m_points: f(3), off_limit: f(4), sigma_off: f(5), row_cl: f(6), row_median: f(7), row_updown: f(8), jump: f(9) };
            model.rules.push(rule);
        }
    }
    let kk = model.data.first().map(|r| r.len()).unwrap_or(k);
    for c in 0..kk {
        let name = cn.get(c).filter(|s| !s.is_empty()).cloned().or_else(|| names.get(c).cloned()).unwrap_or(format!("Characteristic {}", c + 1));
        let (mut lsl, mut tg, mut usl, mut dm) = (None, None, None, None);
        if let Some(l) = r.try_line() {
            let (a, b, cc, d) = (parse_num(l.get(0)), parse_num(l.get(1)), parse_num(l.get(2)), parse_num(l.get(3)));
            // 0, 0, 0 = sin especificación
            let none = a.unwrap_or(0.0) == 0.0 && cc.unwrap_or(0.0) == 0.0;
            if !none {
                lsl = a;
                tg = b;
                usl = cc;
            }
            dm = d;
        }
        model.characteristics.push(Characteristic { name, lsl, target: tg, usl, demerit: dm.filter(|_| attr) });
    }
    Ok(model)
}

fn write_qcc(m: &QccModel) -> Vec<u8> {
    let mut w = Writer::new();
    let attr = m.problem_type == QccType::Attribute;
    let k = m.characteristics.len();
    let horizontal = m.horizontal && !attr;
    // subgrupos (en orden de aparición)
    let mut sgs: Vec<f64> = vec![];
    for r in &m.rows {
        let s = r.subgroup.unwrap_or(0.0);
        if !sgs.contains(&s) {
            sgs.push(s);
        }
    }
    let size = if attr { m.rows.first().and_then(|r| r.size).unwrap_or(1.0) as usize } else { sgs.iter().map(|s| m.rows.iter().filter(|r| r.subgroup.unwrap_or(0.0) == *s).count()).max().unwrap_or(1) };
    let nsub = if attr { m.rows.len() } else { sgs.len() };
    w.row(&["QCC".to_string(), m.name.clone(), if attr { "1" } else { "0" }.into(), if horizontal { "1" } else { "0" }.into()]);
    w.row(&[k.to_string(), nsub.to_string(), size.to_string(), m.causes.len().to_string(), m.actions.len().to_string(), m.rules.len().to_string()]);
    let yes = |b: bool| if b { "1".to_string() } else { String::new() };
    let val = |i: usize, c: usize| m.data.get(i).and_then(|r| r.get(c)).cloned().flatten();
    if attr {
        w.row(&[(m.rows.len() + 1).to_string(), (8 + k).to_string()]);
        let mut h = vec!["Subgroup".to_string(), "Date".into(), "Time".into(), "Size".into()];
        h.extend(m.characteristics.iter().map(|c| c.name.clone()));
        h.extend(["Disabled".to_string(), "Cause Code".into(), "Action Code".into(), "Comment".into()]);
        w.row(&h);
        for (i, r) in m.rows.iter().enumerate() {
            let mut row = vec![if r.id.is_empty() { (i + 1).to_string() } else { r.id.clone() }, r.date.clone(), r.time.clone(), opt_text(r.size)];
            row.extend((0..k).map(|c| opt_text(val(i, c))));
            row.extend([yes(r.disabled), opt_text(r.cause), opt_text(r.action), r.comment.clone()]);
            w.row(&row);
        }
    } else if !horizontal {
        w.row(&[(m.rows.len() + 1).to_string(), (8 + k).to_string()]);
        let mut h = vec!["Number".to_string(), "Date".into(), "Time".into(), "Subgroup".into()];
        h.extend(m.characteristics.iter().map(|c| c.name.clone()));
        h.extend(["Disabled".to_string(), "Cause Code".into(), "Action Code".into(), "Comment".into()]);
        w.row(&h);
        for (i, r) in m.rows.iter().enumerate() {
            let mut row = vec![if r.id.is_empty() { (i + 1).to_string() } else { r.id.clone() }, r.date.clone(), r.time.clone(), opt_text(r.subgroup)];
            row.extend((0..k).map(|c| opt_text(val(i, c))));
            row.extend([yes(r.disabled), opt_text(r.cause), opt_text(r.action), r.comment.clone()]);
            w.row(&row);
        }
    } else {
        w.row(&[(nsub * k + 1).to_string(), (8 + size).to_string()]);
        let mut h = vec!["Subgroup".to_string(), "Characteristics".into(), "Date".into(), "Time".into()];
        h.extend((1..=size).map(|o| format!("Observation {o}")));
        h.extend(["Disabled".to_string(), "Cause Code".into(), "Action Code".into(), "Comment".into()]);
        w.row(&h);
        for s in &sgs {
            let idx: Vec<usize> = (0..m.rows.len()).filter(|&i| m.rows[i].subgroup.unwrap_or(0.0) == *s).collect();
            let first = &m.rows[idx[0]];
            for c in 0..k {
                let mut row = vec![num(*s), m.characteristics[c].name.clone(), first.date.clone(), first.time.clone()];
                row.extend((0..size).map(|t| idx.get(t).map(|&i| opt_text(val(i, c))).unwrap_or_default()));
                row.extend([yes(idx.iter().all(|&i| m.rows[i].disabled)), opt_text(first.cause), opt_text(first.action), first.comment.clone()]);
                w.row(&row);
            }
        }
    }
    for (i, c) in m.characteristics.iter().enumerate() {
        w.row(&[(i + 1).to_string(), c.name.clone()]);
    }
    for (i, c) in m.causes.iter().enumerate() {
        w.row(&[(i + 1).to_string(), c.clone()]);
    }
    for (i, c) in m.actions.iter().enumerate() {
        w.row(&[(i + 1).to_string(), c.clone()]);
    }
    for (i, r) in m.rules.iter().enumerate() {
        w.row(&[(i + 1).to_string(), r.description.clone(), num(r.n_of), num(r.m_points), num(r.off_limit), num(r.sigma_off), num(r.row_cl), num(r.row_median), num(r.row_updown), num(r.jump)]);
    }
    for c in &m.characteristics {
        let z = |x: Option<f64>| num(x.unwrap_or(0.0));
        w.row(&[z(c.lsl), z(c.target), z(c.usl), z(c.demerit)]);
    }
    w.bytes()
}

pub(crate) fn json_rows(v: &[(String, f64)]) -> Value {
    json!(v.iter().map(|(a, b)| json!({"name": a, "value": tidy(*b)})).collect::<Vec<_>>())
}
