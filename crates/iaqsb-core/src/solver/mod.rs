//! Motor de optimización de IAQSB.
//!
//! Un modelo lineal, entero o cuadrático disperso ([`Model`]) se resuelve con
//! [`solve`] y unos parámetros ([`Params`]) parecidos a los de Gurobi y CPLEX:
//! límite de tiempo, brecha MIP, hilos, presolve, método, semilla y arranque con
//! una solución. El motor por defecto es HiGHS (enlazado dentro de IAQSB, sin
//! licencia); si en el equipo están Gurobi, CPLEX, SCIP, CBC o GLPK, se pueden
//! usar con `params.solver` (ver [`external`]).
//!
//! Además: análisis de sensibilidad por rangos ([`ranging`]), subsistema
//! irreducible infactible ([`iis`]), relajación de factibilidad
//! ([`feasibility_relaxation`]), varias funciones objetivo por prioridad y peso
//! (como los objetivos múltiples de Gurobi) y lectura y escritura de los
//! formatos MPS y LP ([`read_text`], [`write_text`]).

pub mod external;
pub mod tune;
mod highs;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

pub use highs::version as highs_version;

/// Tipo de variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    #[default]
    Continuous,
    Integer,
    Binary,
    /// 0 o entre sus cotas [lb, ub] (lb > 0): p. ej. producir nada o al menos un lote.
    SemiContinuous,
    /// 0 o un entero entre sus cotas.
    SemiInteger,
}

impl Kind {
    pub fn is_integer(self) -> bool {
        matches!(self, Kind::Integer | Kind::Binary | Kind::SemiInteger)
    }
}

/// Variable (columna).
#[derive(Debug, Clone)]
pub struct Col {
    pub name: String,
    pub cost: f64,
    pub lb: f64,
    pub ub: f64,
    pub kind: Kind,
}

/// Restricción lo ≤ a·x ≤ hi (lo = hi para una igualdad; ±∞ si no hay cota).
#[derive(Debug, Clone)]
pub struct Row {
    pub name: String,
    pub coefs: Vec<(usize, f64)>,
    pub lo: f64,
    pub hi: f64,
}

/// Función objetivo adicional (modelos multiobjetivo, como en Gurobi): se
/// optimizan por prioridad (mayor primero); las de igual prioridad se suman con
/// sus pesos. Cada nivel puede ceder `rel_tol` (fracción) o `abs_tol` de su óptimo.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Objective {
    pub name: String,
    /// Coeficientes por nombre de variable.
    pub coefs: std::collections::BTreeMap<String, f64>,
    /// true = maximizar esta función.
    pub maximize: bool,
    pub priority: i32,
    pub weight: f64,
    pub rel_tol: f64,
    pub abs_tol: f64,
}

impl Default for Objective {
    fn default() -> Self {
        Objective { name: String::new(), coefs: Default::default(), maximize: false, priority: 0, weight: 1.0, rel_tol: 0.0, abs_tol: 0.0 }
    }
}

/// Modelo de optimización disperso.
#[derive(Debug, Clone, Default)]
pub struct Model {
    pub name: String,
    pub maximize: bool,
    pub offset: f64,
    pub cols: Vec<Col>,
    pub rows: Vec<Row>,
    /// Parte cuadrática del objetivo: c·x + ½·xᵀQx; triángulo inferior (i ≥ j).
    pub q: Vec<(usize, usize, f64)>,
    /// Objetivos adicionales (si hay, reemplazan a `cost`).
    pub objectives: Vec<Objective>,
}

impl Model {
    pub fn add_col(&mut self, name: impl Into<String>, cost: f64, lb: f64, ub: f64, kind: Kind) -> usize {
        self.cols.push(Col { name: name.into(), cost, lb, ub, kind });
        self.cols.len() - 1
    }
    pub fn add_row(&mut self, name: impl Into<String>, coefs: Vec<(usize, f64)>, lo: f64, hi: f64) -> usize {
        self.rows.push(Row { name: name.into(), coefs, lo, hi });
        self.rows.len() - 1
    }
    pub fn is_mip(&self) -> bool {
        self.cols.iter().any(|c| c.kind != Kind::Continuous)
    }
    pub fn nonzeros(&self) -> usize {
        self.rows.iter().map(|r| r.coefs.len()).sum()
    }
    /// Valor del objetivo en x.
    pub fn objective_at(&self, x: &[f64]) -> f64 {
        let lin: f64 = self.cols.iter().zip(x).map(|(c, v)| c.cost * v).sum();
        let quad: f64 = self.q.iter().map(|&(i, j, v)| if i == j { 0.5 * v * x[i] * x[i] } else { v * x[i] * x[j] }).sum();
        self.offset + lin + quad
    }
    /// Actividad de cada fila en x.
    pub fn activity(&self, x: &[f64]) -> Vec<f64> {
        self.rows.iter().map(|r| r.coefs.iter().map(|&(j, a)| a * x[j]).sum()).collect()
    }
}

/// Algoritmo para los problemas continuos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// El motor elige.
    #[default]
    Auto,
    /// Símplex dual.
    DualSimplex,
    /// Símplex primal.
    PrimalSimplex,
    /// Punto interior (barrera) con cruce a una solución básica.
    Barrier,
    /// Primal-dual de primer orden (PDLP), para modelos enormes; menos preciso.
    Pdlp,
}

/// Parámetros del motor. Todos opcionales: lo que no se diga lo decide el motor.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(default)]
pub struct Params {
    /// Motor: "highs" (por defecto, incluido), "gurobi", "cplex", "scip", "cbc" o "glpk"
    /// (estos, si están instalados en el equipo; ver solver_info).
    pub solver: Option<String>,
    /// Segundos como máximo. Al llegar al límite se devuelve la mejor solución encontrada.
    pub time_limit: Option<f64>,
    /// Brecha relativa MIP aceptada (0.01 = 1 %): para cuando |mejor − cota| / |mejor| ≤ brecha.
    pub mip_gap: Option<f64>,
    /// Brecha absoluta MIP aceptada.
    pub mip_abs_gap: Option<f64>,
    /// Hilos de cálculo (por defecto, los que decida el motor).
    pub threads: Option<u32>,
    /// Presolve (simplificación previa del modelo). Por defecto, el motor decide.
    pub presolve: Option<bool>,
    /// Algoritmo para los problemas continuos y las relajaciones.
    pub method: Option<Method>,
    /// Semilla aleatoria (para repetir exactamente una corrida).
    pub seed: Option<i64>,
    /// Máximo de nodos de ramificación y acotamiento.
    pub node_limit: Option<i64>,
    /// Máximo de soluciones enteras encontradas antes de parar.
    pub solution_limit: Option<i64>,
    /// Tolerancia de factibilidad primal.
    pub feasibility_tol: Option<f64>,
    /// Tolerancia de integralidad.
    pub integrality_tol: Option<f64>,
    /// Devolver el registro (log) del motor.
    pub log: bool,
    /// Solución inicial (arranque MIP) por nombre de variable.
    pub start: Option<std::collections::BTreeMap<String, f64>>,
    /// Opciones propias de HiGHS por su nombre (p. ej. {"mip_heuristic_effort": 0.2,
    /// "mip_detect_symmetry": false}); las da también el ajuste automático.
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub raw: std::collections::BTreeMap<String, serde_json::Value>,
}

impl Params {
    /// Combina: lo de `self` manda y lo que falte sale de `base`.
    pub fn or(&self, base: &Params) -> Params {
        Params {
            solver: self.solver.clone().or_else(|| base.solver.clone()),
            time_limit: self.time_limit.or(base.time_limit),
            mip_gap: self.mip_gap.or(base.mip_gap),
            mip_abs_gap: self.mip_abs_gap.or(base.mip_abs_gap),
            threads: self.threads.or(base.threads),
            presolve: self.presolve.or(base.presolve),
            method: self.method.or(base.method),
            seed: self.seed.or(base.seed),
            node_limit: self.node_limit.or(base.node_limit),
            solution_limit: self.solution_limit.or(base.solution_limit),
            feasibility_tol: self.feasibility_tol.or(base.feasibility_tol),
            integrality_tol: self.integrality_tol.or(base.integrality_tol),
            log: self.log || base.log,
            start: self.start.clone().or_else(|| base.start.clone()),
            raw: {
                let mut r = base.raw.clone();
                r.extend(self.raw.clone());
                r
            },
        }
    }
}

thread_local! {
    static DEFAULTS: RefCell<Params> = RefCell::new(Params::default());
    static LAST: RefCell<Vec<Solution>> = const { RefCell::new(Vec::new()) };
}

/// Fija los parámetros por defecto de las resoluciones de este hilo (los usan
/// todos los módulos que resuelven un programa lineal o entero por dentro).
pub fn set_defaults(p: Params) {
    DEFAULTS.with(|d| *d.borrow_mut() = p);
}

pub fn defaults() -> Params {
    DEFAULTS.with(|d| d.borrow().clone())
}

/// Resúmenes de las resoluciones hechas desde la última llamada (para que un
/// informe diga qué motor usó, cuánto tardó y con qué brecha).
pub fn take_runs() -> Vec<Solution> {
    LAST.with(|l| std::mem::take(&mut *l.borrow_mut()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Optimal,
    Infeasible,
    Unbounded,
    InfeasibleOrUnbounded,
    /// Se acabó el tiempo (puede haber una solución factible).
    TimeLimit,
    IterationLimit,
    NodeLimit,
    SolutionLimit,
    Interrupted,
    Error,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Optimal => "Óptimo",
            Status::Infeasible => "No factible",
            Status::Unbounded => "No acotado",
            Status::InfeasibleOrUnbounded => "No factible o no acotado",
            Status::TimeLimit => "Límite de tiempo",
            Status::IterationLimit => "Límite de iteraciones",
            Status::NodeLimit => "Límite de nodos",
            Status::SolutionLimit => "Límite de soluciones",
            Status::Interrupted => "Interrumpido",
            Status::Error => "Error del motor",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BasisStatus {
    Lower,
    Basic,
    Upper,
    Zero,
    Nonbasic,
}

/// Resultado de una resolución.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Solution {
    pub status: Status,
    /// Hay una solución factible en `x` (óptima o la mejor encontrada).
    pub has_solution: bool,
    pub objective: Option<f64>,
    /// Mejor cota conocida del óptimo (MIP: cota dual del árbol).
    pub bound: Option<f64>,
    /// Brecha relativa |objetivo − cota| / |objetivo| (MIP).
    pub gap: Option<f64>,
    pub x: Vec<f64>,
    pub row_activity: Vec<f64>,
    /// Precios sombra (problemas continuos).
    pub row_dual: Option<Vec<f64>>,
    /// Costos reducidos (problemas continuos).
    pub reduced_cost: Option<Vec<f64>>,
    pub col_basis: Option<Vec<BasisStatus>>,
    pub row_basis: Option<Vec<BasisStatus>>,
    pub iterations: i64,
    pub nodes: i64,
    /// Segundos.
    pub time: f64,
    /// Motor y versión ("HiGHS 1.12.0", "Gurobi 13.0.3"…).
    pub solver: String,
    pub is_mip: bool,
    pub cols: usize,
    pub rows: usize,
    pub nonzeros: usize,
    /// Valor de cada objetivo de un modelo multiobjetivo.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objective_values: Vec<(String, f64)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log: Option<String>,
    /// Aviso o mensaje del motor.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message: String,
}

impl Solution {
    pub fn empty(m: &Model, status: Status, solver: &str, message: impl Into<String>) -> Solution {
        Solution {
            status,
            has_solution: false,
            objective: None,
            bound: None,
            gap: None,
            x: vec![0.0; m.cols.len()],
            row_activity: vec![0.0; m.rows.len()],
            row_dual: None,
            reduced_cost: None,
            col_basis: None,
            row_basis: None,
            iterations: 0,
            nodes: 0,
            time: 0.0,
            solver: solver.into(),
            is_mip: m.is_mip(),
            cols: m.cols.len(),
            rows: m.rows.len(),
            nonzeros: m.nonzeros(),
            objective_values: vec![],
            log: None,
            message: message.into(),
        }
    }

    /// Resumen corto del motor (para los informes).
    pub fn summary(&self) -> serde_json::Value {
        serde_json::json!({
            "solver": self.solver, "status": self.status, "time": (self.time * 1e4).round() / 1e4,
            "objective": self.objective, "bound": self.bound, "gap": self.gap,
            "nodes": self.nodes, "iterations": self.iterations,
            "cols": self.cols, "rows": self.rows, "nonzeros": self.nonzeros, "is_mip": self.is_mip,
        })
    }
}

/// Resuelve el modelo con los parámetros dados (completados con los del hilo).
pub fn solve(m: &Model, p: &Params) -> Solution {
    let p = p.or(&defaults());
    let sol = if m.objectives.is_empty() { solve_single(m, &p) } else { solve_multi(m, &p) };
    LAST.with(|l| {
        let mut l = l.borrow_mut();
        if l.len() < 200 {
            let mut s = sol.clone();
            s.x.clear();
            s.row_activity.clear();
            s.row_dual = None;
            s.reduced_cost = None;
            s.col_basis = None;
            s.row_basis = None;
            s.log = None;
            l.push(s);
        }
    });
    sol
}

fn solve_single(m: &Model, p: &Params) -> Solution {
    match p.solver.as_deref().map(|s| s.trim().to_lowercase()) {
        None => highs::solve(m, p),
        Some(s) if s.is_empty() || s == "auto" || s == "highs" => highs::solve(m, p),
        Some(s) => external::solve(&s, m, p),
    }
}

/// Multiobjetivo jerárquico: por prioridad descendente; cada nivel fija su
/// óptimo (con su tolerancia) como restricción para los siguientes.
fn solve_multi(m: &Model, p: &Params) -> Solution {
    let index: std::collections::HashMap<&str, usize> = m.cols.iter().enumerate().map(|(j, c)| (c.name.as_str(), j)).collect();
    let mut prios: Vec<i32> = m.objectives.iter().map(|o| o.priority).collect();
    prios.sort_unstable_by(|a, b| b.cmp(a));
    prios.dedup();
    let mut work = m.clone();
    work.objectives.clear();
    work.q.clear();
    let mut last: Option<Solution> = None;
    let mut time = 0.0;
    for pr in prios {
        // objetivo del nivel: suma ponderada, todo llevado a minimizar
        let mut c = vec![0.0; m.cols.len()];
        for o in m.objectives.iter().filter(|o| o.priority == pr) {
            let s = if o.maximize { -o.weight } else { o.weight };
            for (name, v) in &o.coefs {
                match index.get(name.as_str()) {
                    Some(&j) => c[j] += s * v,
                    None => return Solution::empty(m, Status::Error, "IAQSB", format!("el objetivo {} usa la variable {name}, que no existe", o.name)),
                }
            }
        }
        work.maximize = false;
        for (j, col) in work.cols.iter_mut().enumerate() {
            col.cost = c[j];
        }
        let s = solve_single(&work, p);
        time += s.time;
        if !s.has_solution {
            let mut s = s;
            s.message = format!("el nivel de prioridad {pr} no tiene solución: {}", s.message);
            return s;
        }
        let best = s.objective.unwrap_or(0.0);
        let tol = m
            .objectives
            .iter()
            .filter(|o| o.priority == pr)
            .map(|o| o.abs_tol.max(o.rel_tol * best.abs()))
            .fold(0.0, f64::max)
            .max(1e-9 * (1.0 + best.abs()));
        let coefs: Vec<(usize, f64)> = c.iter().enumerate().filter(|(_, v)| **v != 0.0).map(|(j, v)| (j, *v)).collect();
        work.add_row(format!("_objetivo_prioridad_{pr}"), coefs, f64::NEG_INFINITY, best + tol);
        last = Some(s);
    }
    let mut s = last.unwrap_or_else(|| Solution::empty(m, Status::Error, "IAQSB", "no hay objetivos"));
    s.time = time;
    s.row_activity.truncate(m.rows.len());
    if let Some(d) = s.row_dual.as_mut() {
        d.truncate(m.rows.len());
    }
    if let Some(b) = s.row_basis.as_mut() {
        b.truncate(m.rows.len());
    }
    s.rows = m.rows.len();
    s.objective_values = m
        .objectives
        .iter()
        .map(|o| {
            let v: f64 = o.coefs.iter().filter_map(|(n, a)| index.get(n.as_str()).map(|&j| a * s.x[j])).sum();
            (o.name.clone(), v)
        })
        .collect();
    s.objective = Some(m.objective_at(&s.x));
    s
}

/// Rango de un coeficiente o de un lado derecho en el que la base sigue óptima.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Range {
    pub low: f64,
    pub high: f64,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Ranging {
    /// Por variable: rango de su costo.
    pub cost: Vec<Range>,
    /// Por restricción: rango de su cota activa (lado derecho).
    pub rhs: Vec<Range>,
}

/// Análisis de sensibilidad por rangos (problemas continuos con solución óptima).
pub fn ranging(m: &Model, p: &Params) -> Option<Ranging> {
    if m.is_mip() || !m.q.is_empty() {
        return None;
    }
    highs::ranging(m, &p.or(&defaults()))
}

/// Qué cota interviene en el conflicto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IisBound {
    Lower,
    Upper,
    Both,
    Free,
}

/// Subsistema irreducible infactible: un conjunto mínimo de restricciones y
/// cotas que ya no se pueden cumplir a la vez (quitar cualquiera lo arregla).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Iis {
    pub rows: Vec<(usize, IisBound)>,
    pub cols: Vec<(usize, IisBound)>,
}

/// Calcula un IIS de un problema lineal no factible (de un MIP, el de su relajación).
pub fn iis(m: &Model) -> Option<Iis> {
    let mut lp = m.clone();
    lp.objectives.clear();
    lp.q.clear();
    for c in &mut lp.cols {
        if c.kind == Kind::Binary {
            c.lb = c.lb.max(0.0);
            c.ub = c.ub.min(1.0);
        }
        if matches!(c.kind, Kind::SemiContinuous | Kind::SemiInteger) {
            c.lb = c.lb.min(0.0);
        }
        c.kind = Kind::Continuous;
    }
    highs::iis(&lp)
}

/// Violación de una restricción o cota en la relajación de factibilidad.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Violation {
    /// "row" o "col".
    pub kind: String,
    pub index: usize,
    pub name: String,
    /// Cuánto falta (>0) o sobra para cumplirla.
    pub amount: f64,
    /// "lower" (no llega a su mínimo) o "upper" (pasa su máximo).
    pub side: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Relaxation {
    /// Suma ponderada de las violaciones (lo mínimo posible).
    pub total: f64,
    pub violations: Vec<Violation>,
    pub x: Vec<f64>,
}

/// Relajación de factibilidad (feasRelax de Gurobi): la solución que menos
/// viola las restricciones (`rhs_penalty`) y las cotas (`bound_penalty`; < 0 =
/// las cotas no se pueden violar).
pub fn feasibility_relaxation(m: &Model, rhs_penalty: f64, bound_penalty: f64) -> Option<Relaxation> {
    // solo importa la violación: sin objetivo y minimizando (HiGHS conserva el sentido
    // del modelo, y en uno de maximizar buscaría la mayor violación)
    let mut work = m.clone();
    work.objectives.clear();
    work.q.clear();
    work.maximize = false;
    work.offset = 0.0;
    for c in &mut work.cols {
        c.cost = 0.0;
    }
    highs::feasibility_relaxation(&work, rhs_penalty, bound_penalty)
}

/// Formato de archivo de modelo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// MPS libre (el de Gurobi, CPLEX y HiGHS).
    Mps,
    /// Formato LP de CPLEX («Maximize obj: 3 x + 5 y Subject To …»).
    Lp,
}

impl Format {
    pub fn parse(s: &str) -> Option<Format> {
        match s.trim().trim_start_matches('.').to_lowercase().as_str() {
            "mps" | "freemps" => Some(Format::Mps),
            "lp" | "cplex" | "cplex_lp" => Some(Format::Lp),
            _ => None,
        }
    }
    pub fn ext(self) -> &'static str {
        match self {
            Format::Mps => "mps",
            Format::Lp => "lp",
        }
    }
}

/// Lee un modelo en MPS o LP.
pub fn read_text(text: &str, format: Format) -> Result<Model, String> {
    highs::read_text(text, format)
}

/// Escribe el modelo en MPS o LP (los nombres se ajustan si el formato lo pide).
pub fn write_text(m: &Model, format: Format) -> Result<String, String> {
    highs::write_text(m, format)
}

/// Archivo temporal único en la carpeta temporal del sistema.
pub(crate) fn temp_path(ext: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let k = N.fetch_add(1, Ordering::Relaxed);
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    std::env::temp_dir().join(format!("iaqsb-{}-{t}-{k}.{ext}", std::process::id()))
}

/// Nombres válidos y únicos para MPS/LP (sin espacios ni caracteres especiales).
pub(crate) fn safe_names<'a>(names: impl Iterator<Item = &'a str>, prefix: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    names
        .enumerate()
        .map(|(i, n)| {
            let mut s: String = n
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() || "_.[]".contains(c) { c } else { '_' })
                .collect();
            if s.is_empty() || s.chars().next().map_or(true, |c| c.is_ascii_digit() || c == '.') {
                s = format!("{prefix}{s}");
            }
            if s.len() > 200 {
                s.truncate(200);
            }
            let mut out = s.clone();
            let mut k = 1;
            while !seen.insert(out.to_lowercase()) {
                out = format!("{s}_{i}_{k}");
                k += 1;
            }
            out
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wyndor() -> Model {
        let mut m = Model { name: "Wyndor".into(), maximize: true, ..Default::default() };
        let x = m.add_col("Puertas", 3.0, 0.0, f64::INFINITY, Kind::Continuous);
        let y = m.add_col("Ventanas", 5.0, 0.0, f64::INFINITY, Kind::Continuous);
        m.add_row("Planta 1", vec![(x, 1.0)], f64::NEG_INFINITY, 4.0);
        m.add_row("Planta 2", vec![(y, 2.0)], f64::NEG_INFINITY, 12.0);
        m.add_row("Planta 3", vec![(x, 3.0), (y, 2.0)], f64::NEG_INFINITY, 18.0);
        m
    }

    #[test]
    fn lineal_con_duales_rangos_y_formatos() {
        let m = wyndor();
        let s = solve(&m, &Params::default());
        assert_eq!(s.status, Status::Optimal);
        assert!((s.objective.unwrap() - 36.0).abs() < 1e-7);
        assert!((s.x[0] - 2.0).abs() < 1e-7 && (s.x[1] - 6.0).abs() < 1e-7);
        let d = s.row_dual.unwrap();
        assert!((d[1] - 1.5).abs() < 1e-7 && (d[2] - 1.0).abs() < 1e-7, "{d:?}");
        assert!(s.solver.starts_with("HiGHS"));
        let r = ranging(&m, &Params::default()).unwrap();
        // c(Puertas) puede ir de 0 a 7.5 (Hillier y Lieberman, sec. 7.2)
        assert!((r.cost[0].low - 0.0).abs() < 1e-7 && (r.cost[0].high - 7.5).abs() < 1e-7, "{:?}", r.cost[0]);
        for f in [Format::Mps, Format::Lp] {
            let t = write_text(&m, f).unwrap();
            let m2 = read_text(&t, f).unwrap();
            assert_eq!(m2.cols.len(), 2);
            assert!((solve(&m2, &Params::default()).objective.unwrap() - 36.0).abs() < 1e-7, "{t}");
        }
    }

    #[test]
    fn entero_iis_relajacion_y_multiobjetivo() {
        let mut m = Model { maximize: true, ..Default::default() };
        let x = m.add_col("x", 5.0, 0.0, f64::INFINITY, Kind::Integer);
        let y = m.add_col("y", 8.0, 0.0, f64::INFINITY, Kind::Integer);
        m.add_row("c1", vec![(x, 1.0), (y, 1.0)], f64::NEG_INFINITY, 6.0);
        m.add_row("c2", vec![(x, 5.0), (y, 9.0)], f64::NEG_INFINITY, 45.0);
        let s = solve(&m, &Params { mip_gap: Some(0.0), ..Default::default() });
        assert_eq!(s.status, Status::Optimal);
        assert!((s.objective.unwrap() - 40.0).abs() < 1e-7);
        assert!(s.is_mip && s.gap.unwrap() <= 1e-9);

        // x ≥ 3 y x ≤ 2: el IIS son esas dos filas; la relajación viola 1 unidad
        let mut inf = Model::default();
        let x = inf.add_col("x", 1.0, 0.0, f64::INFINITY, Kind::Continuous);
        inf.add_row("al menos 3", vec![(x, 1.0)], 3.0, f64::INFINITY);
        inf.add_row("a lo más 2", vec![(x, 1.0)], f64::NEG_INFINITY, 2.0);
        inf.add_row("libre", vec![(x, 1.0)], f64::NEG_INFINITY, 100.0);
        assert_eq!(solve(&inf, &Params::default()).status, Status::Infeasible);
        let i = iis(&inf).unwrap();
        let rows: Vec<usize> = i.rows.iter().map(|r| r.0).collect();
        assert!(rows.contains(&0) && rows.contains(&1) && !rows.contains(&2), "{i:?}");
        let r = feasibility_relaxation(&inf, 1.0, -1.0).unwrap();
        assert!((r.total - 1.0).abs() < 1e-7, "{r:?}");

        // dos objetivos: primero maximizar x + y, luego minimizar x
        let mut mo = Model::default();
        let x = mo.add_col("x", 0.0, 0.0, 10.0, Kind::Continuous);
        let y = mo.add_col("y", 0.0, 0.0, 10.0, Kind::Continuous);
        mo.add_row("cap", vec![(x, 1.0), (y, 1.0)], f64::NEG_INFINITY, 12.0);
        mo.objectives = vec![
            Objective { name: "total".into(), coefs: [("x".to_string(), 1.0), ("y".to_string(), 1.0)].into(), maximize: true, priority: 2, ..Default::default() },
            Objective { name: "x".into(), coefs: [("x".to_string(), 1.0)].into(), maximize: false, priority: 1, ..Default::default() },
        ];
        let s = solve(&mo, &Params::default());
        assert_eq!(s.objective_values[0].1.round(), 12.0, "{:?}", s.objective_values);
        // entre todos los que logran 12, el de menor x: x = 2, y = 10
        assert!((s.x[0] - 2.0).abs() < 1e-6 && (s.x[1] - 10.0).abs() < 1e-6, "{:?}", s.x);
    }
}
