//! PERT-CPM · Planificación de proyectos (equivalente al módulo PERT-CPM de WinQSB).
//!
//! - CPM determinista: ruta crítica con tiempos normales o de compresión, todas
//!   las rutas críticas, diagrama de Gantt, análisis de compresión (costo mínimo,
//!   plazo deseado o presupuesto, con penalización y premio) resuelto con HiGHS y
//!   su curva tiempo-costo, PERT/Costo (presupuesto por periodo según inicios
//!   tempranos y tardíos), informe de control de costos y análisis de avance.
//! - PERT probabilístico: 20 distribuciones de tiempo (o una por actividad),
//!   probabilidad de terminar en un plazo y simulación de Monte Carlo con índice
//!   de criticidad.
//!
//! El modo didáctico muestra el pase hacia adelante, el pase hacia atrás, las
//! holguras, el cálculo de medias y varianzas y la compresión semana a semana.

mod cost;
mod cpm;
mod crash;
pub mod dist;
mod format;
mod prob;

use dist::Dist;
use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub enum PertType {
    /// CPM: tiempos conocidos (y, si se quiere, compresión y costos).
    #[default]
    #[serde(rename = "cpm", alias = "deterministic")]
    Deterministic,
    /// PERT: tiempos aleatorios.
    #[serde(rename = "pert", alias = "probabilistic")]
    Probabilistic,
}

impl PertType {
    fn label(self) -> &'static str {
        match self {
            PertType::Deterministic => "CPM determinista",
            PertType::Probabilistic => "PERT probabilístico",
        }
    }
}

/// Una actividad del proyecto.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Activity {
    pub name: String,
    /// Predecesoras inmediatas: nombres o números de actividad.
    #[serde(default)]
    pub predecessors: Vec<String>,
    /// CPM: tiempo normal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normal_time: Option<f64>,
    /// CPM: tiempo de compresión (el mínimo posible).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crash_time: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normal_cost: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crash_cost: Option<f64>,
    /// Costo real hasta la fecha (control de costos).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_cost: Option<f64>,
    /// Porcentaje terminado, de 0 a 100 (control de avance).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percent_complete: Option<f64>,
    /// PERT con distribución mixta: la distribución de esta actividad.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distribution: Option<Dist>,
    /// PERT: parámetros de la distribución, en el orden de WinQSB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p1: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p2: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub p3: Option<f64>,
    /// PERT, distribución discreta: "valor/probabilidad, valor/probabilidad, ...".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discrete: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PertModel {
    #[serde(default)]
    pub name: String,
    pub problem_type: PertType,
    /// Unidad de tiempo, en singular (semana, día...).
    #[serde(default = "default_unit")]
    pub time_unit: String,
    /// PERT: distribución de los tiempos de todas las actividades, o "mixed".
    #[serde(default)]
    pub time_distribution: Dist,
    pub activities: Vec<Activity>,
    /// Posiciones del modelo gráfico de WinQSB (fila, columna).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub positions: Option<Vec<(i64, i64)>>,
    /// El archivo se guardó desde la forma gráfica de WinQSB.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub graphic: bool,
}

fn default_unit() -> String {
    "semana".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimeBasis {
    #[default]
    Normal,
    /// Ruta crítica con los tiempos de compresión.
    Crash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Crashing {
    #[default]
    None,
    /// Programa de costo total mínimo (con penalización por retraso y premio por adelanto).
    MinCost,
    /// Costo mínimo para terminar en el plazo deseado.
    Deadline,
    /// Menor duración posible sin pasar del presupuesto.
    Budget,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct PertOptions {
    /// Mostrar el método paso a paso.
    pub steps: bool,
    /// CPM: tiempos con los que se calcula la ruta crítica.
    pub time_basis: TimeBasis,
    /// CPM: análisis de compresión.
    pub crashing: Crashing,
    /// Plazo deseado de terminación (compresión, probabilidad y simulación).
    pub desired_time: Option<f64>,
    /// Presupuesto total (compresión por presupuesto).
    pub budget: Option<f64>,
    /// Penalización por cada unidad de tiempo de retraso sobre el plazo.
    pub late_penalty: Option<f64>,
    /// Premio por cada unidad de tiempo de adelanto sobre el plazo.
    pub early_reward: Option<f64>,
    /// CPM: fecha actual del proyecto para el análisis de avance.
    pub current_time: Option<f64>,
    /// PERT: observaciones de la simulación (0 = no simular).
    pub simulation_runs: u32,
    /// PERT: semilla de la simulación (27437 es la de WinQSB).
    pub seed: u64,
}

impl Default for PertOptions {
    fn default() -> Self {
        PertOptions {
            steps: true,
            time_basis: TimeBasis::Normal,
            crashing: Crashing::None,
            desired_time: None,
            budget: None,
            late_penalty: None,
            early_reward: None,
            current_time: None,
            simulation_runs: 1000,
            seed: 27437,
        }
    }
}

impl PertModel {
    pub fn validate(&self) -> Result<(), ModError> {
        if self.activities.is_empty() {
            return invalid("el proyecto no tiene actividades");
        }
        check_names("actividades", &self.activities.iter().map(|a| &a.name).collect::<Vec<_>>())?;
        for a in &self.activities {
            let vals = [a.normal_time, a.crash_time, a.normal_cost, a.crash_cost, a.actual_cost, a.percent_complete, a.p1, a.p2, a.p3];
            if vals.iter().flatten().any(|x| !x.is_finite()) {
                return invalid(format!("actividad {}: hay datos que no son números finitos", a.name));
            }
            if self.problem_type == PertType::Deterministic {
                let t = a.normal_time.unwrap_or(0.0);
                if t < 0.0 {
                    return invalid(format!("actividad {}: el tiempo normal no puede ser negativo", a.name));
                }
                if let Some(c) = a.crash_time {
                    if c < 0.0 || c > t + 1e-12 {
                        return invalid(format!("actividad {}: el tiempo de compresión debe estar entre 0 y el tiempo normal", a.name));
                    }
                }
                if let Some(p) = a.percent_complete {
                    if !(0.0..=100.0).contains(&p) {
                        return invalid(format!("actividad {}: el porcentaje terminado debe estar entre 0 y 100", a.name));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn has(&self, f: impl Fn(&Activity) -> Option<f64>) -> bool {
        self.activities.iter().any(|a| f(a).is_some())
    }

    pub fn unit_plural(&self) -> String {
        plural(&self.time_unit)
    }
}

pub fn plural(u: &str) -> String {
    let u = u.trim();
    if u.is_empty() {
        return "unidades".into();
    }
    // Unidades en español o en inglés (los ejemplos de WinQSB usan «day», «week»...).
    let low = u.to_lowercase();
    if low == "mes" {
        return format!("{u}es");
    }
    let ch: Vec<char> = low.chars().collect();
    let last = ch[ch.len() - 1];
    let prev = if ch.len() > 1 { ch[ch.len() - 2] } else { ' ' };
    let vowel = |c: char| "aeiouáéíóú".contains(c);
    match last {
        's' => u.to_string(),
        'y' if !vowel(prev) => format!("{}ies", &u[..u.len() - 1]),
        'x' | 'z' => format!("{u}es"),
        'h' if prev == 'c' || prev == 's' => format!("{u}es"),
        _ => format!("{u}s"),
    }
}

pub struct Pert;

/// Columnas de parámetros: una por (posición, nombre), visible con las distribuciones que la usan.
fn param_columns() -> Vec<Field> {
    let keys = ["p1", "p2", "p3"];
    let mut out = Vec::new();
    for (k, key) in keys.iter().enumerate() {
        let mut labels: Vec<(&str, Vec<&str>)> = Vec::new();
        for d in dist::ALL {
            if d == Dist::Mixed || d == Dist::Discrete {
                continue;
            }
            if let Some(l) = d.params().get(k) {
                match labels.iter_mut().find(|x| x.0 == *l) {
                    Some(x) => x.1.push(d.id()),
                    None => labels.push((l, vec![d.id()])),
                }
            }
        }
        for (l, ids) in labels {
            out.push(Field::number(key, l).optional().show_if("time_distribution", &ids));
        }
    }
    out
}

fn mixed_columns() -> Vec<Field> {
    let opts: Vec<(&str, &str)> = dist::ALL.iter().filter(|d| **d != Dist::Mixed).map(|d| (d.id(), d.label())).collect();
    let help = dist::ALL
        .iter()
        .filter(|d| **d != Dist::Mixed)
        .map(|d| format!("{}: {}", d.label(), d.params().join(", ")))
        .collect::<Vec<_>>()
        .join(" · ");
    let mut v = vec![Field::choice("distribution", "Distribución", &opts)
        .help(&format!("Parámetros 1, 2 y 3 según la distribución. {help}"))
        .show_if("time_distribution", &["mixed"])];
    for (k, key) in ["p1", "p2", "p3"].iter().enumerate() {
        let ids: Vec<&str> = dist::ALL
            .iter()
            .filter(|d| **d != Dist::Mixed && (d.params().len() > k || (**d == Dist::Discrete && k == 0)))
            .map(|d| d.id())
            .collect();
        v.push(
            Field::number(key, &format!("Parámetro {}", k + 1))
                .optional()
                .show_if("time_distribution", &["mixed"])
                .show_if("distribution", &ids),
        );
    }
    v.push(
        Field::text("discrete", "Valores (v/p, v/p, …)")
            .optional()
            .show_if("time_distribution", &["mixed"])
            .show_if("distribution", &["discrete"]),
    );
    v
}

impl TypedModule for Pert {
    type Model = PertModel;
    type Options = PertOptions;

    fn file_tag(&self) -> &'static str {
        "CPM"
    }
    fn file_tags(&self) -> Vec<&'static str> {
        vec!["CPM", "PERT"]
    }

    fn spec(&self) -> ModuleSpec {
        let base = vec![Field::text("name", "Actividad"), Field::list("predecessors", "Predecesoras").help("Nombres o números separados por comas")];
        let mut cpm_cols = base.clone();
        cpm_cols.extend([
            Field::number("normal_time", "Tiempo normal"),
            Field::number("crash_time", "Tiempo de compresión").optional(),
            Field::number("normal_cost", "Costo normal").optional(),
            Field::number("crash_cost", "Costo de compresión").optional(),
            Field::number("actual_cost", "Costo real").optional(),
            Field::number("percent_complete", "% terminado").optional(),
        ]);
        let mut pert_cols = base;
        pert_cols.extend(param_columns());
        pert_cols.push(Field::text("discrete", "Valores (v/p, v/p, …)").optional().show_if("time_distribution", &["discrete"]));
        pert_cols.extend(mixed_columns());
        let dist_opts: Vec<(&str, &str)> = dist::ALL.iter().map(|d| (d.id(), d.label())).collect();
        let unit = Field::text("time_unit", "Unidad de tiempo").default("semana");
        let n = Field::integer("activities", "Número de actividades", 8);
        ModuleSpec {
            id: "pert".into(),
            winqsb: "PERT-CPM".into(),
            name: "PERT y ruta crítica".into(),
            description: "Ruta crítica, Gantt, compresión de tiempos, costos y control del proyecto; PERT con probabilidad y simulación".into(),
            file_ext: "CPM".into(),
            problem_types: vec![
                ProblemType {
                    id: "cpm".into(),
                    label: PertType::Deterministic.label().into(),
                    description: "Tiempos conocidos: ruta crítica, compresión (crashing), PERT/Costo y control".into(),
                    dims: vec![n.clone(), unit.clone()],
                    layout: vec![
                        Widget::params(None, vec![unit.clone()]),
                        Widget::records(Some("Actividades"), "activities", cpm_cols, Some("Añadir actividad")),
                    ],
                },
                ProblemType {
                    id: "pert".into(),
                    label: PertType::Probabilistic.label().into(),
                    description: "Tiempos aleatorios: tres estimaciones u otra distribución; probabilidad y simulación".into(),
                    dims: vec![n, unit.clone(), Field::choice("time_distribution", "Distribución de los tiempos", &dist_opts)],
                    layout: vec![
                        Widget::params(None, vec![unit, Field::choice("time_distribution", "Distribución de los tiempos", &dist_opts)]),
                        Widget::records(Some("Actividades"), "activities", pert_cols, Some("Añadir actividad")),
                    ],
                },
            ],
            options: vec![
                Field::boolean("steps", "Mostrar paso a paso", true),
                Field::choice("time_basis", "Tiempos", &[("normal", "Normales"), ("crash", "De compresión")])
                    .show_if("problem_type", &["cpm"])
                    .show_if("crashing", &["none"]),
                Field::choice(
                    "crashing",
                    "Compresión",
                    &[("none", "Sin compresión"), ("min_cost", "Costo total mínimo"), ("deadline", "Plazo deseado"), ("budget", "Presupuesto")],
                )
                .show_if("problem_type", &["cpm"]),
                Field::number("desired_time", "Plazo deseado")
                    .optional()
                    .help("Para la compresión, la probabilidad y la simulación"),
                Field::number("budget", "Presupuesto total").optional().show_if("problem_type", &["cpm"]).show_if("crashing", &["budget"]),
                Field::number("late_penalty", "Penalización por retraso").optional().help("Por unidad de tiempo").show_if("problem_type", &["cpm"]).show_if("crashing", &["min_cost"]),
                Field::number("early_reward", "Premio por adelanto").optional().help("Por unidad de tiempo").show_if("problem_type", &["cpm"]).show_if("crashing", &["min_cost"]),
                Field::number("current_time", "Fecha actual (control)")
                    .optional()
                    .help("Avance planeado frente al real")
                    .show_if("problem_type", &["cpm"]),
                Field::integer("simulation_runs", "Observaciones simuladas", 1000).help("0 = no simular").show_if("problem_type", &["pert"]),
                Field::integer("seed", "Semilla", 27437).show_if("problem_type", &["pert"]),
            ],
        }
    }

    fn template(&self, problem_type: &str, dims: &Value) -> Result<PertModel, ModError> {
        let t: PertType = serde_json::from_value(Value::from(problem_type))
            .map_err(|_| ModError::Invalid(format!("tipo de problema desconocido: {problem_type}")))?;
        let n = dim(dims, "activities", 8, 1, 2000)?;
        let unit = dim_str(dims, "time_unit", "semana");
        let td = Dist::parse(&dim_str(dims, "time_distribution", "three_time")).unwrap_or_default();
        let name = |i: usize| -> String {
            if n <= 26 {
                ((b'A' + i as u8) as char).to_string()
            } else {
                format!("A{}", i + 1)
            }
        };
        let activities = (0..n)
            .map(|i| Activity {
                name: name(i),
                predecessors: if i == 0 { vec![] } else { vec![name(i - 1)] },
                normal_time: (t == PertType::Deterministic).then_some(1.0),
                distribution: (td == Dist::Mixed).then_some(Dist::ThreeTime),
                ..Default::default()
            })
            .collect();
        Ok(PertModel {
            name: format!("{} nuevo", if t == PertType::Deterministic { "Proyecto CPM" } else { "Proyecto PERT" }),
            problem_type: t,
            time_unit: if unit.trim().is_empty() { "semana".into() } else { unit },
            time_distribution: td,
            activities,
            positions: None,
            graphic: false,
        })
    }

    fn solve(&self, m: &PertModel, o: &PertOptions) -> Result<Report, ModError> {
        m.validate()?;
        let mut rep = match m.problem_type {
            PertType::Deterministic => cpm::solve_cpm(m, o)?,
            PertType::Probabilistic => prob::solve_pert(m, o)?,
        };
        if rep.problem.is_empty() {
            rep.problem = m.name.clone();
        }
        Ok(rep)
    }

    fn read(&self, bytes: &[u8]) -> Result<PertModel, ModError> {
        format::read(bytes)
    }

    fn write(&self, m: &PertModel) -> Result<Vec<u8>, ModError> {
        Ok(format::write(m))
    }

    fn examples(&self) -> Vec<(&'static str, PertModel)> {
        let ex = |s: &str| serde_json::from_str::<PertModel>(s).unwrap();
        vec![
            ("proyecto_cpm", ex(include_str!("ejemplos/reliable.json"))),
            ("proyecto_pert", ex(include_str!("ejemplos/reliable_pert.json"))),
            ("control_costos", ex(include_str!("ejemplos/control.json"))),
        ]
    }
}
