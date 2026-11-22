//! Informe de resultados: resumen de la solución, de las restricciones y análisis
//! de sensibilidad (el "Combined Report" de WinQSB).

use crate::bnb::BnbInfo;
use crate::model::{fmt_num, Bound, Direction, LpModel, Objective};
use crate::simplex::{clean, ColKind, SimplexStatus, Tableau, TableauSnapshot};
use crate::SolveOptions;
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Optimal,
    Infeasible,
    Unbounded,
    IterationLimit,
    NodeLimit,
    /// Hay una solución factible, pero el motor se detuvo antes de probar que es óptima (tiempo, brecha, nodos).
    Feasible,
    /// Se acabó el tiempo sin una solución factible.
    TimeLimit,
    /// Error del motor.
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BasisStatus {
    Basic,
    AtBound,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct VariableResult {
    pub name: String,
    pub value: f64,
    /// Coeficiente c(j) en la función objetivo.
    pub cost: f64,
    /// value * cost.
    pub contribution: f64,
    /// Cuánto debe mejorar c(j) para que la variable entre en la base.
    pub reduced_cost: f64,
    pub basis_status: BasisStatus,
    /// Rango de c(j) en el que la solución sigue siendo óptima.
    pub cost_min: Option<Bound>,
    pub cost_max: Option<Bound>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ConstraintResult {
    pub name: String,
    /// Lado izquierdo evaluado en la solución.
    pub lhs: f64,
    pub direction: Direction,
    pub rhs: f64,
    /// Holgura (<=) o exceso (>=), siempre >= 0 si la solución es factible.
    pub slack_or_surplus: f64,
    /// Precio sombra: cambio del objetivo por unidad de aumento del lado derecho.
    pub shadow_price: Option<f64>,
    /// Rango del lado derecho en el que el precio sombra sigue siendo válido.
    pub rhs_min: Option<Bound>,
    pub rhs_max: Option<Bound>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct LpReport {
    pub problem: String,
    pub objective: Objective,
    pub status: Status,
    /// Método usado: "simplex" o "branch_and_bound".
    pub method: String,
    pub objective_value: Option<f64>,
    pub iterations: usize,
    pub variables: Vec<VariableResult>,
    pub constraints: Vec<ConstraintResult>,
    /// Hay soluciones óptimas alternativas.
    pub alternate_optima: bool,
    /// Si es false, los campos de sensibilidad vienen vacíos (p. ej. en problemas enteros).
    pub sensitivity: bool,
    /// Variable que hace el problema no acotado.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unbounded_variable: Option<String>,
    /// Mensaje legible para el usuario.
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simplex_trace: Option<Vec<TableauSnapshot>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_and_bound: Option<BnbInfo>,
    /// Motor, diagnóstico de infactibilidad, multiobjetivo y registro.
    #[serde(flatten)]
    pub extra: crate::engine::Extra,
}

/// Resuelve la relajación lineal con las cotas dadas. Devuelve también la tabla final.
pub fn solve_lp(
    model: &LpModel,
    bounds: &[(f64, f64)],
    opts: &SolveOptions,
    trace: bool,
) -> (LpReport, Tableau) {
    let mut tab = Tableau::build(model, bounds);
    let mut snaps = Vec::new();
    let (st, iters) = tab.solve(
        if trace { Some(&mut snaps) } else { None },
        opts.max_iterations,
    );
    let status = match st {
        SimplexStatus::Optimal => Status::Optimal,
        SimplexStatus::Infeasible => Status::Infeasible,
        SimplexStatus::Unbounded => Status::Unbounded,
        SimplexStatus::IterationLimit => Status::IterationLimit,
    };
    let values = tab.var_values();
    let n = model.variables.len();

    let (dm, dc) = tab.reduced_costs();
    let optimal = status == Status::Optimal;

    // --- variables ---
    let mut variables = Vec::with_capacity(n);
    for j in 0..n {
        let v = &model.variables[j];
        let vm = &tab.vars[j];
        let value = values[j];
        let basic_col = vm.cols.iter().any(|&(c, _)| tab.basis.contains(&c));
        let ub_slack_basic = vm
            .ub_row
            .map(|r| tab.basis.contains(&tab.rows[r].ident))
            .unwrap_or(true);
        let basis_status = if basic_col && ub_slack_basic {
            BasisStatus::Basic
        } else {
            BasisStatus::AtBound
        };
        let (mut rc, mut cmin, mut cmax) = (0.0, None, None);
        if optimal {
            rc = reduced_cost(&tab, j, &dc);
            let (lo, hi) = cost_range(&tab, j, &dm, &dc);
            cmin = Some(Bound(clean(v.cost + lo)));
            cmax = Some(Bound(clean(v.cost + hi)));
        }
        variables.push(VariableResult {
            name: v.name.clone(),
            value,
            cost: v.cost,
            contribution: clean(value * v.cost),
            reduced_cost: clean(rc),
            basis_status,
            cost_min: cmin,
            cost_max: cmax,
        });
    }

    // --- restricciones ---
    let mut constraints = Vec::with_capacity(model.constraints.len());
    for (i, c) in model.constraints.iter().enumerate() {
        let lhs = clean(c.coefs.iter().zip(&values).map(|(a, x)| a * x).sum::<f64>());
        let slack = clean(match c.direction {
            Direction::Le => c.rhs - lhs,
            Direction::Ge => lhs - c.rhs,
            Direction::Eq => 0.0,
        });
        let r = tab
            .rows
            .iter()
            .position(|ri| ri.origin == crate::simplex::RowOrigin::Constraint(i))
            .unwrap();
        let (mut sp, mut rmin, mut rmax) = (None, None, None);
        if optimal {
            let y = -dc[tab.rows[r].ident]; // dual del problema interno
            sp = Some(clean(tab.sense * y * tab.rows[r].flip));
            let (lo, hi) = rhs_range(&tab, r);
            rmin = Some(Bound(clean(c.rhs + lo)));
            rmax = Some(Bound(clean(c.rhs + hi)));
        }
        constraints.push(ConstraintResult {
            name: c.name.clone(),
            lhs,
            direction: c.direction,
            rhs: c.rhs,
            slack_or_surplus: slack,
            shadow_price: sp,
            rhs_min: rmin,
            rhs_max: rmax,
        });
    }

    let objective_value = if optimal {
        Some(clean(
            model
                .variables
                .iter()
                .zip(&values)
                .map(|(v, x)| v.cost * x)
                .sum::<f64>(),
        ))
    } else {
        None
    };
    let alternate = optimal && has_alternate(&tab, &dm, &dc);
    let mut unbounded_variable = None;
    if status == Status::Unbounded {
        // la última tabla indica qué columna entraba sin límite
        if let Some(e) = entering_col(&tab, &dm, &dc) {
            if let ColKind::Var { var, .. } = tab.cols[e].kind {
                unbounded_variable = Some(model.variables[var].name.clone());
            } else {
                unbounded_variable = Some(tab.cols[e].name.clone());
            }
        }
    }
    let message = match status {
        Status::Optimal => {
            let mut s = format!(
                "Solución óptima encontrada. Objetivo = {}.",
                fmt_num(objective_value.unwrap_or(0.0))
            );
            if alternate {
                s.push_str(" Existen soluciones óptimas alternativas.");
            }
            s
        }
        Status::Infeasible => {
            "El problema no es factible: ninguna solución cumple todas las restricciones.".into()
        }
        Status::Unbounded => match &unbounded_variable {
            Some(v) => format!(
                "El problema no está acotado: {v} puede crecer sin límite mejorando el objetivo."
            ),
            None => "El problema no está acotado.".into(),
        },
        Status::IterationLimit => "Se alcanzó el máximo de iteraciones del símplex.".into(),
        Status::NodeLimit => "Se alcanzó el máximo de nodos.".into(),
        Status::Feasible | Status::TimeLimit | Status::Error => String::new(),
    };

    let report = LpReport {
        problem: model.name.clone(),
        objective: model.objective,
        status,
        method: "simplex".into(),
        objective_value,
        iterations: iters,
        variables,
        constraints,
        alternate_optima: alternate,
        sensitivity: optimal,
        unbounded_variable,
        message,
        simplex_trace: if trace { Some(snaps) } else { None },
        branch_and_bound: None,
        extra: Default::default(),
    };
    (report, tab)
}

fn entering_col(tab: &Tableau, dm: &[f64], dc: &[f64]) -> Option<usize> {
    (0..tab.ncol())
        .filter(|j| !tab.basis.contains(j))
        .filter(|&j| dm[j] < -tab.eps || (dm[j].abs() <= tab.eps && dc[j] < -tab.eps))
        .min_by(|&a, &b| (dm[a], dc[a]).partial_cmp(&(dm[b], dc[b])).unwrap())
}

/// Coste reducido de una variable original, en unidades de su coeficiente.
fn reduced_cost(tab: &Tableau, j: usize, dc: &[f64]) -> f64 {
    let vm = &tab.vars[j];
    // variable en su cota superior: el coste reducido es el dual de la fila de cota
    if let Some(r) = vm.ub_row {
        let ident = tab.rows[r].ident;
        if !tab.basis.contains(&ident) {
            return dc[ident];
        }
    }
    // la columna no básica con menor coste reducido
    vm.cols
        .iter()
        .filter(|(c, _)| !tab.basis.contains(c))
        .map(|&(c, _)| dc[c])
        .fold(None, |acc: Option<f64>, x| {
            Some(acc.map_or(x, |a| a.min(x)))
        })
        .unwrap_or(0.0)
}

/// Rango de variación del coste de la variable j (como desplazamientos [lo, hi]).
fn cost_range(tab: &Tableau, j: usize, dm: &[f64], dc: &[f64]) -> (f64, f64) {
    let ncol = tab.ncol();
    let mut v = vec![0.0; ncol];
    for &(c, s) in &tab.vars[j].cols {
        v[c] = tab.sense * s;
    }
    let mut lo = f64::NEG_INFINITY;
    let mut hi = f64::INFINITY;
    let tol = tab.eps;
    for l in 0..ncol {
        if tab.basis.contains(&l) || matches!(tab.cols[l].kind, ColKind::Artificial { .. }) {
            continue;
        }
        if dm[l] > tol {
            continue; // tiene coste M positivo: nunca entra
        }
        let mut g = v[l];
        for (r, &bc) in tab.basis.iter().enumerate() {
            if v[bc] != 0.0 {
                g -= v[bc] * tab.a[r][l];
            }
        }
        if g.abs() <= 1e-12 {
            continue;
        }
        let t = -dc[l] / g;
        if g > 0.0 {
            lo = lo.max(t);
        } else {
            hi = hi.min(t);
        }
    }
    (lo.min(0.0), hi.max(0.0))
}

/// Rango de variación del lado derecho de la fila r (desplazamientos [lo, hi]).
fn rhs_range(tab: &Tableau, r: usize) -> (f64, f64) {
    let ident = tab.rows[r].ident;
    let f = tab.rows[r].flip;
    let mut lo = f64::NEG_INFINITY;
    let mut hi = f64::INFINITY;
    for (k, _) in tab.basis.iter().enumerate() {
        let beta = f * tab.a[k][ident];
        if beta.abs() <= 1e-12 {
            continue;
        }
        let t = -tab.b[k] / beta;
        if beta > 0.0 {
            lo = lo.max(t);
        } else {
            hi = hi.min(t);
        }
    }
    (lo.min(0.0), hi.max(0.0))
}

/// Hay óptimos alternativos si una columna no básica tiene C(j)-Z(j) = 0 y puede entrar.
fn has_alternate(tab: &Tableau, dm: &[f64], dc: &[f64]) -> bool {
    let tol = tab.eps;
    (0..tab.ncol()).any(|l| {
        !tab.basis.contains(&l)
            && !matches!(tab.cols[l].kind, ColKind::Artificial { .. })
            && dm[l].abs() <= tol
            && dc[l].abs() <= tol
            && ((0..tab.m()).any(|r| tab.a[r][l] > 1e-9 && tab.b[r] > 1e-9)
                || (0..tab.m()).all(|r| tab.a[r][l] <= 1e-9))
    })
}
