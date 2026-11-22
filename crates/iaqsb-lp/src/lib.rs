//! IAQSB · módulo LP-ILP: programación lineal y entera.
//!
//! Reimplementación del módulo "Linear and Integer Programming" de WinQSB 2.0:
//! símplex con gran M, branch and bound, análisis de sensibilidad, método gráfico
//! y paso a paso de las iteraciones.

pub mod algebra;
pub mod bnb;
pub mod engine;
pub mod graphic;
pub mod model;
pub mod report;
pub mod simplex;

pub use model::*;
pub use report::*;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LpError {
    #[error("modelo no válido: {0}")]
    Invalid(String),
}

/// Opciones de resolución. Las de WinQSB (paso a paso, branch and bound
/// didáctico) y las del motor (tiempo, brecha, hilos, motor externo…).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct SolveOptions {
    /// Guardar la tabla del símplex de cada iteración ("Solve and Display Steps").
    /// Solo en modelos pequeños (el símplex de tablas es didáctico).
    pub trace: bool,
    /// Guardar los nodos de branch and bound (didáctico, modelos pequeños).
    pub bnb_trace: bool,
    /// Tolerancia para considerar entero un valor en el branch and bound didáctico (WinQSB: 0.01).
    pub integer_tolerance: f64,
    /// Calidad de solución (WinQSB): % aceptado respecto al óptimo. Equivale a mip_gap.
    pub solution_quality_pct: f64,
    /// Máximo de nodos del branch and bound didáctico.
    pub max_nodes: usize,
    /// Máximo de iteraciones del símplex didáctico.
    pub max_iterations: usize,
    /// Ignorar las variables enteras y resolver la relajación lineal.
    pub relax_integers: bool,
    /// Si no es factible, explicar por qué (IIS y relajación de factibilidad).
    pub diagnose: bool,
    /// Usar siempre el motor (HiGHS o el externo), sin el símplex de tablas aunque el modelo sea pequeño.
    pub engine_only: bool,
    /// Parámetros del motor: solver, time_limit, mip_gap, threads, presolve, method, seed…
    #[serde(flatten)]
    pub engine: iaqsb_core::solver::Params,
}

impl Default for SolveOptions {
    fn default() -> Self {
        SolveOptions {
            trace: false,
            bnb_trace: false,
            integer_tolerance: 0.01,
            solution_quality_pct: 0.0,
            max_nodes: 100_000,
            max_iterations: 100_000,
            relax_integers: false,
            diagnose: true,
            engine_only: false,
            engine: Default::default(),
        }
    }
}

impl SolveOptions {
    /// Parámetros del motor, con la calidad de solución de WinQSB como brecha.
    pub fn engine_params(&self) -> iaqsb_core::solver::Params {
        let mut p = self.engine.clone();
        if p.mip_gap.is_none() && self.solution_quality_pct > 0.0 {
            p.mip_gap = Some(self.solution_quality_pct / 100.0);
        }
        p
    }
}

/// ¿Se resuelve con el método didáctico (tablas del símplex o árbol de branch
/// and bound de WinQSB)? Solo en modelos pequeños y si nada pide el motor.
fn didactic(model: &LpModel, opts: &SolveOptions) -> bool {
    let n = model.variables.len();
    let m = model.constraints.len();
    let p = &opts.engine;
    let engine_asked = opts.engine_only || engine::external(p) || p.time_limit.is_some() || p.mip_gap.is_some() || p.threads.is_some() || p.method.is_some() || p.log;
    let special = !model.objectives.is_empty() || model.variables.iter().any(|v| matches!(v.var_type, VarType::SemiContinuous | VarType::SemiInteger));
    if engine_asked || special {
        return false;
    }
    if model.has_integers() && !opts.relax_integers {
        opts.bnb_trace && n <= 40 && m <= 40
    } else {
        n * (n + m) <= 40_000
    }
}

/// Resuelve un modelo LP o ILP y devuelve el informe completo.
///
/// Los modelos pequeños (los de clase) se resuelven con el símplex de tablas o
/// el branch and bound de WinQSB, que muestran cada paso; el resultado se
/// comprueba con el motor. Los demás van directo al motor (HiGHS o el que se
/// pida), sin límite de tamaño.
pub fn solve(model: &LpModel, opts: &SolveOptions) -> Result<LpReport, LpError> {
    let mut model = model.clone();
    model.normalize()?;
    model.validate()?;
    let em = engine::to_engine(&model, opts.relax_integers);
    let params = opts.engine_params();
    if !didactic(&model, opts) {
        let mut rep = engine::solve_engine(&em, &params, opts.diagnose);
        if opts.trace {
            rep.extra.notes.push("Las tablas del símplex se muestran solo en modelos pequeños resueltos sin parámetros del motor.".into());
        }
        return Ok(rep);
    }
    let mut rep = if model.has_integers() && !opts.relax_integers {
        bnb::branch_and_bound(&model, opts)
    } else {
        let bounds: Vec<(f64, f64)> = (0..model.variables.len()).map(|j| model.effective_bounds(j)).collect();
        report::solve_lp(&model, &bounds, opts, opts.trace).0
    };
    // comprobación con el motor
    let check = iaqsb_core::solver::solve(&em, &params);
    if let (Some(a), Some(b)) = (rep.objective_value, check.objective) {
        if rep.status == Status::Optimal && check.status == iaqsb_core::solver::Status::Optimal && (a - b).abs() > 1e-6 * (1.0 + b.abs()) {
            rep.extra.notes.push(format!("El motor da otro óptimo ({}); se muestra el del motor.", model::fmt_num(b)));
            let mut e = engine::solve_engine(&em, &params, opts.diagnose);
            e.simplex_trace = rep.simplex_trace.take();
            e.branch_and_bound = rep.branch_and_bound.take();
            e.extra.notes = rep.extra.notes.clone();
            return Ok(e);
        }
    }
    rep.extra.engine = Some(check.summary());
    if opts.diagnose && rep.status == Status::Infeasible {
        rep.extra.diagnosis = engine::diagnose(&em);
    }
    Ok(rep)
}
