//! INV · Gestión de inventarios (no existe en WinQSB; es el tema de inventarios
//! de clase). Cuatro herramientas:
//!
//! - Despejes del EOQ y del POQ: a partir de lo que se observa (la duración del
//!   ciclo, el ritmo al que crece el inventario, el inventario máximo...) se
//!   despejan los demás datos en cadena, como en los problemas "al revés".
//! - Clasificación ABC (Pareto), con cortes por quiebre de pendiente o por
//!   porcentaje acumulado.
//! - Punto de reorden con demanda variable: sistema continuo R = d·L + Z·σ·√L y
//!   periódico I = d(T + L) + Z·σ·√(T + L).
//! - Simulación día a día de la política (Q, R) con ventas perdidas o con
//!   pedidos pendientes (back orders), con escenarios y búsqueda de la mejor
//!   combinación.

mod abc;
mod despejes;
mod qr;
mod reorden;

use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum InvType {
    #[default]
    Despejes,
    Abc,
    Reorden,
    QrSim,
}

/// Un valor de una serie (ventas de un día, histórico...).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct Val {
    #[serde(default)]
    pub value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct AbcItem {
    #[serde(default)]
    pub name: String,
    /// Precio (o costo) unitario.
    #[serde(default)]
    pub price: Option<f64>,
    /// Cantidad vendida (o usada) en el período.
    #[serde(default)]
    pub quantity: Option<f64>,
    /// Valor del período, si no se dan precio y cantidad.
    #[serde(default)]
    pub value: Option<f64>,
}

/// Un escenario de la política (Q, R).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct Scenario {
    #[serde(default)]
    pub name: String,
    /// Nivel de servicio (%) para calcular R.
    #[serde(default)]
    pub service_level: Option<f64>,
    /// Tiempo de entrega del proveedor (días).
    #[serde(default)]
    pub lead_time: Option<f64>,
    /// Q como múltiplo de la demanda promedio (p. ej. 6 = seis veces el promedio).
    #[serde(default)]
    pub q_factor: Option<f64>,
    /// Q fijo (si se da, manda sobre el múltiplo).
    #[serde(default)]
    pub quantity: Option<f64>,
    /// R fijo (si se da, no se calcula con el nivel de servicio).
    #[serde(default)]
    pub reorder_point: Option<f64>,
    /// Aceptar pedidos pendientes en este escenario (vacío = lo del modelo).
    #[serde(default)]
    pub backorders: Option<bool>,
}

fn d360() -> f64 {
    360.0
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InvModel {
    #[serde(default)]
    pub name: String,
    pub problem_type: InvType,

    // ----- despejes (todos opcionales: se despejan los que falten)
    /// "eoq" (el lote llega completo) o "poq" (entregas parciales).
    #[serde(default)]
    pub model_kind: Option<String>,
    /// Días del año.
    #[serde(default = "d360")]
    pub days_per_year: f64,
    #[serde(default)]
    pub annual_demand: Option<f64>,
    #[serde(default)]
    pub daily_demand: Option<f64>,
    /// Tasa de entrega o producción por día (p).
    #[serde(default)]
    pub delivery_rate: Option<f64>,
    /// Ritmo al que crece el inventario mientras llega el lote (p − d).
    #[serde(default)]
    pub growth_rate: Option<f64>,
    #[serde(default)]
    pub quantity: Option<f64>,
    #[serde(default)]
    pub max_inventory: Option<f64>,
    /// Días de entregas (Tp).
    #[serde(default)]
    pub delivery_days: Option<f64>,
    /// Días de agotamiento (Td).
    #[serde(default)]
    pub depletion_days: Option<f64>,
    /// Duración del ciclo en días (T).
    #[serde(default)]
    pub cycle_days: Option<f64>,
    /// Pedidos o corridas por año (N).
    #[serde(default)]
    pub orders_per_year: Option<f64>,
    /// Costo de ordenar (S).
    #[serde(default)]
    pub setup_cost: Option<f64>,
    /// Costo de mantener por unidad y año (H).
    #[serde(default)]
    pub holding_cost: Option<f64>,
    /// Tasa de mantener (% anual) y costo unitario, para H = i·C.
    #[serde(default)]
    pub holding_rate: Option<f64>,
    #[serde(default)]
    pub unit_cost: Option<f64>,
    /// El lote observado es el óptimo (costo de ordenar = costo de mantener).
    #[serde(default)]
    pub is_optimal: bool,

    // ----- ABC
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<AbcItem>,

    // ----- punto de reorden y simulación
    /// Ventas históricas (para calcular la demanda promedio y su desviación).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<Val>,
    /// Demanda promedio por día (si no hay histórico).
    #[serde(default)]
    pub mean_demand: Option<f64>,
    /// Desviación estándar de la demanda diaria (si no hay histórico).
    #[serde(default)]
    pub std_demand: Option<f64>,
    /// Tiempo de entrega (días).
    #[serde(default)]
    pub lead_time: Option<f64>,
    /// Nivel de servicio (%).
    #[serde(default)]
    pub service_level: Option<f64>,
    /// "continuo" o "periodico".
    #[serde(default)]
    pub review: Option<String>,
    /// Sistema periódico: días entre revisiones (T) y existencias al revisar.
    #[serde(default)]
    pub review_days: Option<f64>,
    #[serde(default)]
    pub on_hand: Option<f64>,

    /// Pronóstico de ventas día a día (simulación).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forecast: Vec<Val>,
    #[serde(default)]
    pub initial_inventory: f64,
    /// Costo de almacenar por unidad (sobre el inventario promedio).
    #[serde(default)]
    pub unit_holding_cost: f64,
    /// Costo de inexistencia por unidad no vendida.
    #[serde(default)]
    pub shortage_cost: f64,
    /// Costo por unidad en pedido pendiente (si se aceptan).
    #[serde(default)]
    pub backorder_cost: f64,
    /// Se aceptan pedidos pendientes (back orders).
    #[serde(default)]
    pub backorders: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct InvOptions {
    pub steps: bool,
    /// ABC: "quiebres" (las dos mayores caídas) o "porcentajes".
    pub abc_method: String,
    /// ABC por porcentajes: % acumulado donde terminan A y B.
    pub a_cut: f64,
    pub b_cut: f64,
    /// Valor Z: "exacto" (como DISTR.NORM.ESTAND.INV de Excel) o "tabla" (dos decimales).
    pub z_mode: String,
    /// Redondear Q y R al entero más cercano (como en clase).
    pub round: bool,
    /// Simulación: buscar la mejor combinación (Q, R).
    pub search: bool,
    /// Búsqueda: tiempo de entrega mínimo (días).
    pub min_lead: f64,
}

impl Default for InvOptions {
    fn default() -> Self {
        InvOptions { steps: true, abc_method: "quiebres".into(), a_cut: 80.0, b_cut: 95.0, z_mode: "exacto".into(), round: true, search: true, min_lead: 2.0 }
    }
}

/// Valor Z para un nivel de servicio (%), exacto o redondeado como en la tabla.
pub(crate) fn z_value(service: f64, mode: &str) -> Result<f64, ModError> {
    use statrs::distribution::{ContinuousCDF, Normal};
    if !(service > 0.0 && service < 100.0) {
        return invalid("el nivel de servicio debe estar entre 0 y 100 %");
    }
    let z = Normal::new(0.0, 1.0).unwrap().inverse_cdf(service / 100.0);
    Ok(if mode == "tabla" { (z * 100.0).round() / 100.0 } else { z })
}

/// Promedio y desviación estándar muestral (como PROMEDIO y DESVEST de Excel).
pub(crate) fn mean_sd(v: &[f64]) -> (f64, f64) {
    let n = v.len() as f64;
    let m = v.iter().sum::<f64>() / n;
    let s = if v.len() > 1 { (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1.0)).sqrt() } else { 0.0 };
    (m, s)
}

pub(crate) fn values(v: &[Val]) -> Vec<f64> {
    v.iter().filter_map(|x| x.value).collect()
}

pub struct Inv;

impl TypedModule for Inv {
    type Model = InvModel;
    type Options = InvOptions;

    fn file_tag(&self) -> &'static str {
        "IAQSB-INV"
    }

    fn spec(&self) -> ModuleSpec {
        let pt = |id: &str, label: &str, desc: &str, layout: Vec<Widget>, dims: Vec<Field>| ProblemType { id: id.into(), label: label.into(), description: desc.into(), dims, layout };
        let opt = |k: &str, l: &str| Field::number(k, l).optional();
        ModuleSpec {
            id: "inv".into(),
            winqsb: "Nuevo".into(),
            name: "Gestión de inventarios".into(),
            description: "Despejes del EOQ y del POQ, clasificación ABC, punto de reorden con demanda variable y simulación de la política (Q, R) día a día".into(),
            file_ext: "INV".into(),
            problem_types: vec![
                pt(
                    "despejes",
                    "EOQ y POQ al revés (despejes)",
                    "Se dan los datos que se conocen (ciclo, ritmo de crecimiento, inventario máximo...) y se despejan los demás en cadena",
                    vec![
                        Widget::params(
                            Some("Modelo"),
                            vec![
                                Field::choice("model_kind", "El lote", &[("poq", "Entra poco a poco (POQ, entregas parciales)"), ("eoq", "Llega completo (EOQ)")]),
                                Field::number("days_per_year", "Días del año").default(360.0),
                                Field::boolean("is_optimal", "El lote es el óptimo (costo de ordenar = costo de mantener)", false),
                            ],
                        ),
                        Widget::params(
                            Some("Lo que se conoce (deja vacío lo que no)"),
                            vec![
                                opt("annual_demand", "Demanda anual (D)"),
                                opt("daily_demand", "Demanda diaria (d)"),
                                opt("delivery_rate", "Tasa de entrega por día (p)").show_if("model_kind", &["poq"]),
                                opt("growth_rate", "Ritmo al que crece el inventario (p − d)").show_if("model_kind", &["poq"]),
                                opt("quantity", "Tamaño del lote (Q)"),
                                opt("max_inventory", "Inventario máximo (Imax)").show_if("model_kind", &["poq"]),
                                opt("delivery_days", "Días de entregas (Tp)").show_if("model_kind", &["poq"]),
                                opt("depletion_days", "Días de agotamiento (Td)").show_if("model_kind", &["poq"]),
                                opt("cycle_days", "Duración del ciclo en días (T)"),
                                opt("orders_per_year", "Pedidos al año (N)"),
                                opt("setup_cost", "Costo de ordenar (S)"),
                                opt("holding_cost", "Costo de mantener por unidad-año (H)"),
                                opt("holding_rate", "o tasa de mantener (% anual)"),
                                opt("unit_cost", "y costo unitario (C)"),
                            ],
                        ),
                    ],
                    vec![],
                ),
                pt(
                    "abc",
                    "Clasificación ABC (Pareto)",
                    "Ordena los artículos por su participación en las ventas y los agrupa en A, B y C",
                    vec![Widget::records(
                        Some("Artículos"),
                        "items",
                        vec![Field::text("name", "Artículo"), opt("price", "Precio unitario"), opt("quantity", "Cantidad vendida"), opt("value", "o valor del período")],
                        Some("Añadir artículo"),
                    )],
                    vec![Field::integer("items", "Número de artículos", 10)],
                ),
                pt(
                    "reorden",
                    "Punto de reorden con demanda variable",
                    "Inventario de seguridad y punto de reorden (sistema continuo) o inventario objetivo (sistema periódico)",
                    vec![
                        Widget::params(
                            Some("Demanda y servicio"),
                            vec![
                                Field::choice("review", "Sistema", &[("continuo", "Continuo (Q, R)"), ("periodico", "Periódico (T, I)")]),
                                opt("mean_demand", "Demanda promedio por día (d)").help("Vacío = se calcula con el histórico"),
                                opt("std_demand", "Desviación estándar diaria (σ)").help("Vacío = se calcula con el histórico"),
                                opt("lead_time", "Tiempo de entrega en días (L)"),
                                opt("service_level", "Nivel de servicio (%)"),
                                opt("review_days", "Días entre revisiones (T)").show_if("review", &["periodico"]),
                                opt("on_hand", "Existencias al revisar (e)").show_if("review", &["periodico"]),
                            ],
                        ),
                        Widget::records(Some("Histórico de ventas (opcional)"), "history", vec![opt("value", "Ventas del día")], Some("Añadir día")),
                    ],
                    vec![],
                ),
                pt(
                    "qr_sim",
                    "Simulación de la política (Q, R)",
                    "Tabla día a día con el inventario en bodega, en tránsito y la posición; ventas perdidas o pendientes; costos y gráfica",
                    vec![
                        Widget::params(
                            Some("Datos"),
                            vec![
                                opt("mean_demand", "Demanda promedio diaria (d)").help("Vacío = se calcula con el histórico"),
                                opt("std_demand", "Desviación estándar diaria (σ)").help("Vacío = se calcula con el histórico"),
                                Field::number("initial_inventory", "Inventario al inicio del día 1"),
                                Field::number("unit_holding_cost", "Costo de almacenar por unidad").help("Se multiplica por el inventario promedio en bodega"),
                                Field::number("shortage_cost", "Costo de inexistencia por unidad"),
                                Field::boolean("backorders", "Se aceptan pedidos pendientes (back orders)", false),
                                Field::number("backorder_cost", "Costo por unidad pendiente").show_if("backorders", &["true"]),
                            ],
                        ),
                        Widget::records(
                            Some("Escenarios"),
                            "scenarios",
                            vec![
                                Field::text("name", "Escenario"),
                                opt("service_level", "Nivel de servicio (%)"),
                                opt("lead_time", "Entrega (días)"),
                                opt("q_factor", "Q = k × promedio"),
                                opt("quantity", "o Q fijo"),
                                opt("reorder_point", "o R fijo"),
                            ],
                            Some("Añadir escenario"),
                        ),
                        Widget::records(Some("Pronóstico de ventas"), "forecast", vec![opt("value", "Ventas del día")], Some("Añadir día")),
                        Widget::records(Some("Histórico de ventas (opcional)"), "history", vec![opt("value", "Ventas del día")], Some("Añadir día")),
                    ],
                    vec![Field::integer("days", "Días del pronóstico", 15)],
                ),
            ],
            options: vec![
                Field::boolean("steps", "Mostrar paso a paso", true),
                Field::choice("abc_method", "Cortes", &[("quiebres", "Por quiebre de pendiente"), ("porcentajes", "Por % acumulado")]).comparable().show_if("problem_type", &["abc"]),
                Field::number("a_cut", "A hasta (% acumulado)").default(80.0).show_if("problem_type", &["abc"]).show_if("abc_method", &["porcentajes"]),
                Field::number("b_cut", "B hasta (% acumulado)").default(95.0).show_if("problem_type", &["abc"]).show_if("abc_method", &["porcentajes"]),
                Field::choice("z_mode", "Valor Z", &[("exacto", "Exacto (como Excel)"), ("tabla", "De la tabla (dos decimales)")]).comparable().show_if("problem_type", &["reorden", "qr_sim"]),
                Field::boolean("round", "Redondear Q y R al entero", true).show_if("problem_type", &["qr_sim"]),
                Field::boolean("search", "Buscar la mejor combinación (Q, R)", true).show_if("problem_type", &["qr_sim"]),
                Field::number("min_lead", "Búsqueda: entrega mínima (días)").default(2.0).show_if("problem_type", &["qr_sim"]).show_if("search", &["true"]),
            ],
        }
    }

    fn template(&self, pt: &str, dims: &Value) -> Result<InvModel, ModError> {
        let t: InvType = serde_json::from_value(Value::from(pt)).map_err(|_| ModError::Invalid(format!("tipo de problema desconocido: {pt}")))?;
        let mut m: InvModel = serde_json::from_value(serde_json::json!({"name": "Inventario nuevo", "problem_type": t})).unwrap();
        match t {
            InvType::Despejes => m.model_kind = Some("poq".into()),
            InvType::Abc => {
                let n = dim(dims, "items", 10, 1, 5000)?;
                m.items = (1..=n).map(|i| AbcItem { name: i.to_string(), ..Default::default() }).collect();
            }
            InvType::Reorden => {
                m.review = Some("continuo".into());
                m.service_level = Some(95.0);
            }
            InvType::QrSim => {
                let n = dim(dims, "days", 15, 1, 3650)?;
                m.forecast = vec![Val { value: Some(0.0) }; n];
                m.scenarios = vec![Scenario { name: "A".into(), service_level: Some(75.0), lead_time: Some(5.0), q_factor: Some(6.0), ..Default::default() }];
            }
        }
        Ok(m)
    }

    fn solve(&self, m: &InvModel, o: &InvOptions) -> Result<Report, ModError> {
        let mut r = match m.problem_type {
            InvType::Despejes => despejes::solve(m, o)?,
            InvType::Abc => abc::solve(m, o)?,
            InvType::Reorden => reorden::solve(m, o)?,
            InvType::QrSim => qr::solve(m, o)?,
        };
        if r.problem.is_empty() {
            r.problem = m.name.clone();
        }
        Ok(r)
    }

    fn read(&self, bytes: &[u8]) -> Result<InvModel, ModError> {
        let s = String::from_utf8_lossy(bytes);
        let s = s.trim_start_matches('\u{feff}');
        let (head, rest) = s.split_once('\n').unwrap_or((s, ""));
        if !head.trim().to_ascii_uppercase().starts_with("IAQSB-INV") {
            return Err(ModError::Format("no es un archivo de gestión de inventarios de IAQSB".into()));
        }
        serde_json::from_str(rest).map_err(|e| ModError::Format(e.to_string()))
    }

    fn write(&self, m: &InvModel) -> Result<Vec<u8>, ModError> {
        Ok(format!("IAQSB-INV 1\n{}\n", serde_json::to_string_pretty(m).unwrap()).into_bytes())
    }

    fn examples(&self) -> Vec<(&'static str, InvModel)> {
        let ex = |s: &str| serde_json::from_str::<InvModel>(s).unwrap();
        vec![
            ("parcial_poq_entregas", ex(include_str!("ejemplos/parcial_poq.json"))),
            ("parcial_eoq_grafica", ex(include_str!("ejemplos/parcial_eoq.json"))),
            ("poq_tarea", ex(include_str!("ejemplos/poq_tarea.json"))),
            ("abc_15_articulos", ex(include_str!("ejemplos/abc.json"))),
            ("sell_rite", ex(include_str!("ejemplos/sell_rite.json"))),
            ("taller2_qr", ex(include_str!("ejemplos/taller2_qr.json"))),
        ]
    }
}
