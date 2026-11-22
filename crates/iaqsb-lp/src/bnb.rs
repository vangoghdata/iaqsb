//! Branch and bound para variables enteras y binarias, con la regla de
//! ramificación de WinQSB: se ramifica sobre la variable entera fraccionaria de
//! mayor prioridad (a igual prioridad, la primera), primero la rama x <= floor.

use crate::model::{fmt_num, fmt_short, LpModel, Objective, VarType};
use crate::report::{solve_lp, LpReport, Status};
use crate::SolveOptions;
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct BnbNode {
    pub node: usize,
    pub depth: usize,
    /// Ramas aplicadas desde la raíz, como "X1 <= 2".
    pub branches: Vec<String>,
    pub status: Status,
    pub objective_value: Option<f64>,
    pub values: Vec<f64>,
    /// Qué se hizo con el nodo.
    pub action: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct BnbInfo {
    /// Óptimo de la relajación lineal (raíz).
    pub relaxed_objective: Option<f64>,
    pub nodes_explored: usize,
    /// Número de soluciones enteras mejores encontradas.
    pub incumbents_found: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<Vec<BnbNode>>,
}

struct Pending {
    bounds: Vec<(f64, f64)>,
    branches: Vec<String>,
    depth: usize,
}

fn better(obj: Objective, a: f64, b: f64) -> bool {
    match obj {
        Objective::Max => a > b + 1e-9 * (1.0 + b.abs()),
        Objective::Min => a < b - 1e-9 * (1.0 + b.abs()),
    }
}

pub fn branch_and_bound(model: &LpModel, opts: &SolveOptions) -> LpReport {
    let n = model.variables.len();
    let root_bounds: Vec<(f64, f64)> = (0..n).map(|j| model.effective_bounds(j)).collect();
    let tol = opts.integer_tolerance.max(1e-9);
    let priority = |j: usize| -> i32 {
        model.variables[j]
            .priority
            .unwrap_or(match model.variables[j].var_type {
                VarType::Binary => 2,
                _ => 1,
            })
    };

    let mut stack = vec![Pending {
        bounds: root_bounds,
        branches: vec![],
        depth: 0,
    }];
    let mut incumbent: Option<(LpReport, f64)> = None;
    let mut relaxed: Option<f64> = None;
    let mut root_status = None;
    let mut explored = 0usize;
    let mut incumbents = 0usize;
    let mut trace = Vec::new();
    let mut total_iters = 0usize;
    let mut hit_limit = false;

    while let Some(node) = stack.pop() {
        if explored >= opts.max_nodes {
            hit_limit = true;
            break;
        }
        explored += 1;
        let (rep, _) = solve_lp(model, &node.bounds, opts, false);
        total_iters += rep.iterations;
        if explored == 1 {
            relaxed = rep.objective_value;
            root_status = Some(rep.status);
        }
        let values: Vec<f64> = rep.variables.iter().map(|v| v.value).collect();
        let mut record = |action: String, rep: &LpReport| {
            if opts.bnb_trace {
                trace.push(BnbNode {
                    node: explored,
                    depth: node.depth,
                    branches: node.branches.clone(),
                    status: rep.status,
                    objective_value: rep.objective_value,
                    values: values.clone(),
                    action,
                });
            }
        };
        if rep.status != Status::Optimal {
            let why = match rep.status {
                Status::Unbounded => "relajación no acotada",
                Status::Infeasible => "no factible",
                _ => "sin solución",
            };
            record(format!("descartado: {why}"), &rep);
            if explored == 1 && rep.status == Status::Unbounded {
                // la relajación de la raíz no está acotada: se informa tal cual
                let mut r = rep;
                r.method = "branch_and_bound".into();
                r.sensitivity = false;
                return r;
            }
            continue;
        }
        let z = rep.objective_value.unwrap();
        // poda por cota (con la calidad de solución pedida)
        if let Some((_, best)) = &incumbent {
            let gap = opts.solution_quality_pct.abs() / 100.0 * best.abs();
            let promising = match model.objective {
                Objective::Max => z > best + gap + 1e-9 * (1.0 + best.abs()),
                Objective::Min => z < best - gap - 1e-9 * (1.0 + best.abs()),
            };
            if !promising {
                record(
                    format!("podado: no mejora la mejor solución ({})", fmt_short(*best)),
                    &rep,
                );
                continue;
            }
        }
        // variable para ramificar
        let mut pick: Option<usize> = None;
        for j in 0..n {
            if !model.is_integer(j) {
                continue;
            }
            let x = values[j];
            if (x - (x + 0.5).floor()).abs() > tol {
                pick = match pick {
                    None => Some(j),
                    Some(k) if priority(j) > priority(k) => Some(j),
                    p => p,
                };
            }
        }
        match pick {
            None => {
                let improves = incumbent
                    .as_ref()
                    .map_or(true, |(_, best)| better(model.objective, z, *best));
                if improves {
                    incumbents += 1;
                    record(
                        format!("nueva mejor solución entera: {}", fmt_short(z)),
                        &rep,
                    );
                    incumbent = Some((rep, z));
                } else {
                    record("solución entera, no mejora".into(), &rep);
                }
            }
            Some(j) => {
                let x = values[j];
                let fl = x.floor();
                let name = &model.variables[j].name;
                record(format!("ramificar sobre {name} = {}", fmt_short(x)), &rep);
                let (l, u) = node.bounds[j];
                // se apila primero la rama >= para explorar antes la rama <=
                let mut up = node.bounds.clone();
                up[j] = (l.max(fl + 1.0), u);
                let mut br_up = node.branches.clone();
                br_up.push(format!("{name} >= {}", fmt_num(fl + 1.0)));
                let mut down = node.bounds.clone();
                down[j] = (l, u.min(fl));
                let mut br_down = node.branches.clone();
                br_down.push(format!("{name} <= {}", fmt_num(fl)));
                if up[j].0 <= up[j].1 {
                    stack.push(Pending {
                        bounds: up,
                        branches: br_up,
                        depth: node.depth + 1,
                    });
                }
                if down[j].0 <= down[j].1 {
                    stack.push(Pending {
                        bounds: down,
                        branches: br_down,
                        depth: node.depth + 1,
                    });
                }
            }
        }
    }

    let info = BnbInfo {
        relaxed_objective: relaxed,
        nodes_explored: explored,
        incumbents_found: incumbents,
        trace: if opts.bnb_trace { Some(trace) } else { None },
    };
    match incumbent {
        Some((mut rep, z)) => {
            // redondear las variables enteras dentro de la tolerancia
            for (j, v) in rep.variables.iter_mut().enumerate() {
                if model.is_integer(j) {
                    let r = (v.value + 0.5).floor();
                    if (v.value - r).abs() <= 1e-6 {
                        v.value = r;
                    }
                }
                v.cost_min = None;
                v.cost_max = None;
            }
            for c in rep.constraints.iter_mut() {
                c.shadow_price = None;
                c.rhs_min = None;
                c.rhs_max = None;
            }
            rep.method = "branch_and_bound".into();
            rep.sensitivity = false;
            rep.iterations = total_iters;
            rep.status = if hit_limit {
                Status::NodeLimit
            } else {
                Status::Optimal
            };
            rep.message = if hit_limit {
                format!(
                    "Se alcanzó el máximo de {} nodos. Mejor solución entera encontrada: {}.",
                    opts.max_nodes,
                    fmt_num(z)
                )
            } else {
                format!(
                    "Solución entera óptima: {}. Branch and bound exploró {} nodos.",
                    fmt_num(z),
                    explored
                )
            };
            rep.alternate_optima = false;
            rep.branch_and_bound = Some(info);
            rep
        }
        None => {
            let status = if hit_limit {
                Status::NodeLimit
            } else if root_status == Some(Status::Unbounded) {
                Status::Unbounded
            } else {
                Status::Infeasible
            };
            LpReport {
                problem: model.name.clone(),
                objective: model.objective,
                status,
                method: "branch_and_bound".into(),
                objective_value: None,
                iterations: total_iters,
                variables: vec![],
                constraints: vec![],
                alternate_optima: false,
                sensitivity: false,
                unbounded_variable: None,
                message: if hit_limit {
                    "Se alcanzó el máximo de nodos sin encontrar una solución entera.".into()
                } else {
                    "El problema entero no es factible.".into()
                },
                simplex_trace: None,
                branch_and_bound: Some(info),
                extra: Default::default(),
            }
        }
    }
}
