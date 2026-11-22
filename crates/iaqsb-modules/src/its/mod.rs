//! ITS · Teoría y sistemas de inventario (equivalente al módulo ITS de WinQSB).
//!
//! Los ocho tipos de problema de WinQSB:
//! - EOQ determinístico (con faltantes planeados, producción gradual y tiempo de entrega).
//! - Descuentos por cantidad (sobre todas las unidades o incrementales).
//! - Período único con demanda aleatoria (vendedor de periódicos).
//! - Tamaño de lote con demanda dinámica: diez métodos, Wagner-Whitin el óptimo.
//! - Sistemas de revisión continua (s, Q) y (s, S) y de revisión periódica
//!   (R, S) y (R, s, S), con solución óptima, valores conocidos o nivel de
//!   servicio, y simulación.

mod det;
mod lot;
mod newsboy;
mod stoch;

use crate::pert::dist::{self, Dist, Law};
use iaqsb_core::linear::Bnd;
use iaqsb_core::winqsb::{num, parse_num, Reader, Writer};
use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub enum ItsType {
    #[default]
    #[serde(rename = "eoq")]
    Eoq,
    #[serde(rename = "discount")]
    Discount,
    #[serde(rename = "newsboy")]
    Newsboy,
    #[serde(rename = "lot_sizing")]
    LotSizing,
    #[serde(rename = "sq")]
    SQ,
    #[serde(rename = "ss")]
    SS,
    #[serde(rename = "rs")]
    RS,
    #[serde(rename = "rss")]
    RSS,
}

impl ItsType {
    pub const ALL: [ItsType; 8] = [ItsType::Eoq, ItsType::Discount, ItsType::Newsboy, ItsType::LotSizing, ItsType::SQ, ItsType::SS, ItsType::RS, ItsType::RSS];
    pub fn code(self) -> usize {
        ItsType::ALL.iter().position(|t| *t == self).unwrap()
    }
    pub fn system(self) -> bool {
        matches!(self, ItsType::SQ | ItsType::SS | ItsType::RS | ItsType::RSS)
    }
}

/// Una distribución de probabilidad con sus parámetros (en el orden de WinQSB).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DistIn {
    pub distribution: Dist,
    #[serde(default)]
    pub p1: Option<f64>,
    #[serde(default)]
    pub p2: Option<f64>,
    #[serde(default)]
    pub p3: Option<f64>,
    /// Valores discretos "v/p, v/p, …".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<String>,
}

impl DistIn {
    pub fn normal(m: f64, s: f64) -> DistIn {
        DistIn { distribution: Dist::Normal, p1: Some(m), p2: Some(s), p3: None, values: None }
    }
    pub fn constant(v: f64) -> DistIn {
        DistIn { distribution: Dist::Constant, p1: Some(v), p2: None, p3: None, values: None }
    }
    pub fn law(&self, what: &str) -> Result<Law, ModError> {
        if matches!(self.distribution, Dist::ThreeTime | Dist::Mixed) {
            return invalid(format!("{what}: esa distribución no se usa en inventarios"));
        }
        Law::new(self.distribution, [self.p1, self.p2, self.p3], self.values.as_deref(), what)
    }
}

impl ItsModel {
    /// Lleva los datos "de clase" (días del año, tasa diaria, H = i·C, precio por
    /// rango) a los de WinQSB, que es como se resuelve y se guarda.
    pub fn normalized(&self) -> Result<ItsModel, ModError> {
        let mut m = self.clone();
        if let Some(dpy) = m.days_per_year {
            if dpy <= 0.0 {
                return invalid("los días del año deben ser positivos");
            }
            m.lead_time = self.lead_time / dpy;
            if let Some(p) = m.production_per_day.filter(|p| *p > 0.0) {
                m.production_rate = iaqsb_core::linear::Bnd(p * dpy);
            }
        } else if m.production_per_day.is_some_and(|p| p > 0.0) {
            return invalid("para usar la tasa de producción por día indica los días del año");
        }
        if let Some(r) = m.holding_rate.filter(|r| *r > 0.0) {
            if m.unit_cost <= 0.0 {
                return invalid("para calcular H = i·C indica el costo unitario");
            }
            m.holding_cost = r / 100.0 * m.unit_cost;
            m.discount_holding = true;
        }
        for b in m.breaks.iter_mut() {
            if let Some(pr) = b.price {
                if m.unit_cost <= 0.0 {
                    return invalid("con precios por rango indica el costo unitario del primer rango");
                }
                if pr <= 0.0 || pr > m.unit_cost {
                    return invalid(format!("el precio desde {} unidades debe estar entre 0 y el costo unitario sin descuento", fmt(b.quantity)));
                }
                b.discount = (1.0 - pr / m.unit_cost) * 100.0;
            }
        }
        Ok(m)
    }
}

impl Default for DistIn {
    fn default() -> Self {
        DistIn::normal(0.0, 0.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Break {
    /// Cantidad a partir de la cual se aplica el descuento.
    pub quantity: f64,
    /// Descuento en %.
    #[serde(default)]
    pub discount: f64,
    /// Precio unitario desde esta cantidad (en lugar del descuento en %).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LotPeriod {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub demand: f64,
    #[serde(default)]
    pub setup_cost: f64,
    #[serde(default)]
    pub unit_cost: f64,
    /// Costo de mantener una unidad de este período al siguiente.
    #[serde(default)]
    pub holding_cost: f64,
    /// Costo por unidad atrasada un período ("M" = no se permiten atrasos).
    #[serde(default = "m_inf")]
    pub backorder_cost: Bnd,
}

fn m_inf() -> Bnd {
    Bnd::INF
}
fn hundred() -> f64 {
    100.0
}
fn one() -> f64 {
    1.0
}
fn year() -> String {
    "año".into()
}
fn const_zero() -> DistIn {
    DistIn::constant(0.0)
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ItsModel {
    #[serde(default)]
    pub name: String,
    pub problem_type: ItsType,
    /// Unidad de tiempo (año, mes, semana, día…).
    #[serde(default = "year")]
    pub time_unit: String,

    // --- EOQ y descuentos
    /// Demanda por unidad de tiempo.
    #[serde(default)]
    pub demand: f64,
    /// Costo de ordenar o de preparación, por pedido.
    #[serde(default)]
    pub setup_cost: f64,
    /// Costo de mantener una unidad por unidad de tiempo.
    #[serde(default)]
    pub holding_cost: f64,
    /// Costo de faltante por unidad y unidad de tiempo ("M" = no se permiten faltantes).
    #[serde(default = "m_inf")]
    pub shortage_cost: Bnd,
    /// Costo de faltante por unidad, independiente del tiempo.
    #[serde(default)]
    pub shortage_cost_fixed: f64,
    /// Tasa de reabastecimiento o producción ("M" = instantáneo).
    #[serde(default = "m_inf")]
    pub production_rate: Bnd,
    /// Tiempo de entrega de un pedido (en días si se indican los días del año).
    #[serde(default)]
    pub lead_time: f64,
    /// Días hábiles del año (p. ej. 250): el tiempo de entrega se da en días y
    /// los resultados se muestran también por día.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub days_per_year: Option<f64>,
    /// Tasa de producción por día (con los días del año), en lugar de la tasa anual.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub production_per_day: Option<f64>,
    /// Costo de mantener como % anual del costo unitario (H = i·C); con
    /// descuentos, cada precio tiene su propio H.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holding_rate: Option<f64>,
    /// Costo unitario de adquisición (sin descuento).
    #[serde(default)]
    pub unit_cost: f64,
    /// Cantidad a evaluar además de la óptima.
    #[serde(default)]
    pub known_quantity: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub breaks: Vec<Break>,
    /// Descuento incremental (si no, se aplica a todas las unidades).
    #[serde(default)]
    pub incremental: bool,
    #[serde(default)]
    pub discount_holding: bool,
    #[serde(default)]
    pub discount_shortage: bool,
    #[serde(default)]
    pub discount_shortage_fixed: bool,

    // --- demanda aleatoria
    #[serde(default)]
    pub demand_dist: DistIn,
    #[serde(default)]
    pub selling_price: f64,
    /// Costo de oportunidad por unidad faltante (además de la ganancia perdida).
    #[serde(default)]
    pub opportunity_cost: f64,
    #[serde(default)]
    pub salvage_value: f64,
    #[serde(default)]
    pub initial_inventory: f64,
    /// Nivel de servicio deseado (%) a evaluar.
    #[serde(default)]
    pub known_service: Option<f64>,
    /// % de los faltantes que se atrasan (el resto se pierde).
    #[serde(default = "hundred")]
    pub backorder_pct: f64,
    #[serde(default)]
    pub backorder_cost: f64,
    #[serde(default = "m_inf")]
    pub lost_sale_cost: Bnd,
    /// Costo fijo cada vez que hay faltante.
    #[serde(default)]
    pub fixed_shortage_cost: f64,
    #[serde(default = "const_zero")]
    pub lead_dist: DistIn,
    /// (s, S): tamaño medio de los pedidos de los clientes.
    #[serde(default = "one")]
    pub order_size: f64,
    /// (R, S) y (R, s, S): costo de cada revisión.
    #[serde(default)]
    pub review_cost: f64,

    // --- tamaño de lote
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub periods: Vec<LotPeriod>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum SysMode {
    /// Política óptima.
    #[default]
    Optimal,
    /// Evaluar valores conocidos de s, Q, S y R.
    Known,
    /// Con un nivel de servicio deseado.
    Service,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum LotMethod {
    #[default]
    WagnerWhitin,
    LotForLot,
    Foq,
    Eoq,
    Foi,
    Poq,
    Luc,
    Ltc,
    Ppb,
    SilverMeal,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ItsOptions {
    pub steps: bool,
    pub mode: SysMode,
    /// Valores conocidos (sistemas de inventario).
    pub reorder_point: Option<f64>,
    pub order_quantity: Option<f64>,
    pub order_up_to: Option<f64>,
    pub review_interval: Option<f64>,
    /// Nivel de servicio deseado (%).
    pub service_level: f64,
    pub method: LotMethod,
    /// FOQ: cantidad fija.
    pub fixed_quantity: Option<f64>,
    /// FOI: períodos entre pedidos.
    pub fixed_interval: Option<u32>,
    /// Simulación de los sistemas: tiempo simulado (0 = no simular).
    pub simulation_time: u32,
    pub warmup: u32,
    pub seed: u64,
}

impl Default for ItsOptions {
    fn default() -> Self {
        ItsOptions {
            steps: true,
            mode: SysMode::Optimal,
            reorder_point: None,
            order_quantity: None,
            order_up_to: None,
            review_interval: None,
            service_level: 95.0,
            method: LotMethod::WagnerWhitin,
            fixed_quantity: None,
            fixed_interval: None,
            simulation_time: 0,
            warmup: 0,
            seed: 27437,
        }
    }
}

/// Mínimo de una función unimodal en [a, b] (sección áurea).
pub(crate) fn golden(f: &dyn Fn(f64) -> f64, mut a: f64, mut b: f64) -> f64 {
    let g = (5f64.sqrt() - 1.0) / 2.0;
    let mut c = b - g * (b - a);
    let mut d = a + g * (b - a);
    let (mut fc, mut fd) = (f(c), f(d));
    for _ in 0..200 {
        if (b - a).abs() <= 1e-10 * (1.0 + a.abs() + b.abs()) {
            break;
        }
        if fc <= fd {
            b = d;
            d = c;
            fd = fc;
            c = b - g * (b - a);
            fc = f(c);
        } else {
            a = c;
            c = d;
            fc = fd;
            d = a + g * (b - a);
            fd = f(d);
        }
    }
    let x = 0.5 * (a + b);
    // los extremos pueden ser mejores si la función no es unimodal en el borde
    [a, b, x].into_iter().min_by(|p, q| f(*p).partial_cmp(&f(*q)).unwrap_or(std::cmp::Ordering::Equal)).unwrap()
}

/// Campos de una distribución dentro de un objeto anidado (`path`).
pub(crate) fn dist_widget(title: &str, path: &str) -> Widget {
    let opts: Vec<(&str, &str)> = dist::ALL.iter().filter(|d| !matches!(d, Dist::ThreeTime | Dist::Mixed | Dist::Arbitrary)).map(|d| (d.id(), d.label())).collect();
    let mut fields = vec![Field::choice("distribution", "Distribución", &opts)];
    for (k, key) in ["p1", "p2", "p3"].iter().enumerate() {
        let mut labels: Vec<(&str, Vec<&str>)> = Vec::new();
        for (d, _) in &opts {
            let d = Dist::parse(d).unwrap();
            if d == Dist::Discrete {
                continue;
            }
            if let Some(l) = d.params().get(k) {
                let l = if d == Dist::Constant { "Valor" } else { l };
                match labels.iter_mut().find(|x| x.0 == l) {
                    Some(x) => x.1.push(d.id()),
                    None => labels.push((l, vec![d.id()])),
                }
            }
        }
        for (l, ids) in labels {
            fields.push(Field::number(key, l).optional().show_if("distribution", &ids));
        }
    }
    fields.push(Field::text("values", "Valores (v/p, v/p, …)").optional().show_if("distribution", &["discrete"]));
    Widget::Params { title: Some(title.into()), path: Some(path.into()), fields }
}

pub struct Its;

impl TypedModule for Its {
    type Model = ItsModel;
    type Options = ItsOptions;

    fn file_tag(&self) -> &'static str {
        "ITS"
    }
    fn file_tags(&self) -> Vec<&'static str> {
        vec!["ITS", "IN"]
    }

    fn spec(&self) -> ModuleSpec {
        let unit = Field::text("time_unit", "Unidad de tiempo").default("año");
        let eoq_fields = |discount: bool| {
            let mut v = vec![
                unit.clone(),
                Field::number("demand", "Demanda por unidad de tiempo"),
                Field::number("setup_cost", "Costo de ordenar o preparar (por pedido)"),
                Field::number("holding_cost", "Costo de mantener (por unidad y unidad de tiempo)"),
                Field::bound("shortage_cost", "Costo de faltante por unidad y unidad de tiempo", Value::from("M")).help("M = no se permiten faltantes"),
                Field::number("shortage_cost_fixed", "Costo de faltante por unidad (independiente del tiempo)"),
                Field::bound("production_rate", "Tasa de reabastecimiento o producción", Value::from("M")).help("M = el pedido llega completo"),
                Field::number("lead_time", "Tiempo de entrega").help("En días si indicas los días del año"),
                Field::number("days_per_year", "Días del año (opcional)").optional().help("Ej. 250 días laborales: el tiempo de entrega va en días y se muestra la demanda diaria"),
                Field::number("production_per_day", "Tasa de producción por día (opcional)").optional().help("POQ: entregas parciales por día; requiere los días del año"),
                Field::number("unit_cost", if discount { "Costo unitario sin descuento (primer rango)" } else { "Costo unitario de adquisición" }),
                Field::number("holding_rate", "Costo de mantener como % del costo unitario (opcional)").optional().help("H = i·C; con descuentos cada precio tiene su H"),
                Field::number("known_quantity", "Cantidad a evaluar (opcional)").optional(),
            ];
            if discount {
                v.extend([
                    Field::boolean("incremental", "Descuento incremental (si no, sobre todas las unidades)", false),
                    Field::boolean("discount_holding", "El costo de mantener también se descuenta", false),
                    Field::boolean("discount_shortage", "El costo de faltante por tiempo también se descuenta", false),
                    Field::boolean("discount_shortage_fixed", "El costo de faltante por unidad también se descuenta", false),
                ]);
            }
            v
        };
        let sys_fields = |extra: Option<Field>| {
            let mut v = vec![
                unit.clone(),
                Field::number("setup_cost", "Costo de ordenar (por pedido)"),
                Field::number("unit_cost", "Costo unitario de adquisición"),
                Field::number("holding_cost", "Costo de mantener (por unidad y unidad de tiempo)"),
                Field::number("backorder_pct", "% de faltantes que se atrasan").default(100.0).help("El resto se pierde"),
                Field::number("backorder_cost", "Costo por unidad atrasada"),
                Field::bound("lost_sale_cost", "Costo por venta perdida", Value::from("M")),
                Field::number("fixed_shortage_cost", "Costo fijo cada vez que hay faltante"),
            ];
            v.extend(extra);
            v
        };
        let pt = |id: &str, label: &str, desc: &str, layout: Vec<Widget>, dims: Vec<Field>| ProblemType { id: id.into(), label: label.into(), description: desc.into(), dims, layout };
        ModuleSpec {
            id: "its".into(),
            winqsb: "ITS".into(),
            name: "Inventarios".into(),
            description: "EOQ, descuentos, vendedor de periódicos, tamaño de lote y sistemas (s,Q), (s,S), (R,S), (R,s,S)".into(),
            file_ext: "ITS".into(),
            problem_types: vec![
                pt("eoq", "EOQ (demanda determinística)", "Cantidad económica de pedido con faltantes planeados y producción gradual", vec![Widget::params(None, eoq_fields(false))], vec![]),
                pt(
                    "discount",
                    "Descuentos por cantidad",
                    "EOQ con precios que bajan al comprar más",
                    vec![
                        Widget::params(None, eoq_fields(true)),
                        Widget::records(
                            Some("Descuentos"),
                            "breaks",
                            vec![
                                Field::number("quantity", "A partir de (cantidad)"),
                                Field::number("discount", "Descuento (%)"),
                                Field::number("price", "o precio unitario").optional().help("Si lo das, se calcula el descuento"),
                            ],
                            Some("Añadir descuento"),
                        ),
                    ],
                    vec![Field::integer("breaks", "Número de descuentos", 2)],
                ),
                pt(
                    "newsboy",
                    "Período único (vendedor de periódicos)",
                    "Cuánto pedir para una sola temporada con demanda aleatoria",
                    vec![
                        dist_widget("Demanda del período", "demand_dist"),
                        Widget::params(
                            Some("Costos y precios"),
                            vec![
                                Field::number("setup_cost", "Costo fijo de ordenar"),
                                Field::number("unit_cost", "Costo unitario"),
                                Field::number("selling_price", "Precio de venta"),
                                Field::number("opportunity_cost", "Costo de oportunidad por unidad faltante"),
                                Field::number("salvage_value", "Valor de rescate por unidad sobrante"),
                                Field::number("initial_inventory", "Inventario inicial"),
                                Field::number("known_quantity", "Cantidad a evaluar (opcional)").optional(),
                                Field::number("known_service", "Nivel de servicio a evaluar (%, opcional)").optional(),
                            ],
                        ),
                    ],
                    vec![],
                ),
                pt(
                    "lot_sizing",
                    "Tamaño de lote (demanda dinámica)",
                    "Cuándo y cuánto producir con demandas distintas en cada período",
                    vec![Widget::records(
                        Some("Períodos"),
                        "periods",
                        vec![
                            Field::text("name", "Período"),
                            Field::number("demand", "Demanda"),
                            Field::number("setup_cost", "Costo de preparación"),
                            Field::number("unit_cost", "Costo unitario"),
                            Field::number("holding_cost", "Costo de mantener"),
                            Field::bound("backorder_cost", "Costo de atraso", Value::from("M")).help("M = sin atrasos"),
                        ],
                        Some("Añadir período"),
                    )],
                    vec![Field::integer("periods", "Número de períodos", 6)],
                ),
                pt(
                    "sq",
                    "Revisión continua (s, Q)",
                    "Pedir Q unidades cuando el inventario llega a s",
                    vec![Widget::params(Some("Costos"), sys_fields(None)), dist_widget("Demanda por unidad de tiempo", "demand_dist"), dist_widget("Tiempo de entrega", "lead_dist")],
                    vec![],
                ),
                pt(
                    "ss",
                    "Revisión continua (s, S)",
                    "Pedir hasta S cuando el inventario llega a s",
                    vec![
                        Widget::params(Some("Costos"), sys_fields(Some(Field::number("order_size", "Tamaño medio de los pedidos de los clientes").default(1.0)))),
                        dist_widget("Demanda por unidad de tiempo", "demand_dist"),
                        dist_widget("Tiempo de entrega", "lead_dist"),
                    ],
                    vec![],
                ),
                pt(
                    "rs",
                    "Revisión periódica (R, S)",
                    "Cada R unidades de tiempo pedir hasta S",
                    vec![
                        Widget::params(Some("Costos"), sys_fields(Some(Field::number("review_cost", "Costo de cada revisión")))),
                        dist_widget("Demanda por unidad de tiempo", "demand_dist"),
                        dist_widget("Tiempo de entrega", "lead_dist"),
                    ],
                    vec![],
                ),
                pt(
                    "rss",
                    "Revisión periódica (R, s, S)",
                    "Cada R unidades de tiempo, si el inventario está por debajo de s, pedir hasta S",
                    vec![
                        Widget::params(Some("Costos"), sys_fields(Some(Field::number("review_cost", "Costo de cada revisión")))),
                        dist_widget("Demanda por unidad de tiempo", "demand_dist"),
                        dist_widget("Tiempo de entrega", "lead_dist"),
                    ],
                    vec![],
                ),
            ],
            options: vec![
                Field::boolean("steps", "Mostrar paso a paso", true),
                Field::choice("mode", "Resolver", &[("optimal", "Política óptima"), ("known", "Con valores conocidos"), ("service", "Con nivel de servicio")]).default("optimal").show_if("problem_type", &["sq", "ss", "rs", "rss"]),
                Field::number("service_level", "Nivel de servicio deseado (%)").default(95.0).show_if("problem_type", &["sq", "ss", "rs", "rss"]).show_if("mode", &["service"]),
                Field::number("reorder_point", "Punto de reorden s").optional().show_if("problem_type", &["sq", "ss", "rss"]).show_if("mode", &["known"]),
                Field::number("order_quantity", "Cantidad a pedir Q").optional().show_if("problem_type", &["sq"]).show_if("mode", &["known"]),
                Field::number("order_up_to", "Nivel máximo S").optional().show_if("problem_type", &["ss", "rs", "rss"]).show_if("mode", &["known"]),
                Field::number("review_interval", "Intervalo de revisión R").optional().show_if("problem_type", &["rs", "rss"]).help("Vacío = el óptimo"),
                Field::choice(
                    "method",
                    "Método",
                    &[
                        ("wagner_whitin", "Wagner-Whitin (óptimo)"),
                        ("lot_for_lot", "Lote a lote"),
                        ("foq", "Cantidad fija (FOQ)"),
                        ("eoq", "EOQ"),
                        ("foi", "Intervalo fijo (FOI)"),
                        ("poq", "Cantidad periódica (POQ)"),
                        ("luc", "Mínimo costo unitario (LUC)"),
                        ("ltc", "Mínimo costo total (LTC)"),
                        ("ppb", "Balanceo de períodos (PPB)"),
                        ("silver_meal", "Silver-Meal"),
                    ],
                ).comparable()
                .default("wagner_whitin")
                .show_if("problem_type", &["lot_sizing"]),
                Field::number("fixed_quantity", "Cantidad fija (FOQ)").optional().show_if("problem_type", &["lot_sizing"]).show_if("method", &["foq"]),
                Field::integer("fixed_interval", "Períodos entre pedidos (FOI)", 2).show_if("problem_type", &["lot_sizing"]).show_if("method", &["foi"]),
                Field::integer("simulation_time", "Tiempo simulado", 0).help("0 = no simular").show_if("problem_type", &["sq", "ss", "rs", "rss"]),
                Field::integer("warmup", "Inicio de la recolección de datos", 0).show_if("problem_type", &["sq", "ss", "rs", "rss"]),
                Field::integer("seed", "Semilla", 27437).show_if("problem_type", &["sq", "ss", "rs", "rss"]),
            ],
        }
    }

    fn template(&self, pt: &str, dims: &Value) -> Result<ItsModel, ModError> {
        let t: ItsType = serde_json::from_value(Value::from(pt)).map_err(|_| ModError::Invalid(format!("tipo de problema desconocido: {pt}")))?;
        let mut m: ItsModel = serde_json::from_value(serde_json::json!({"name": "Inventario nuevo", "problem_type": t})).unwrap();
        match t {
            ItsType::Discount => {
                let n = dim(dims, "breaks", 2, 1, 100)?;
                m.breaks = (1..=n).map(|i| Break { quantity: 0.0, discount: i as f64, price: None }).collect();
            }
            ItsType::LotSizing => {
                let n = dim(dims, "periods", 6, 1, 300)?;
                m.periods = (1..=n).map(|i| LotPeriod { name: i.to_string(), demand: 0.0, setup_cost: 0.0, unit_cost: 0.0, holding_cost: 0.0, backorder_cost: Bnd::INF }).collect();
            }
            _ => {}
        }
        if t == ItsType::LotSizing {
            m.time_unit = "período".into();
        }
        Ok(m)
    }

    fn solve(&self, m0: &ItsModel, o: &ItsOptions) -> Result<Report, ModError> {
        let n = m0.normalized()?;
        let m = &n;
        let mut r = match m.problem_type {
            ItsType::Eoq => det::eoq(m, o)?,
            ItsType::Discount => det::discount(m, o)?,
            ItsType::Newsboy => newsboy::solve(m, o)?,
            ItsType::LotSizing => lot::solve(m, o)?,
            _ => stoch::solve(m, o)?,
        };
        if r.problem.is_empty() {
            r.problem = m.name.clone();
        }
        Ok(r)
    }

    fn read(&self, bytes: &[u8]) -> Result<ItsModel, ModError> {
        read_its(bytes)
    }

    fn write(&self, m: &ItsModel) -> Result<Vec<u8>, ModError> {
        Ok(write_its(&m.normalized()?))
    }

    fn examples(&self) -> Vec<(&'static str, ItsModel)> {
        let ex = |s: &str| serde_json::from_str::<ItsModel>(s).unwrap();
        vec![
            ("eoq_altavoces", ex(include_str!("ejemplos/eoq.json"))),
            ("eoq_faltantes", ex(include_str!("ejemplos/eoq_faltantes.json"))),
            ("descuentos", ex(include_str!("ejemplos/descuentos.json"))),
            ("periodicos", ex(include_str!("ejemplos/periodicos.json"))),
            ("tamano_lote", ex(include_str!("ejemplos/lote.json"))),
            ("sistema_sq", ex(include_str!("ejemplos/sq.json"))),
            ("sistema_rs", ex(include_str!("ejemplos/rs.json"))),
            ("sistema_rss", ex(include_str!("ejemplos/rss.json"))),
            ("clase_ferreteria_eoq", ex(include_str!("ejemplos/ferreteria_eoq.json"))),
            ("clase_ferreteria_poq", ex(include_str!("ejemplos/ferreteria_poq.json"))),
            ("clase_te_always_fresh", ex(include_str!("ejemplos/te_always_fresh.json"))),
            ("clase_descuento_limpieza", ex(include_str!("ejemplos/descuento_limpieza.json"))),
            ("clase_descuento_tres_rangos", ex(include_str!("ejemplos/descuento_tres_rangos.json"))),
        ]
    }
}

pub(crate) fn money(x: f64) -> Cell {
    Cell::Num(tidy(x))
}

// ------------------------------------------------------------------ archivos

/// Etiquetas de WinQSB para los parámetros de una distribución.
fn dist_param_labels(d: Dist) -> [&'static str; 3] {
    match d {
        Dist::Normal => ["Mean (u)", "Standard deviation (s>0)", "(Not used)"],
        Dist::Uniform => ["Lower limit (a)", "Upper limit (b)", "(Not used)"],
        Dist::Triangular => ["Lower limit (a)", "Shape parameter (mode) (b)", "Upper limit (c)"],
        Dist::Exponential => ["Location parameter (a)", "Scale parameter (b>0)", "(Not used)"],
        Dist::Lognormal => ["Mean of log(x) (u)", "Standard deviation of log(x) (s>0)", "(Not used)"],
        Dist::Binomial => ["Number of trials (n>0)", "Probability of success (p>0)", "(Not used)"],
        Dist::Poisson => ["Mean (u)", "(Not used)", "(Not used)"],
        Dist::Geometric => ["Probability of success (p>0)", "(Not used)", "(Not used)"],
        Dist::Hypergeometric => ["Population size (N>0)", "Number of target items (K>0,K<=N)", "Sample size (n>0)"],
        Dist::Constant => ["Constant value", "(Not used)", "(Not used)"],
        Dist::Discrete => ["Discrete values (v/p,v/p,...)", "(Not used)", "(Not used)"],
        Dist::Beta => ["Scale parameter (b>0)", "Shape parameter (v>0)", "Shape parameter (w>0)"],
        Dist::Erlang => ["Location parameter (a)", "Scale parameter (b>0)", "Shape parameter (c>=1, integer)"],
        Dist::Laplace => ["Location parameter (a)", "Scale parameter (b>0)", "(Not used)"],
        Dist::Pareto => ["Location parameter (a>0)", "Shape parameter (c>0)", "(Not used)"],
        _ => ["Location parameter (a)", "Scale parameter (b>0)", "Shape parameter (c>0)"],
    }
}

fn opt_num(x: Option<f64>) -> String {
    x.map(num).unwrap_or_else(|| " ".into())
}

pub(crate) fn dist_rows(w: &mut Writer, title: &str, d: &DistIn) {
    w.row(&[title.to_string(), d.distribution.winqsb().to_string()]);
    let l = dist_param_labels(d.distribution);
    if d.distribution == Dist::Discrete {
        w.row(&[l[0].to_string(), d.values.clone().unwrap_or_default()]);
        w.row(&[l[1].to_string(), " ".into()]);
        w.row(&[l[2].to_string(), " ".into()]);
    } else {
        for (k, v) in [d.p1, d.p2, d.p3].iter().enumerate() {
            w.row(&[l[k].to_string(), opt_num(*v)]);
        }
    }
}

fn write_its(m: &ItsModel) -> Vec<u8> {
    let mut w = Writer::new();
    let t = m.problem_type;
    let u = &m.time_unit;
    let np = if t == ItsType::LotSizing { m.periods.len() } else { 0 };
    w.row(&["ITS".to_string(), m.name.clone(), u.clone(), t.code().to_string(), np.to_string()]);
    let item = |w: &mut Writer, l: String, v: String| {
        w.row(&[l, v]);
    };
    match t {
        ItsType::Eoq | ItsType::Discount => {
            item(&mut w, "DATA  ITEM".into(), "ENTRY".into());
            item(&mut w, format!("Demand per {u}"), num(m.demand));
            item(&mut w, "Order or setup cost per order".into(), num(m.setup_cost));
            item(&mut w, format!("Unit holding cost per {u}"), num(m.holding_cost));
            item(&mut w, format!("Unit shortage cost per {u}"), m.shortage_cost.text());
            item(&mut w, "Unit shortage cost independent of time".into(), num(m.shortage_cost_fixed));
            item(&mut w, format!("Replenishment or production rate per {u}"), m.production_rate.text());
            item(&mut w, format!("Lead time for a new order in {u}"), num(m.lead_time));
            item(&mut w, "Unit acquisition cost without discount".into(), num(m.unit_cost));
            item(&mut w, "Number of discount breaks (quantities)".into(), if t == ItsType::Discount { m.breaks.len().to_string() } else { " ".into() });
            item(&mut w, "Order quantity if you known".into(), opt_num(m.known_quantity));
            if t == ItsType::Discount {
                let b = |x: bool| if x { "1" } else { "0" }.to_string();
                w.row(&[b(m.incremental), b(m.discount_holding), b(m.discount_shortage), b(m.discount_shortage_fixed)]);
                w.row(&[m.breaks.len().to_string()]);
                for (i, br) in m.breaks.iter().enumerate() {
                    w.row(&[(i + 1).to_string(), num(br.quantity), num(br.discount)]);
                }
            }
        }
        ItsType::Newsboy => {
            item(&mut w, "DATA  ITEM".into(), "ENTRY".into());
            dist_rows(&mut w, &format!("Demand distribution (in {u})"), &m.demand_dist);
            item(&mut w, "Order or setup cost".into(), num(m.setup_cost));
            item(&mut w, "Unit acquisition cost".into(), num(m.unit_cost));
            item(&mut w, "Unit selling price".into(), num(m.selling_price));
            item(&mut w, "Unit shortage (opportunity) cost".into(), num(m.opportunity_cost));
            item(&mut w, "Unit salvage value".into(), num(m.salvage_value));
            item(&mut w, "Initial inventory".into(), num(m.initial_inventory));
            item(&mut w, "Order quantity if you know".into(), opt_num(m.known_quantity));
            item(&mut w, "Desired service level (%) if you know".into(), opt_num(m.known_service));
        }
        ItsType::LotSizing => {
            w.row(&[u.as_str(), "Demand", "Setup Cost", "Unit Variable Cost", "Unit Holding Cost", "Unit Backorder Cost"]);
            for (i, p) in m.periods.iter().enumerate() {
                let name = if p.name.trim().is_empty() { (i + 1).to_string() } else { p.name.clone() };
                w.row(&[name, num(p.demand), num(p.setup_cost), num(p.unit_cost), num(p.holding_cost), p.backorder_cost.text()]);
            }
        }
        _ => {
            item(&mut w, "DATA  ITEM".into(), "ENTRY".into());
            dist_rows(&mut w, &format!("Demand distribution (in {u})"), &m.demand_dist);
            item(&mut w, "Order or setup cost".into(), num(m.setup_cost));
            item(&mut w, "Unit acquisition cost".into(), num(m.unit_cost));
            item(&mut w, format!("Unit holding cost per {u}"), num(m.holding_cost));
            item(&mut w, "Estimated % of shortage will be backordered".into(), num(m.backorder_pct));
            item(&mut w, "Unit backorder cost".into(), num(m.backorder_cost));
            item(&mut w, "Estimated % of shortage will be lost".into(), num(100.0 - m.backorder_pct));
            item(&mut w, "Unit lost-sales cost".into(), m.lost_sale_cost.text());
            item(&mut w, "Fixed cost if shortage occurs".into(), num(m.fixed_shortage_cost));
            dist_rows(&mut w, &format!("Lead time distribution (in {u})"), &m.lead_dist);
            match t {
                ItsType::SS => item(&mut w, "Average customer order size".into(), num(m.order_size)),
                ItsType::RS | ItsType::RSS => item(&mut w, "Review cost per review".into(), num(m.review_cost)),
                _ => {}
            }
        }
    }
    w.bytes()
}

pub(crate) fn read_dist(r: &mut Reader) -> Result<DistIn, ModError> {
    let l = r.line("la distribución")?;
    let d = Dist::parse(l.get(1)).filter(|d| *d != Dist::ThreeTime || !l.get(1).trim().is_empty()).unwrap_or(Dist::Constant);
    let mut out = DistIn { distribution: d, p1: None, p2: None, p3: None, values: None };
    let rows: Vec<String> = (0..3).map(|_| r.line("los parámetros de la distribución").map(|x| x.get(1).to_string())).collect::<Result<_, _>>()?;
    if d == Dist::Discrete {
        out.values = Some(rows[0].trim().to_string());
    } else {
        out.p1 = parse_num(&rows[0]);
        out.p2 = parse_num(&rows[1]);
        out.p3 = parse_num(&rows[2]);
    }
    Ok(out)
}

fn read_its(bytes: &[u8]) -> Result<ItsModel, ModError> {
    let mut r = Reader::new(bytes);
    let h = r.line("la cabecera")?;
    let tag = h.get(0).trim().to_ascii_uppercase();
    if !(tag.starts_with("IT") || tag.starts_with("IN")) {
        return Err(ModError::Format("no es un archivo de ITS".into()));
    }
    let code = parse_num(h.get(3)).unwrap_or(0.0) as usize;
    let t = *ItsType::ALL.get(code).ok_or_else(|| ModError::Format(format!("tipo de problema de ITS desconocido: {code}")))?;
    let mut m: ItsModel = serde_json::from_value(serde_json::json!({"name": h.get(1).trim(), "problem_type": t})).unwrap();
    if m.name.is_empty() {
        m.name = "Inventory Problem".into();
    }
    let unit = h.get(2).trim();
    m.time_unit = if unit.is_empty() { "year".into() } else { unit.to_string() };
    let np = parse_num(h.get(4)).unwrap_or(0.0) as usize;
    r.line("la fila de títulos")?;
    let val = |r: &mut Reader, what: &str| -> Result<String, ModError> { Ok(r.line(what)?.get(1).trim().to_string()) };
    let bnd = |s: &str| parse_num(s).map(Bnd).unwrap_or(Bnd::INF);
    let n0 = |s: &str| parse_num(s).unwrap_or(0.0);
    match t {
        ItsType::Eoq | ItsType::Discount => {
            m.demand = n0(&val(&mut r, "la demanda")?);
            m.setup_cost = n0(&val(&mut r, "el costo de ordenar")?);
            m.holding_cost = n0(&val(&mut r, "el costo de mantener")?);
            m.shortage_cost = bnd(&val(&mut r, "el costo de faltante")?);
            m.shortage_cost_fixed = n0(&val(&mut r, "el costo de faltante por unidad")?);
            m.production_rate = bnd(&val(&mut r, "la tasa de producción")?);
            m.lead_time = n0(&val(&mut r, "el tiempo de entrega")?);
            m.unit_cost = n0(&val(&mut r, "el costo unitario")?);
            let _nb = val(&mut r, "el número de descuentos")?;
            m.known_quantity = parse_num(&val(&mut r, "la cantidad conocida")?);
            if t == ItsType::Discount {
                if let Some(f) = r.try_line() {
                    let b = |i: usize| parse_num(f.get(i)).unwrap_or(0.0) != 0.0;
                    m.incremental = b(0);
                    m.discount_holding = b(1);
                    m.discount_shortage = b(2);
                    m.discount_shortage_fixed = b(3);
                    let n = r.try_line().and_then(|l| parse_num(l.get(0))).unwrap_or(0.0) as usize;
                    for _ in 0..n.min(10_000) {
                        let l = r.line("un descuento")?;
                        m.breaks.push(Break { quantity: n0(l.get(1)), discount: n0(l.get(2)), price: None });
                    }
                }
            }
        }
        ItsType::Newsboy => {
            m.demand_dist = read_dist(&mut r)?;
            m.setup_cost = n0(&val(&mut r, "el costo de ordenar")?);
            m.unit_cost = n0(&val(&mut r, "el costo unitario")?);
            m.selling_price = n0(&val(&mut r, "el precio de venta")?);
            m.opportunity_cost = n0(&val(&mut r, "el costo de oportunidad")?);
            m.salvage_value = n0(&val(&mut r, "el valor de rescate")?);
            m.initial_inventory = n0(&val(&mut r, "el inventario inicial")?);
            m.known_quantity = r.try_line().and_then(|l| parse_num(l.get(1)));
            m.known_service = r.try_line().and_then(|l| parse_num(l.get(1)));
        }
        ItsType::LotSizing => {
            for i in 0..np {
                let l = r.line(&format!("el período {}", i + 1))?;
                m.periods.push(LotPeriod {
                    name: Some(l.get(0).trim()).filter(|s| !s.is_empty()).map(str::to_string).unwrap_or((i + 1).to_string()),
                    demand: n0(l.get(1)),
                    setup_cost: n0(l.get(2)),
                    unit_cost: n0(l.get(3)),
                    holding_cost: n0(l.get(4)),
                    backorder_cost: bnd(l.get(5)),
                });
            }
        }
        _ => {
            m.demand_dist = read_dist(&mut r)?;
            m.setup_cost = n0(&val(&mut r, "el costo de ordenar")?);
            m.unit_cost = n0(&val(&mut r, "el costo unitario")?);
            m.holding_cost = n0(&val(&mut r, "el costo de mantener")?);
            m.backorder_pct = parse_num(&val(&mut r, "el % atrasado")?).unwrap_or(100.0);
            m.backorder_cost = n0(&val(&mut r, "el costo de atraso")?);
            let _lost = val(&mut r, "el % perdido")?;
            m.lost_sale_cost = bnd(&val(&mut r, "el costo de venta perdida")?);
            m.fixed_shortage_cost = n0(&val(&mut r, "el costo fijo de faltante")?);
            m.lead_dist = read_dist(&mut r)?;
            if let Some(l) = r.try_line() {
                let v = n0(l.get(1));
                match t {
                    ItsType::SS => m.order_size = if v > 0.0 { v } else { 1.0 },
                    ItsType::RS | ItsType::RSS => m.review_cost = v,
                    _ => {}
                }
            }
        }
    }
    Ok(m)
}
