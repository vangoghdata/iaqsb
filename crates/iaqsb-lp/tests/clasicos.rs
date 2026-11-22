use iaqsb_lp::*;

fn var(name: &str, cost: f64) -> Variable {
    Variable {
        name: name.into(),
        cost,
        lower: Bound(0.0),
        upper: Bound::INF,
        var_type: VarType::Continuous,
        priority: None,
    }
}
fn con(name: &str, coefs: &[f64], d: Direction, rhs: f64) -> Constraint {
    Constraint {
        terms: None,
        name: name.into(),
        coefs: coefs.to_vec(),
        direction: d,
        rhs,
    }
}
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6 * (1.0 + b.abs())
}

fn wyndor() -> LpModel {
    LpModel {
        objectives: vec![],
        start: None,
        name: "Wyndor".into(),
        objective: Objective::Max,
        variables: vec![var("X1", 3.0), var("X2", 5.0)],
        constraints: vec![
            con("C1", &[1.0, 0.0], Direction::Le, 4.0),
            con("C2", &[0.0, 2.0], Direction::Le, 12.0),
            con("C3", &[3.0, 2.0], Direction::Le, 18.0),
        ],
    }
}

#[test]
fn wyndor_optimo_y_sensibilidad() {
    let r = solve(&wyndor(), &SolveOptions::default()).unwrap();
    assert_eq!(r.status, Status::Optimal);
    assert!(close(r.objective_value.unwrap(), 36.0));
    assert!(close(r.variables[0].value, 2.0));
    assert!(close(r.variables[1].value, 6.0));
    let sp: Vec<f64> = r
        .constraints
        .iter()
        .map(|c| c.shadow_price.unwrap())
        .collect();
    assert!(
        close(sp[0], 0.0) && close(sp[1], 1.5) && close(sp[2], 1.0),
        "{sp:?}"
    );
    // rangos de costes: c1 en [0, 7.5], c2 en [2, M]
    assert!(close(r.variables[0].cost_min.unwrap().0, 0.0));
    assert!(close(r.variables[0].cost_max.unwrap().0, 7.5));
    assert!(close(r.variables[1].cost_min.unwrap().0, 2.0));
    assert!(r.variables[1].cost_max.unwrap().0.is_infinite());
    // rangos de lado derecho
    let c = &r.constraints;
    assert!(close(c[0].rhs_min.unwrap().0, 2.0) && c[0].rhs_max.unwrap().0.is_infinite());
    assert!(close(c[1].rhs_min.unwrap().0, 6.0) && close(c[1].rhs_max.unwrap().0, 18.0));
    assert!(close(c[2].rhs_min.unwrap().0, 12.0) && close(c[2].rhs_max.unwrap().0, 24.0));
}

#[test]
fn minimizar_con_mayor_o_igual() {
    let m = LpModel {
        objectives: vec![],
        start: None,
        name: "Dieta".into(),
        objective: Objective::Min,
        variables: vec![var("X", 2.0), var("Y", 3.0)],
        constraints: vec![
            con("C1", &[1.0, 1.0], Direction::Ge, 4.0),
            con("C2", &[1.0, 3.0], Direction::Ge, 6.0),
        ],
    };
    let r = solve(
        &m,
        &SolveOptions {
            trace: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(r.status, Status::Optimal);
    assert!(close(r.objective_value.unwrap(), 9.0));
    assert!(close(r.variables[0].value, 3.0) && close(r.variables[1].value, 1.0));
    // precios sombra positivos: subir el requisito encarece
    assert!(close(r.constraints[0].shadow_price.unwrap(), 1.5));
    assert!(close(r.constraints[1].shadow_price.unwrap(), 0.5));
    let t = r.simplex_trace.unwrap();
    assert!(t.len() >= 2);
    assert!(
        t[0].cj_minus_zj.iter().any(|v| v.m != 0.0),
        "la primera tabla debe llevar términos en M"
    );
}

#[test]
fn no_factible() {
    let m = LpModel {
        objectives: vec![],
        start: None,
        name: "NF".into(),
        objective: Objective::Max,
        variables: vec![var("X", 1.0)],
        constraints: vec![
            con("C1", &[1.0], Direction::Le, 1.0),
            con("C2", &[1.0], Direction::Ge, 2.0),
        ],
    };
    assert_eq!(
        solve(&m, &SolveOptions::default()).unwrap().status,
        Status::Infeasible
    );
}

#[test]
fn no_acotado() {
    let m = LpModel {
        objectives: vec![],
        start: None,
        name: "NA".into(),
        objective: Objective::Max,
        variables: vec![var("X", 1.0), var("Y", 1.0)],
        constraints: vec![con("C1", &[1.0, -1.0], Direction::Le, 1.0)],
    };
    let r = solve(&m, &SolveOptions::default()).unwrap();
    assert_eq!(r.status, Status::Unbounded);
    assert!(r.unbounded_variable.is_some());
}

#[test]
fn igualdad_libre_y_cotas() {
    // min X + 2Y, X libre, 1 <= Y <= 3, X + Y = 2, X >= -5 via restricción
    let mut x = var("X", 1.0);
    x.var_type = VarType::Unrestricted;
    let mut y = var("Y", 2.0);
    y.lower = Bound(1.0);
    y.upper = Bound(3.0);
    let m = LpModel {
        objectives: vec![],
        start: None,
        name: "Libre".into(),
        objective: Objective::Min,
        variables: vec![x, y],
        constraints: vec![
            con("C1", &[1.0, 1.0], Direction::Eq, 2.0),
            con("C2", &[1.0, 0.0], Direction::Ge, -5.0),
        ],
    };
    let r = solve(&m, &SolveOptions::default()).unwrap();
    assert_eq!(r.status, Status::Optimal);
    // Y en su mínimo 1, X = 1, objetivo 3
    assert!(close(r.variables[1].value, 1.0), "{:?}", r.variables);
    assert!(close(r.variables[0].value, 1.0));
    assert!(close(r.objective_value.unwrap(), 3.0));
}

#[test]
fn lado_derecho_negativo() {
    // max X + Y; -X - Y >= -4 (equivale a X + Y <= 4)
    let m = LpModel {
        objectives: vec![],
        start: None,
        name: "Neg".into(),
        objective: Objective::Max,
        variables: vec![var("X", 1.0), var("Y", 1.0)],
        constraints: vec![con("C1", &[-1.0, -1.0], Direction::Ge, -4.0)],
    };
    let r = solve(&m, &SolveOptions::default()).unwrap();
    assert!(close(r.objective_value.unwrap(), 4.0));
    assert!(close(r.constraints[0].shadow_price.unwrap(), -1.0));
    assert!(r.alternate_optima);
}

#[test]
fn entero_clasico() {
    let mut a = var("X1", 5.0);
    a.var_type = VarType::Integer;
    let mut b = var("X2", 8.0);
    b.var_type = VarType::Integer;
    let m = LpModel {
        objectives: vec![],
        start: None,
        name: "ILP".into(),
        objective: Objective::Max,
        variables: vec![a, b],
        constraints: vec![
            con("C1", &[1.0, 1.0], Direction::Le, 6.0),
            con("C2", &[5.0, 9.0], Direction::Le, 45.0),
        ],
    };
    let r = solve(
        &m,
        &SolveOptions {
            bnb_trace: true,
            integer_tolerance: 1e-6,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(r.status, Status::Optimal);
    assert!(
        close(r.objective_value.unwrap(), 40.0),
        "{:?}",
        r.objective_value
    );
    assert!(close(r.variables[0].value, 0.0) && close(r.variables[1].value, 5.0));
    let info = r.branch_and_bound.unwrap();
    assert!(close(info.relaxed_objective.unwrap(), 41.25));
    assert!(info.nodes_explored > 1);
}

#[test]
fn mochila_binaria() {
    let items = [(10.0, 5.0), (40.0, 4.0), (30.0, 6.0), (50.0, 3.0)];
    let variables: Vec<Variable> = items
        .iter()
        .enumerate()
        .map(|(i, &(v, _))| Variable {
            var_type: VarType::Binary,
            ..var(&format!("X{}", i + 1), v)
        })
        .collect();
    let m = LpModel {
        objectives: vec![],
        start: None,
        name: "Mochila".into(),
        objective: Objective::Max,
        variables,
        constraints: vec![con("Peso", &items.map(|x| x.1), Direction::Le, 10.0)],
    };
    let r = solve(
        &m,
        &SolveOptions {
            integer_tolerance: 1e-6,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(close(r.objective_value.unwrap(), 90.0));
    let sel: Vec<f64> = r.variables.iter().map(|v| v.value).collect();
    assert_eq!(sel, vec![0.0, 1.0, 0.0, 1.0]);
}

#[test]
fn metodo_grafico() {
    let g = graphic::graphic(&wyndor(), 0, 1).unwrap();
    assert_eq!(g.feasible_region.len(), 5);
    let o = g.optimum.unwrap();
    assert!(close(o[0], 2.0) && close(o[1], 6.0));
    assert!(!g.region_unbounded);
}

/// El motor (HiGHS) y el símplex de tablas dan el mismo óptimo y la misma
/// sensibilidad (precios sombra, costos reducidos y rangos) en problemas
/// pequeños no degenerados, de máximo y de mínimo.
#[test]
fn motor_y_simplex_de_tablas_coinciden() {
    let mut seed: u64 = 12345;
    let mut rnd = |a: f64, b: f64| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        a + (b - a) * ((seed >> 33) as f64 / (1u64 << 31) as f64)
    };
    let mut cases = vec![wyndor()];
    for k in 0..40 {
        let n = 2 + k % 4;
        let m = 2 + (k / 4) % 4;
        let max = k % 2 == 0;
        let vars: Vec<Variable> = (0..n).map(|j| var(&format!("X{}", j + 1), (rnd(1.0, 9.0) * 4.0).round() / 4.0)).collect();
        let cons: Vec<Constraint> = (0..m)
            .map(|i| {
                let coefs: Vec<f64> = (0..n).map(|_| (rnd(0.0, 6.0) * 2.0).round() / 2.0 + 0.5).collect();
                let d = if max { Direction::Le } else { Direction::Ge };
                con(&format!("C{}", i + 1), &coefs, d, (rnd(5.0, 40.0)).round())
            })
            .collect();
        cases.push(LpModel { objectives: vec![], start: None, name: format!("azar {k}"), objective: if max { Objective::Max } else { Objective::Min }, variables: vars, constraints: cons });
    }
    let mut compared = 0;
    for m in cases {
        let a = solve(&m, &SolveOptions::default()).unwrap();
        let b = solve(&m, &SolveOptions { engine_only: true, ..Default::default() }).unwrap();
        assert_eq!(a.status, b.status, "{}", m.name);
        assert!(close(a.objective_value.unwrap(), b.objective_value.unwrap()), "{}", m.name);
        // la sensibilidad solo es única si no hay degeneración ni óptimos alternativos
        let degenerate = a.variables.iter().any(|v| v.basis_status == BasisStatus::Basic && v.value.abs() < 1e-9)
            || a.constraints.iter().filter(|c| c.slack_or_surplus.abs() < 1e-9).count() + a.variables.iter().filter(|v| v.value.abs() > 1e-9).count() != m.constraints.len().max(1) + 0 && false;
        if a.alternate_optima || b.alternate_optima || degenerate {
            continue;
        }
        let basics = a.variables.iter().filter(|v| v.value.abs() > 1e-9).count() + a.constraints.iter().filter(|c| c.slack_or_surplus.abs() > 1e-9).count();
        if basics != m.constraints.len() {
            continue; // degenerado
        }
        compared += 1;
        for (x, y) in a.variables.iter().zip(&b.variables) {
            assert!(close(x.value, y.value), "{}: {} {} vs {}", m.name, x.name, x.value, y.value);
            assert!(close(x.reduced_cost, y.reduced_cost), "{}: costo reducido de {}: {} vs {}", m.name, x.name, x.reduced_cost, y.reduced_cost);
            assert!(close(x.cost_min.unwrap().0, y.cost_min.unwrap().0) || (x.cost_min.unwrap().0.is_infinite() && y.cost_min.unwrap().0.is_infinite()), "{}: cmin {} {:?} vs {:?}", m.name, x.name, x.cost_min, y.cost_min);
            assert!(close(x.cost_max.unwrap().0, y.cost_max.unwrap().0) || (x.cost_max.unwrap().0.is_infinite() && y.cost_max.unwrap().0.is_infinite()), "{}: cmax {} {:?} vs {:?}", m.name, x.name, x.cost_max, y.cost_max);
        }
        for (x, y) in a.constraints.iter().zip(&b.constraints) {
            assert!(close(x.shadow_price.unwrap(), y.shadow_price.unwrap()), "{}: precio sombra de {}: {:?} vs {:?}", m.name, x.name, x.shadow_price, y.shadow_price);
            let (p, q) = (x.rhs_min.unwrap().0, y.rhs_min.unwrap().0);
            assert!(close(p, q) || (p.is_infinite() && q.is_infinite()), "{}: rhs_min {}: {p} vs {q}", m.name, x.name);
            let (p, q) = (x.rhs_max.unwrap().0, y.rhs_max.unwrap().0);
            assert!(close(p, q) || (p.is_infinite() && q.is_infinite()), "{}: rhs_max {}: {p} vs {q}", m.name, x.name);
        }
    }
    assert!(compared >= 15, "solo se compararon {compared}");
}

#[test]
fn diagnostico_de_infactibilidad() {
    let mut m = wyndor();
    m.constraints.push(con("Mínimo de ventanas", &[0.0, 1.0], Direction::Ge, 7.0));
    let r = solve(&m, &SolveOptions::default()).unwrap();
    assert_eq!(r.status, Status::Infeasible);
    let d = r.extra.diagnosis.expect("sin diagnóstico");
    let names: Vec<&str> = d.conflict.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"C2") && names.contains(&"Mínimo de ventanas"), "{names:?}");
    assert!(!names.contains(&"C1"), "{names:?}");
    // lo más barato: bajar el mínimo de ventanas de 7 a 6 (C2 permite X2 ≤ 6)
    assert!((d.relaxation_total.unwrap() - 1.0).abs() < 1e-6, "{d:?}");
}

#[test]
fn terminos_dispersos_semicontinuas_y_multiobjetivo() {
    // restricciones con «terms» en lugar de coeficientes densos
    let mut m = wyndor();
    for c in &mut m.constraints {
        let t: std::collections::BTreeMap<String, f64> = c.coefs.iter().zip(["X1", "X2"]).filter(|(a, _)| **a != 0.0).map(|(a, n)| (n.to_string(), *a)).collect();
        c.coefs.clear();
        c.terms = Some(t);
    }
    let r = solve(&m, &SolveOptions::default()).unwrap();
    assert!(close(r.objective_value.unwrap(), 36.0));
    // semicontinua: X1 = 0 o entre 3 y 4
    let mut s = wyndor();
    s.objective = Objective::Min;
    s.variables[0].cost = 1.0;
    s.variables[0].lower = Bound(3.0);
    s.variables[0].var_type = VarType::SemiContinuous;
    s.constraints.push(con("Algo", &[1.0, 1.0], Direction::Ge, 1.0));
    let r = solve(&s, &SolveOptions::default()).unwrap();
    // X1 = 3 cuesta 3 y X2 = 1 cuesta 5: conviene X1 en su mínimo de 3
    assert!(close(r.variables[0].value, 3.0) && close(r.variables[1].value, 0.0), "{:?}", r.variables);
    // con X1 más cara, X1 = 0 (la parte «semi») y X2 = 1
    s.variables[0].cost = 2.0;
    let r = solve(&s, &SolveOptions::default()).unwrap();
    assert!(close(r.variables[0].value, 0.0) && close(r.variables[1].value, 1.0), "{:?}", r.variables);
    assert_eq!(r.method, "branch_and_cut");
}
