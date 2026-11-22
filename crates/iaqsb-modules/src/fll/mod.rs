//! FLL · Localización y distribución de instalaciones (equivalente al módulo FLL de WinQSB).
//!
//! - Localización de instalaciones nuevas respecto de las existentes, con
//!   distancia rectilínea (programación lineal, exacta), euclidiana (Weiszfeld)
//!   o euclidiana al cuadrado (sistema lineal, exacta); en 2 o 3 dimensiones.
//! - Distribución funcional por intercambios de departamentos (2, 3, 2 y luego
//!   3, 3 y luego 2), al estilo de CRAFT.
//! - Balanceo de líneas con diez reglas heurísticas, búsqueda del mejor
//!   (ramificación y acotamiento) y COMSOAL.

mod bal;
mod lay;
mod loc;

use iaqsb_core::winqsb::{num, parse_num, Reader, Writer};
use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum FllType {
    #[default]
    Location,
    Layout,
    LineBalancing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum FacKind {
    #[default]
    Existing,
    New,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Facility {
    pub name: String,
    #[serde(default)]
    pub kind: FacKind,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Department {
    pub name: String,
    #[serde(default)]
    pub fixed: bool,
    /// Celdas: "(3,5), (1,1)-(2,4)" (fila, columna; un rango es un rectángulo).
    #[serde(default)]
    pub cells: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Task {
    pub name: String,
    #[serde(default)]
    pub time: f64,
    /// Debe ir solo en su estación.
    #[serde(default)]
    pub isolated: bool,
    /// Sucesores inmediatos (nombres o números).
    #[serde(default)]
    pub successors: String,
}

fn two() -> u32 {
    2
}
fn minute() -> String {
    "minuto".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FllModel {
    #[serde(default)]
    pub name: String,
    pub problem_type: FllType,
    /// Minimizar (o maximizar) el costo total de flujo × distancia.
    #[serde(default)]
    pub maximize: bool,
    // localización
    #[serde(default = "two")]
    pub dims: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facilities: Vec<Facility>,
    /// Flujo × costo unitario de cada instalación a cada otra.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flows: Vec<Vec<f64>>,
    // distribución
    #[serde(default)]
    pub rows: u32,
    #[serde(default)]
    pub cols: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub departments: Vec<Department>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dept_flows: Vec<Vec<f64>>,
    // balanceo
    #[serde(default = "minute")]
    pub time_unit: String,
    #[serde(default)]
    pub cycle_time: Option<f64>,
    #[serde(default)]
    pub time_available: Option<f64>,
    #[serde(default)]
    pub production: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Distance {
    #[default]
    Rectilinear,
    Euclidean,
    Squared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Exchange {
    #[default]
    TwoWay,
    ThreeWay,
    TwoThenThree,
    ThreeThenTwo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum BalMethod {
    #[default]
    Heuristic,
    BestBud,
    Comsoal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum BalRule {
    #[default]
    Rpw,
    Lot,
    Sot,
    MostFollowers,
    LeastFollowers,
    MostImmediate,
    LeastImmediate,
    MostPredecessors,
    SmallestNumber,
    Random,
}

impl BalRule {
    pub const ALL: [BalRule; 10] = [
        BalRule::Rpw,
        BalRule::Lot,
        BalRule::Sot,
        BalRule::MostFollowers,
        BalRule::LeastFollowers,
        BalRule::MostImmediate,
        BalRule::LeastImmediate,
        BalRule::MostPredecessors,
        BalRule::SmallestNumber,
        BalRule::Random,
    ];
    pub fn id(self) -> &'static str {
        match self {
            BalRule::Rpw => "rpw",
            BalRule::Lot => "lot",
            BalRule::Sot => "sot",
            BalRule::MostFollowers => "most_followers",
            BalRule::LeastFollowers => "least_followers",
            BalRule::MostImmediate => "most_immediate",
            BalRule::LeastImmediate => "least_immediate",
            BalRule::MostPredecessors => "most_predecessors",
            BalRule::SmallestNumber => "smallest_number",
            BalRule::Random => "random",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            BalRule::Rpw => "Peso posicional (RPW)",
            BalRule::Lot => "Mayor tiempo (LOT)",
            BalRule::Sot => "Menor tiempo (SOT)",
            BalRule::MostFollowers => "Más tareas siguientes",
            BalRule::LeastFollowers => "Menos tareas siguientes",
            BalRule::MostImmediate => "Más sucesores inmediatos",
            BalRule::LeastImmediate => "Menos sucesores inmediatos",
            BalRule::MostPredecessors => "Más predecesoras",
            BalRule::SmallestNumber => "Menor número de tarea",
            BalRule::Random => "Al azar",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct FllOptions {
    pub steps: bool,
    pub distance: Distance,
    pub exchange: Exchange,
    pub method: BalMethod,
    pub rule: BalRule,
    pub tie_breaker: BalRule,
    pub generations: u32,
    pub seed: u64,
}

impl Default for FllOptions {
    fn default() -> Self {
        FllOptions { steps: true, distance: Distance::Rectilinear, exchange: Exchange::TwoWay, method: BalMethod::Heuristic, rule: BalRule::Rpw, tie_breaker: BalRule::Lot, generations: 1000, seed: 27437 }
    }
}

pub fn dist(d: Distance, a: &[f64], b: &[f64]) -> f64 {
    match d {
        Distance::Rectilinear => a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum(),
        Distance::Euclidean => a.iter().zip(b).map(|(x, y)| (x - y).powi(2)).sum::<f64>().sqrt(),
        Distance::Squared => a.iter().zip(b).map(|(x, y)| (x - y).powi(2)).sum(),
    }
}

pub fn dist_label(d: Distance) -> &'static str {
    match d {
        Distance::Rectilinear => "rectilínea",
        Distance::Euclidean => "euclidiana",
        Distance::Squared => "euclidiana al cuadrado",
    }
}

pub struct Fll;

impl TypedModule for Fll {
    type Model = FllModel;
    type Options = FllOptions;

    fn file_tag(&self) -> &'static str {
        "FLL"
    }

    fn spec(&self) -> ModuleSpec {
        let rules: Vec<(&str, &str)> = BalRule::ALL.iter().map(|r| (r.id(), r.label())).collect();
        let mut dept_axis = Axis::new("departments", "Desde").add("Añadir departamento");
        dept_axis = dept_axis.extra(Field::boolean("fixed", "Fijo", false)).extra(Field::text("cells", "Celdas iniciales").help("(fila,col) o rangos (1,1)-(2,4)"));
        ModuleSpec {
            id: "fll".into(),
            winqsb: "FLL".into(),
            name: "Localización y distribución de instalaciones".into(),
            description: "Localización con tres distancias, distribución por intercambios y balanceo de líneas".into(),
            file_ext: "FLL".into(),
            problem_types: vec![
                ProblemType {
                    id: "location".into(),
                    label: "Localización de instalaciones".into(),
                    description: "Dónde poner instalaciones nuevas para minimizar flujo × distancia".into(),
                    dims: vec![Field::integer("existing", "Instalaciones existentes", 4), Field::integer("new", "Instalaciones nuevas", 1), Field::integer("dims", "Dimensiones (2 o 3)", 2)],
                    layout: vec![
                        Widget::params(None, vec![Field::integer("dims", "Dimensiones (2 o 3)", 2), Field::boolean("maximize", "Maximizar (instalación indeseable)", false)]),
                        Widget::records(
                            Some("Instalaciones"),
                            "facilities",
                            vec![
                                Field::text("name", "Instalación"),
                                Field::choice("kind", "Tipo", &[("existing", "Existente"), ("new", "Nueva")]),
                                Field::number("x", "X").show_if("kind", &["existing"]),
                                Field::number("y", "Y").show_if("kind", &["existing"]),
                                Field::number("z", "Z").show_if("kind", &["existing"]).show_if("dims", &["3"]),
                            ],
                            Some("Añadir instalación"),
                        ),
                        Widget::matrix(Some("Flujo × costo unitario (desde × hacia)"), "flows", Axis::new("facilities", "Desde"), Axis::new("facilities", "Hacia"), Field::number("f", "Flujo")),
                    ],
                },
                ProblemType {
                    id: "layout".into(),
                    label: "Distribución funcional".into(),
                    description: "Ubicar departamentos en una cuadrícula para minimizar flujo × distancia".into(),
                    dims: vec![Field::integer("departments", "Departamentos", 4), Field::integer("rows", "Filas del área", 4), Field::integer("cols", "Columnas del área", 4)],
                    layout: vec![
                        Widget::params(None, vec![Field::integer("rows", "Filas del área", 4), Field::integer("cols", "Columnas del área", 4), Field::boolean("maximize", "Maximizar", false)]),
                        Widget::matrix(Some("Departamentos: flujo × costo unitario y distribución inicial"), "dept_flows", dept_axis, Axis::new("departments", "Hacia"), Field::number("f", "Flujo")),
                    ],
                },
                ProblemType {
                    id: "line_balancing".into(),
                    label: "Balanceo de líneas".into(),
                    description: "Agrupar tareas en estaciones sin pasar el tiempo de ciclo".into(),
                    dims: vec![Field::integer("tasks", "Número de tareas", 8)],
                    layout: vec![
                        Widget::params(
                            Some("Línea"),
                            vec![
                                Field::text("time_unit", "Unidad de tiempo"),
                                Field::number("cycle_time", "Tiempo de ciclo").optional().help("O bien tiempo disponible y producción"),
                                Field::number("time_available", "Tiempo disponible").optional(),
                                Field::number("production", "Producción deseada").optional(),
                            ],
                        ),
                        Widget::records(
                            Some("Tareas"),
                            "tasks",
                            vec![Field::text("name", "Tarea"), Field::number("time", "Tiempo"), Field::boolean("isolated", "Aislada", false), Field::list("successors", "Sucesores inmediatos").help("Nombres o números separados por comas")],
                            Some("Añadir tarea"),
                        ),
                    ],
                },
            ],
            options: vec![
                Field::boolean("steps", "Mostrar paso a paso", true),
                Field::choice("distance", "Distancia", &[("rectilinear", "Rectilínea"), ("euclidean", "Euclidiana"), ("squared", "Euclidiana al cuadrado")]).default("rectilinear").show_if("problem_type", &["location", "layout"]),
                Field::choice("exchange", "Intercambios", &[("two_way", "De 2"), ("three_way", "De 3"), ("two_then_three", "De 2 y luego de 3"), ("three_then_two", "De 3 y luego de 2")]).default("two_way").show_if("problem_type", &["layout"]),
                Field::choice("method", "Método", &[("heuristic", "Regla heurística"), ("best_bud", "Búsqueda del mejor"), ("comsoal", "COMSOAL (aleatorio)")]).comparable().default("heuristic").show_if("problem_type", &["line_balancing"]),
                Field::choice("rule", "Regla", &rules).comparable().default("rpw").show_if("problem_type", &["line_balancing"]).show_if("method", &["heuristic"]),
                Field::choice("tie_breaker", "Desempate", &rules).default("lot").show_if("problem_type", &["line_balancing"]).show_if("method", &["heuristic"]),
                Field::integer("generations", "Soluciones generadas", 1000).show_if("problem_type", &["line_balancing"]).show_if("method", &["comsoal"]),
                Field::integer("seed", "Semilla", 27437).show_if("problem_type", &["line_balancing"]),
            ],
        }
    }

    fn template(&self, pt: &str, dims: &Value) -> Result<FllModel, ModError> {
        let t: FllType = serde_json::from_value(Value::from(pt)).map_err(|_| ModError::Invalid(format!("tipo de problema desconocido: {pt}")))?;
        let mut m: FllModel = serde_json::from_value(json!({"name": "Problema nuevo", "problem_type": t})).unwrap();
        match t {
            FllType::Location => {
                let e = dim(dims, "existing", 4, 1, 500)?;
                let n = dim(dims, "new", 1, 1, 100)?;
                m.dims = dim(dims, "dims", 2, 2, 3)? as u32;
                m.facilities = (1..=e).map(|i| Facility { name: format!("E{i}"), kind: FacKind::Existing, x: 0.0, y: 0.0, z: 0.0 }).chain((1..=n).map(|i| Facility { name: format!("N{i}"), kind: FacKind::New, x: 0.0, y: 0.0, z: 0.0 })).collect();
                m.flows = vec![vec![0.0; e + n]; e + n];
            }
            FllType::Layout => {
                let d = dim(dims, "departments", 4, 1, 200)?;
                m.rows = dim(dims, "rows", 4, 1, 200)? as u32;
                m.cols = dim(dims, "cols", 4, 1, 200)? as u32;
                m.departments = (1..=d).map(|i| Department { name: format!("D{i}"), fixed: false, cells: String::new() }).collect();
                m.dept_flows = vec![vec![0.0; d]; d];
            }
            FllType::LineBalancing => {
                let n = dim(dims, "tasks", 8, 1, 1000)?;
                m.tasks = (1..=n).map(|i| Task { name: format!("T{i}"), time: 1.0, isolated: false, successors: if i < n { (i + 1).to_string() } else { String::new() } }).collect();
                m.cycle_time = Some(5.0);
            }
        }
        Ok(m)
    }

    fn solve(&self, m: &FllModel, o: &FllOptions) -> Result<Report, ModError> {
        let mut r = match m.problem_type {
            FllType::Location => loc::solve(m, o)?,
            FllType::Layout => lay::solve(m, o)?,
            FllType::LineBalancing => bal::solve(m, o)?,
        };
        if r.problem.is_empty() {
            r.problem = m.name.clone();
        }
        Ok(r)
    }

    fn read(&self, bytes: &[u8]) -> Result<FllModel, ModError> {
        read_fll(bytes)
    }

    fn write(&self, m: &FllModel) -> Result<Vec<u8>, ModError> {
        Ok(write_fll(m))
    }

    fn examples(&self) -> Vec<(&'static str, FllModel)> {
        let ex = |s: &str| serde_json::from_str::<FllModel>(s).unwrap();
        vec![
            ("almacen_nuevo", ex(include_str!("ejemplos/localizacion.json"))),
            ("dos_plantas_nuevas", ex(include_str!("ejemplos/dos_plantas.json"))),
            ("distribucion_oficinas", ex(include_str!("ejemplos/distribucion.json"))),
            ("balanceo_linea", ex(include_str!("ejemplos/balanceo.json"))),
        ]
    }
}

// ------------------------------------------------------------------ archivos

fn write_fll(m: &FllModel) -> Vec<u8> {
    let mut w = Writer::new();
    let obj = if m.maximize { "MAX" } else { "MIN" };
    match m.problem_type {
        FllType::Location => {
            let ex: Vec<usize> = (0..m.facilities.len()).filter(|&i| m.facilities[i].kind == FacKind::Existing).collect();
            let nw: Vec<usize> = (0..m.facilities.len()).filter(|&i| m.facilities[i].kind == FacKind::New).collect();
            let order: Vec<usize> = ex.iter().chain(nw.iter()).copied().collect();
            w.row(&["FLL".to_string(), m.name.clone(), "LOCATION".into(), ex.len().to_string(), nw.len().to_string(), m.dims.to_string(), obj.into()]);
            let mut h = vec!["Facility Number".to_string(), "Facility Name".to_string()];
            h.extend((1..=ex.len()).map(|k| format!("To Existing {k} Flow/Unit Cost")));
            h.extend((1..=nw.len()).map(|k| format!("To New {k} Flow/Unit Cost")));
            h.extend(["Location X Axis".to_string(), "Location Y Axis".to_string()]);
            if m.dims == 3 {
                h.push("Location Z Axis".into());
            }
            w.row(&h);
            for (k, &i) in order.iter().enumerate() {
                let f = &m.facilities[i];
                let mut r = vec![(k + 1).to_string(), f.name.clone()];
                r.extend(order.iter().map(|&j| num(m.flows.get(i).and_then(|x| x.get(j)).copied().unwrap_or(0.0))));
                let new = f.kind == FacKind::New;
                r.push(if new { " ".into() } else { num(f.x) });
                r.push(if new { " ".into() } else { num(f.y) });
                if m.dims == 3 {
                    r.push(if new { " ".into() } else { num(f.z) });
                }
                w.row(&r);
            }
        }
        FllType::Layout => {
            let n = m.departments.len();
            w.row(&["FLL".to_string(), m.name.clone(), "LAYOUT".into(), n.to_string(), m.rows.to_string(), m.cols.to_string(), obj.into()]);
            let mut h = vec!["Department Number".to_string(), "Department Name".to_string(), "Location Fixed".to_string()];
            h.extend((1..=n).map(|k| format!("To Dep. {k} Flow/Unit Cost")));
            h.push("Initial Layout in Cell Locations".into());
            w.row(&h);
            for (i, d) in m.departments.iter().enumerate() {
                let mut r = vec![(i + 1).to_string(), d.name.clone(), if d.fixed { "Yes" } else { "No" }.into()];
                r.extend((0..n).map(|j| num(m.dept_flows.get(i).and_then(|x| x.get(j)).copied().unwrap_or(0.0))));
                r.push(d.cells.clone());
                w.row(&r);
            }
        }
        FllType::LineBalancing => {
            let o = |x: Option<f64>| x.map(num).unwrap_or_else(|| " ".into());
            w.row(&["FLL".to_string(), m.name.clone(), "LINE BALANCING".into(), m.tasks.len().to_string(), m.time_unit.clone(), o(m.cycle_time), o(m.time_available), o(m.production)]);
            w.row(&["Task Number", "Task Name", "Task Time", "Task Isolated (Y/N)", "Immediate Successor"]);
            let names: Vec<String> = m.tasks.iter().map(|t| t.name.to_ascii_uppercase()).collect();
            for (i, t) in m.tasks.iter().enumerate() {
                // los sucesores se guardan por número, como WinQSB
                let succ: Vec<String> = t
                    .successors
                    .split([',', ';'])
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| names.iter().position(|x| *x == s.to_ascii_uppercase()).map(|k| (k + 1).to_string()).unwrap_or_else(|| s.to_string()))
                    .collect();
                w.row(&[(i + 1).to_string(), t.name.clone(), num(t.time), if t.isolated { "Yes" } else { "No" }.into(), succ.join(",")]);
            }
        }
    }
    w.bytes()
}

fn read_fll(bytes: &[u8]) -> Result<FllModel, ModError> {
    let mut r = Reader::new(bytes);
    let h = r.line("la cabecera")?;
    if !h.get(0).trim().to_ascii_uppercase().starts_with("FLL") {
        return Err(ModError::Format("no es un archivo de localización y distribución".into()));
    }
    let name = Some(h.get(1).trim()).filter(|s| !s.is_empty()).unwrap_or("FLL Problem").to_string();
    let kind = h.get(2).trim().to_ascii_uppercase();
    let g = |i: usize| parse_num(h.get(i)).unwrap_or(0.0);
    let mut m: FllModel = serde_json::from_value(json!({"name": name, "problem_type": "location"})).unwrap();
    if kind.starts_with("LOC") {
        let (ne, nn) = (g(3) as usize, g(4) as usize);
        let d = if (2.0..=3.0).contains(&g(5)) { g(5) as usize } else { 2 };
        m.dims = d as u32;
        m.maximize = h.get(6).trim().to_ascii_uppercase().starts_with("MAX");
        if ne + nn == 0 || ne + nn > 100_000 {
            return Err(ModError::Format("el número de instalaciones no es válido".into()));
        }
        r.line("la fila de títulos")?;
        let n = ne + nn;
        for i in 0..n {
            let l = r.line(&format!("la instalación {}", i + 1))?;
            let new = i >= ne;
            let nm = Some(l.get(1).trim()).filter(|s| !s.is_empty()).map(str::to_string).unwrap_or(if new { format!("New{}", i - ne + 1) } else { format!("Existing{}", i + 1) });
            m.flows.push((0..n).map(|j| parse_num(l.get(2 + j)).unwrap_or(0.0)).collect());
            let c = |k: usize| parse_num(l.get(2 + n + k)).unwrap_or(0.0);
            m.facilities.push(Facility { name: nm, kind: if new { FacKind::New } else { FacKind::Existing }, x: c(0), y: c(1), z: if d == 3 { c(2) } else { 0.0 } });
        }
    } else if kind.starts_with("LAY") {
        m.problem_type = FllType::Layout;
        let n = g(3) as usize;
        m.rows = g(4) as u32;
        m.cols = g(5) as u32;
        m.maximize = h.get(6).trim().to_ascii_uppercase().starts_with("MAX");
        if n == 0 || n > 100_000 {
            return Err(ModError::Format("el número de departamentos no es válido".into()));
        }
        r.line("la fila de títulos")?;
        for i in 0..n {
            let l = r.line(&format!("el departamento {}", i + 1))?;
            m.dept_flows.push((0..n).map(|j| parse_num(l.get(3 + j)).unwrap_or(0.0)).collect());
            m.departments.push(Department {
                name: Some(l.get(1).trim()).filter(|s| !s.is_empty()).map(str::to_string).unwrap_or(format!("Dep{}", i + 1)),
                fixed: l.get(2).trim().to_ascii_uppercase().starts_with('Y'),
                cells: l.get(3 + n).trim().to_string(),
            });
        }
    } else {
        m.problem_type = FllType::LineBalancing;
        let n = g(3) as usize;
        if n == 0 || n > 100_000 {
            return Err(ModError::Format("el número de tareas no es válido".into()));
        }
        m.time_unit = Some(h.get(4).trim()).filter(|s| !s.is_empty()).unwrap_or("minute").to_string();
        let pos = |i: usize| parse_num(h.get(i)).filter(|x| *x > 0.0);
        m.cycle_time = pos(5);
        m.time_available = pos(6);
        m.production = pos(7);
        r.line("la fila de títulos")?;
        for i in 0..n {
            let l = r.line(&format!("la tarea {}", i + 1))?;
            m.tasks.push(Task {
                name: Some(l.get(1).trim()).filter(|s| !s.is_empty()).map(str::to_string).unwrap_or(format!("Task{}", i + 1)),
                time: parse_num(l.get(2)).unwrap_or(0.0),
                isolated: l.get(3).trim().to_ascii_uppercase().starts_with('Y'),
                successors: l.get(4).trim().trim_matches('"').to_string(),
            });
        }
        // WinQSB guarda los sucesores por número: se pasan a nombres
        let names: Vec<String> = m.tasks.iter().map(|t| t.name.clone()).collect();
        for t in m.tasks.iter_mut() {
            t.successors = t
                .successors
                .split([',', ';'])
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.parse::<usize>().ok().filter(|k| *k >= 1 && *k <= names.len()).map(|k| names[k - 1].clone()).unwrap_or_else(|| s.to_string()))
                .collect::<Vec<_>>()
                .join(", ");
        }
    }
    Ok(m)
}
