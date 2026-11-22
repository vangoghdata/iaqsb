//! LP-ILP con el motor de [`iaqsb_core::solver`] (HiGHS o el motor externo que se
//! pida): para modelos de cualquier tamaño, con sensibilidad por rangos,
//! estadísticas del motor y diagnóstico de infactibilidad.

use crate::model::{fmt_num, Bound, Direction, LpModel, Objective, VarType};
use crate::report::{BasisStatus, ConstraintResult, LpReport, Status, VariableResult};
use crate::simplex::clean;
use iaqsb_core::solver::{self, BasisStatus as Bs, IisBound, Kind, Model, Params, Solution};
use schemars::JsonSchema;
use serde::Serialize;

/// Datos del motor y del diagnóstico que se agregan al informe.
#[derive(Debug, Clone, Default, Serialize, JsonSchema)]
pub struct Extra {
    /// Motor, tiempo, cota, brecha, nodos, tamaño del modelo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<serde_json::Value>,
    /// Por qué no es factible (IIS y relajación de factibilidad).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnosis: Option<Diagnosis>,
    /// Valor de cada objetivo (modelos multiobjetivo).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub objective_values: Vec<(String, f64)>,
    /// Registro del motor (si se pidió).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log: Option<String>,
    /// Avisos.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

/// Un elemento del conflicto de un modelo no factible.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Conflict {
    /// "constraint" o "bound".
    pub kind: String,
    pub name: String,
    /// La restricción o la cota escrita («C1: 2X1 + X2 <= 10», «X1 >= 3»).
    pub text: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Relaxed {
    pub kind: String,
    pub name: String,
    /// Cuánto hay que mover el lado derecho o la cota para que sea factible.
    pub change: f64,
    pub text: String,
}

/// Diagnóstico de un modelo no factible.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Diagnosis {
    /// Subsistema irreducible infactible: estas restricciones y cotas no se pueden
    /// cumplir a la vez, y basta quitar o relajar una cualquiera para romper el conflicto.
    pub conflict: Vec<Conflict>,
    /// Relajación de factibilidad: el menor cambio total de los lados derechos que hace factible el modelo.
    pub relaxation: Vec<Relaxed>,
    pub relaxation_total: Option<f64>,
    pub summary: String,
}

/// El modelo de LP-ILP como modelo del motor.
pub fn to_engine(model: &LpModel, relax_integers: bool) -> Model {
    let mut m = Model { name: model.name.clone(), maximize: model.objective == Objective::Max, ..Default::default() };
    for (j, v) in model.variables.iter().enumerate() {
        let (lb, ub) = model.effective_bounds(j);
        let kind = if relax_integers {
            Kind::Continuous
        } else {
            match v.var_type {
                VarType::Integer => Kind::Integer,
                VarType::Binary => Kind::Binary,
                VarType::SemiContinuous => Kind::SemiContinuous,
                VarType::SemiInteger => Kind::SemiInteger,
                _ => Kind::Continuous,
            }
        };
        m.add_col(v.name.clone(), v.cost, lb, ub, kind);
    }
    for c in &model.constraints {
        let coefs: Vec<(usize, f64)> = c.coefs.iter().enumerate().filter(|(_, a)| **a != 0.0).map(|(j, a)| (j, *a)).collect();
        let (lo, hi) = match c.direction {
            Direction::Le => (f64::NEG_INFINITY, c.rhs),
            Direction::Ge => (c.rhs, f64::INFINITY),
            Direction::Eq => (c.rhs, c.rhs),
        };
        m.add_row(c.name.clone(), coefs, lo, hi);
    }
    m.objectives = model.objectives.clone();
    m
}

fn row_text(m: &Model, i: usize, side: Option<IisBound>) -> String {
    let r = &m.rows[i];
    let mut lhs = String::new();
    for (k, &(j, a)) in r.coefs.iter().enumerate() {
        if k >= 8 {
            lhs.push_str(" + …");
            break;
        }
        let sign = if a < 0.0 { " - " } else if lhs.is_empty() { "" } else { " + " };
        let mag = a.abs();
        let coef = if (mag - 1.0).abs() < 1e-12 { String::new() } else { fmt_num(mag) };
        lhs.push_str(&format!("{sign}{coef}{}", m.cols[j].name));
    }
    if lhs.is_empty() {
        lhs = "0".into();
    }
    let rel = if r.lo == r.hi {
        format!("= {}", fmt_num(r.lo))
    } else {
        match side {
            Some(IisBound::Lower) => format!(">= {}", fmt_num(r.lo)),
            Some(IisBound::Upper) => format!("<= {}", fmt_num(r.hi)),
            _ if r.lo.is_finite() && r.hi.is_finite() => format!("entre {} y {}", fmt_num(r.lo), fmt_num(r.hi)),
            _ if r.lo.is_finite() => format!(">= {}", fmt_num(r.lo)),
            _ => format!("<= {}", fmt_num(r.hi)),
        }
    };
    format!("{}: {lhs} {rel}", r.name)
}

fn col_text(m: &Model, j: usize, side: IisBound) -> String {
    let c = &m.cols[j];
    match side {
        IisBound::Lower => format!("{} >= {}", c.name, fmt_num(c.lb)),
        IisBound::Upper => format!("{} <= {}", c.name, fmt_num(c.ub)),
        _ => format!("{} entre {} y {}", c.name, fmt_num(c.lb), fmt_num(c.ub)),
    }
}

/// IIS y relajación de factibilidad de un modelo no factible.
pub fn diagnose(m: &Model) -> Option<Diagnosis> {
    let iis = solver::iis(m);
    let conflict: Vec<Conflict> = iis
        .as_ref()
        .map(|i| {
            let mut v: Vec<Conflict> = i.rows.iter().map(|&(r, b)| Conflict { kind: "constraint".into(), name: m.rows[r].name.clone(), text: row_text(m, r, Some(b)) }).collect();
            v.extend(i.cols.iter().filter(|(_, b)| *b != IisBound::Free).map(|&(j, b)| Conflict { kind: "bound".into(), name: m.cols[j].name.clone(), text: col_text(m, j, b) }));
            v
        })
        .unwrap_or_default();
    // relajación: solo los lados derechos (las cotas de las variables se respetan)
    let rel = solver::feasibility_relaxation(m, 1.0, -1.0).or_else(|| solver::feasibility_relaxation(m, 1.0, 1.0));
    let relaxation: Vec<Relaxed> = rel
        .as_ref()
        .map(|r| {
            r.violations
                .iter()
                .map(|v| {
                    let (kind, text) = if v.kind == "row" {
                        let row = &m.rows[v.index];
                        let (bound, new) = if v.side == "lower" { (row.lo, row.lo - v.amount) } else { (row.hi, row.hi + v.amount) };
                        ("constraint", format!("{}: lado derecho de {} a {}", row.name, fmt_num(bound), fmt_num(clean(new))))
                    } else {
                        let c = &m.cols[v.index];
                        let (bound, new) = if v.side == "lower" { (c.lb, c.lb - v.amount) } else { (c.ub, c.ub + v.amount) };
                        ("bound", format!("{}: cota de {} a {}", c.name, fmt_num(bound), fmt_num(clean(new))))
                    };
                    Relaxed { kind: kind.into(), name: v.name.clone(), change: clean(v.amount), text }
                })
                .collect()
        })
        .unwrap_or_default();
    if conflict.is_empty() && relaxation.is_empty() {
        return None;
    }
    let mut summary = String::new();
    if !conflict.is_empty() {
        let names: Vec<&str> = conflict.iter().map(|c| c.name.as_str()).collect();
        summary.push_str(&format!("No se pueden cumplir a la vez: {}.", names.join(", ")));
    }
    if let Some(r) = rel.as_ref().filter(|_| !relaxation.is_empty()) {
        if !summary.is_empty() {
            summary.push(' ');
        }
        summary.push_str(&format!(
            "El cambio mínimo para que sea factible suma {}: {}.",
            fmt_num(clean(r.total)),
            relaxation.iter().map(|x| x.text.clone()).collect::<Vec<_>>().join("; ")
        ));
    }
    Some(Diagnosis { relaxation_total: rel.map(|r| clean(r.total)), conflict, relaxation, summary })
}

fn status_of(s: solver::Status, has: bool) -> Status {
    match s {
        solver::Status::Optimal => Status::Optimal,
        solver::Status::Infeasible => Status::Infeasible,
        solver::Status::Unbounded | solver::Status::InfeasibleOrUnbounded => Status::Unbounded,
        solver::Status::NodeLimit => Status::NodeLimit,
        solver::Status::IterationLimit => Status::IterationLimit,
        solver::Status::TimeLimit if has => Status::Feasible,
        solver::Status::SolutionLimit | solver::Status::Interrupted if has => Status::Feasible,
        solver::Status::TimeLimit => Status::TimeLimit,
        _ => Status::Error,
    }
}

/// Informe a partir del resultado del motor (cualquier tamaño).
pub fn report(m: &Model, sol: &Solution, rg: Option<&solver::Ranging>) -> LpReport {
    let objective = if m.maximize { Objective::Max } else { Objective::Min };
    let status = status_of(sol.status, sol.has_solution);
    let optimal = status == Status::Optimal;
    let lp_dual = optimal && !sol.is_mip && sol.row_dual.is_some();
    let mut variables = Vec::with_capacity(m.cols.len());
    for (j, c) in m.cols.iter().enumerate() {
        let value = if sol.has_solution { clean(sol.x[j]) } else { 0.0 };
        let basic = sol.col_basis.as_ref().map_or(value.abs() > 1e-9 && value > c.lb + 1e-9 && value < c.ub - 1e-9, |b| b[j] == Bs::Basic);
        // costo reducido como en WinQSB: cuánto debe mejorar c(j) para que la variable entre
        let flip = if m.maximize { -1.0 } else { 1.0 };
        let rc = if lp_dual { clean(flip * sol.reduced_cost.as_ref().map_or(0.0, |r| r[j])) } else { 0.0 };
        let (cmin, cmax) = match rg {
            Some(r) if lp_dual => (Some(Bound(clean(r.cost[j].low))), Some(Bound(clean(r.cost[j].high)))),
            _ => (None, None),
        };
        variables.push(VariableResult {
            name: c.name.clone(),
            value,
            cost: c.cost,
            contribution: clean(value * c.cost),
            reduced_cost: rc,
            basis_status: if basic { BasisStatus::Basic } else { BasisStatus::AtBound },
            cost_min: cmin,
            cost_max: cmax,
        });
    }
    let mut constraints = Vec::with_capacity(m.rows.len());
    for (i, r) in m.rows.iter().enumerate() {
        let lhs = if sol.has_solution { clean(sol.row_activity[i]) } else { 0.0 };
        let (direction, rhs) = if r.lo == r.hi {
            (Direction::Eq, r.lo)
        } else if r.hi.is_finite() && (!r.lo.is_finite() || (lhs - r.hi).abs() <= (lhs - r.lo).abs()) {
            (Direction::Le, r.hi)
        } else if r.lo.is_finite() {
            (Direction::Ge, r.lo)
        } else {
            (Direction::Le, f64::INFINITY)
        };
        let slack = clean(match direction {
            Direction::Le => rhs - lhs,
            Direction::Ge => lhs - rhs,
            Direction::Eq => 0.0,
        });
        // fila holgada (básica): el lado derecho puede moverse hasta el valor del lado izquierdo
        let row_basic = sol.row_basis.as_ref().map_or(false, |b| b[i] == Bs::Basic) && slack.abs() > 1e-9;
        let (rmin, rmax) = match rg {
            Some(_) if lp_dual && row_basic => match direction {
                Direction::Le => (Some(Bound(lhs)), Some(Bound::INF)),
                Direction::Ge => (Some(Bound::NEG_INF), Some(Bound(lhs))),
                Direction::Eq => (Some(Bound(lhs)), Some(Bound(lhs))),
            },
            Some(g) if lp_dual => (Some(Bound(clean(g.rhs[i].low))), Some(Bound(clean(g.rhs[i].high)))),
            _ => (None, None),
        };
        constraints.push(ConstraintResult {
            name: r.name.clone(),
            lhs,
            direction,
            rhs,
            slack_or_surplus: if slack.is_finite() { slack } else { 0.0 },
            shadow_price: lp_dual.then(|| clean(sol.row_dual.as_ref().map_or(0.0, |d| d[i]))),
            rhs_min: rmin,
            rhs_max: rmax,
        });
    }
    // óptimos alternativos: una variable o una holgura no básica con costo reducido 0
    let alternate = lp_dual
        && (sol.col_basis.as_ref().zip(sol.reduced_cost.as_ref()).map_or(false, |(b, rc)| b.iter().zip(rc).any(|(s, d)| *s != Bs::Basic && d.abs() <= 1e-9))
            || sol.row_basis.as_ref().zip(sol.row_dual.as_ref()).map_or(false, |(b, d)| b.iter().zip(d).zip(&m.rows).any(|((s, y), r)| *s != Bs::Basic && y.abs() <= 1e-9 && r.lo != r.hi)));
    let objective_value = sol.has_solution.then(|| clean(sol.objective.unwrap_or(0.0)));
    let message = match status {
        Status::Optimal if sol.is_mip => format!("Solución entera óptima: {}.", fmt_num(objective_value.unwrap_or(0.0))),
        Status::Optimal => {
            let mut s = format!("Solución óptima encontrada. Objetivo = {}.", fmt_num(objective_value.unwrap_or(0.0)));
            if alternate {
                s.push_str(" Existen soluciones óptimas alternativas.");
            }
            s
        }
        Status::Feasible => format!(
            "Mejor solución encontrada: {} (cota {}, brecha {}); {}.",
            fmt_num(objective_value.unwrap_or(0.0)),
            sol.bound.map(fmt_num).unwrap_or_else(|| "–".into()),
            sol.gap.map(|g| format!("{} %", fmt_num(clean(100.0 * g)))).unwrap_or_else(|| "–".into()),
            if sol.message.is_empty() { "el motor se detuvo por un límite".to_string() } else { sol.message.clone() }
        ),
        Status::Infeasible => "El problema no es factible: ninguna solución cumple todas las restricciones.".into(),
        Status::Unbounded => "El problema no está acotado (o no es factible): el objetivo puede mejorar sin límite.".into(),
        Status::TimeLimit => "Se acabó el tiempo sin encontrar una solución factible.".into(),
        Status::NodeLimit => "Se alcanzó el máximo de nodos.".into(),
        Status::IterationLimit => "Se alcanzó el máximo de iteraciones.".into(),
        Status::Error => format!("El motor no pudo resolver el modelo: {}", sol.message),
    };
    LpReport {
        problem: m.name.clone(),
        objective,
        status,
        method: if sol.is_mip { "branch_and_cut".into() } else { "engine".into() },
        objective_value,
        iterations: sol.iterations.max(0) as usize,
        variables,
        constraints,
        alternate_optima: alternate,
        sensitivity: lp_dual,
        unbounded_variable: None,
        message,
        simplex_trace: None,
        branch_and_bound: None,
        extra: Extra { engine: Some(sol.summary()), objective_values: sol.objective_values.clone(), log: sol.log.clone(), ..Default::default() },
    }
}

/// Resuelve con el motor y arma el informe (con diagnóstico si no es factible).
pub fn solve_engine(m: &Model, p: &Params, diagnose_infeasible: bool) -> LpReport {
    let sol = solver::solve(m, p);
    let rg = if !sol.is_mip && sol.status == solver::Status::Optimal && m.objectives.is_empty() && !external(p) { solver::ranging(m, p) } else { None };
    let mut rep = report(m, &sol, rg.as_ref());
    if diagnose_infeasible && matches!(sol.status, solver::Status::Infeasible | solver::Status::InfeasibleOrUnbounded) {
        rep.extra.diagnosis = diagnose(m);
    }
    rep
}

/// ¿Se pidió un motor externo?
pub fn external(p: &Params) -> bool {
    p.solver.as_deref().map(|s| !matches!(s.trim().to_lowercase().as_str(), "" | "auto" | "highs")).unwrap_or(false)
}

/// Celdas de la matriz densa a partir de las cuales un modelo se deja como texto
/// (MPS/LP) en lugar de pasarlo a la hoja de LP-ILP.
pub const DENSE_LIMIT: usize = 4_000_000;

/// Un modelo del motor como modelo de LP-ILP (hoja densa, compatible con WinQSB).
/// Las filas con dos lados se parten en dos y las filas libres se omiten.
pub fn from_engine(m: &Model) -> LpModel {
    use crate::model::{Constraint, Variable};
    let n = m.cols.len();
    let variables = m
        .cols
        .iter()
        .map(|c| {
            let var_type = match c.kind {
                Kind::Integer => VarType::Integer,
                Kind::Binary => VarType::Binary,
                Kind::SemiContinuous => VarType::SemiContinuous,
                Kind::SemiInteger => VarType::SemiInteger,
                Kind::Continuous if c.lb == f64::NEG_INFINITY && c.ub == f64::INFINITY => VarType::Unrestricted,
                Kind::Continuous => VarType::Continuous,
            };
            Variable { name: c.name.clone(), cost: c.cost, lower: Bound(c.lb), upper: Bound(c.ub), var_type, priority: None }
        })
        .collect();
    let mut constraints = vec![];
    for r in &m.rows {
        let mut coefs = vec![0.0; n];
        for &(j, a) in &r.coefs {
            coefs[j] += a;
        }
        let mk = |name: String, direction: Direction, rhs: f64, coefs: Vec<f64>| Constraint { terms: None, name, coefs, direction, rhs };
        if r.lo == r.hi {
            constraints.push(mk(r.name.clone(), Direction::Eq, r.lo, coefs));
        } else if r.lo.is_finite() && r.hi.is_finite() {
            constraints.push(mk(format!("{}_min", r.name), Direction::Ge, r.lo, coefs.clone()));
            constraints.push(mk(format!("{}_max", r.name), Direction::Le, r.hi, coefs));
        } else if r.hi.is_finite() {
            constraints.push(mk(r.name.clone(), Direction::Le, r.hi, coefs));
        } else if r.lo.is_finite() {
            constraints.push(mk(r.name.clone(), Direction::Ge, r.lo, coefs));
        }
    }
    LpModel {
        name: m.name.clone(),
        objective: if m.maximize { Objective::Max } else { Objective::Min },
        variables,
        constraints,
        objectives: m.objectives.clone(),
        start: None,
    }
}
