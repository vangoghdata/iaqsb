//! PERT-CPM: respuestas de libro (Hillier, Render), archivos .CPM y errores.

use iaqsb_core::Module;
use iaqsb_modules::pert::Pert;
use serde_json::{json, Value};

fn example(name: &str) -> Value {
    Module::examples(&Pert).into_iter().find(|(n, _)| n == name).unwrap().1
}

fn solve(model: &Value, opts: Value) -> iaqsb_core::Report {
    Module::solve(&Pert, model, &opts).unwrap()
}

#[test]
fn reliable_ruta_critica_y_compresion() {
    let m = example("proyecto_cpm");
    let r = solve(&m, json!({}));
    assert_eq!(r.data["completion_time"], 44.0);
    assert_eq!(r.data["critical_paths"], json!([["A", "B", "C", "E", "F", "J", "L", "N"]]));
    assert_eq!(r.data["total_cost"], 4550.0);
    // Hillier: terminar en 40 semanas cuesta 140 mil más (F y J, 2 semanas cada una)
    let r = solve(&m, json!({"crashing": "deadline", "desired_time": 40}));
    let c = &r.data["crashing"];
    assert_eq!(c["suggested_time"], 40.0);
    assert!((c["additional_cost"].as_f64().unwrap() - 140.0).abs() < 1e-6);
    assert_eq!(c["crash_time"], 28.0);
    // plazo imposible
    let r = solve(&m, json!({"crashing": "deadline", "desired_time": 27}));
    assert_eq!(r.status, iaqsb_core::Status::Infeasible);
    // con los tiempos de compresión
    let r = solve(&m, json!({"time_basis": "crash"}));
    assert_eq!(r.data["completion_time"], 28.0);
}

#[test]
fn reliable_pert_probabilidad() {
    let m = example("proyecto_pert");
    let r = solve(&m, json!({"desired_time": 47, "simulation_runs": 2000}));
    assert_eq!(r.data["expected_completion_time"], 44.0);
    assert!((r.data["std_dev"].as_f64().unwrap() - 3.0).abs() < 1e-9);
    // Hillier: P(T ≤ 47) = Φ(1) = 0.8413
    assert!((r.data["probability"]["project"].as_f64().unwrap() - 0.841345).abs() < 1e-5);
    let p = r.data["simulation"]["probability"].as_f64().unwrap();
    assert!(p > 0.78 && p < 0.88, "{p}");
    // misma semilla, mismo resultado
    let r2 = solve(&m, json!({"desired_time": 47, "simulation_runs": 2000}));
    assert_eq!(r.data["simulation"], r2.data["simulation"]);
}

#[test]
fn general_foundry_control() {
    let m = example("control_costos");
    let r = solve(&m, json!({"current_time": 6}));
    assert_eq!(r.data["completion_time"], 15.0);
    let c = &r.data["control"];
    // Render: valor del trabajo hecho 100, costo real 112, sobrecosto 12
    assert_eq!(c["earned_value"], 100.0);
    assert_eq!(c["actual_cost"], 112.0);
    assert_eq!(c["overrun"], 12.0);
    assert_eq!(c["late_activities"], json!(["D", "E"]));
    assert!((c["forecast_completion"].as_f64().unwrap() - 16.6).abs() < 1e-9);
}

#[test]
fn archivo_cpm_de_winqsb() {
    // como lo guarda WinQSB: indicadores -1/0 y predecesoras por nombre o número
    let txt = "CPM\tCasa\tweeks\t4\tMatrix\r\n-1\t-1\t-1\t-1\t0\t0\r\n\
               Activity No.\tActivity Name\tImmediate Predecessor\tNormal Time\tCrash Time\tNormal Cost\tCrash Cost\r\n\
               1\tA\t\t3\t2\t100\t150\r\n2\tB\t1\t4\t2\t200\t300\r\n3\tC\tA\t2\t2\t50\t50\r\n4\tD\tB,C\t5\t3\t400\t500\r\n0\r\n";
    let m = Module::read_file(&Pert, txt.as_bytes()).unwrap();
    assert_eq!(m["problem_type"], "cpm");
    assert_eq!(m["time_unit"], "week");
    assert_eq!(m["activities"][3]["predecessors"], json!(["B", "C"]));
    assert_eq!(m["activities"][1]["crash_cost"], 300.0);
    let r = solve(&m, json!({}));
    assert_eq!(r.data["completion_time"], 12.0);
    // ida y vuelta
    let out = Module::write_file(&Pert, &m).unwrap();
    let m2 = Module::read_file(&Pert, &out).unwrap();
    assert_eq!(m, m2);
    assert!(Module::recognizes(&Pert, "CPM\tx") && Module::recognizes(&Pert, "PERT\tx"));
}

#[test]
fn archivo_pert_mixto_y_discreto() {
    let m = json!({
        "name": "Mixto", "problem_type": "pert", "time_unit": "día", "time_distribution": "mixed",
        "activities": [
            {"name": "A", "predecessors": [], "distribution": "discrete", "p1": 3, "discrete": "2/0.2,4/0.5,6/0.3"},
            {"name": "B", "predecessors": ["A"], "distribution": "normal", "p1": 5, "p2": 1},
            {"name": "C", "predecessors": ["A"], "distribution": "three_time", "p1": 2, "p2": 4, "p3": 12}
        ],
        "positions": [[1, 1], [1, 3], [3, 3]]
    });
    let out = Module::write_file(&Pert, &m).unwrap();
    let text = iaqsb_core::winqsb::decode(&out);
    assert!(text.starts_with("PERT\tMixto\tdía\t3\tMatrix\r\nMixed\r\n"), "{text}");
    let m2 = Module::read_file(&Pert, &out).unwrap();
    let a: Value = serde_json::from_value(m2.clone()).unwrap();
    assert_eq!(a["activities"][0]["discrete"], "2/0.2,4/0.5,6/0.3");
    assert_eq!(a["activities"][2]["distribution"], "three_time");
    assert_eq!(a["positions"], json!([[1, 1], [1, 3], [3, 3]]));
    let r = solve(&m2, json!({"simulation_runs": 0}));
    assert!((r.data["expected_completion_time"].as_f64().unwrap() - 9.2).abs() < 1e-9);
    // una sola distribución para todas: solo sus parámetros
    let m = json!({"name": "U", "problem_type": "pert", "time_distribution": "uniform",
        "activities": [{"name": "A", "predecessors": [], "p1": 1, "p2": 3}]});
    let out = Module::write_file(&Pert, &m).unwrap();
    let text = iaqsb_core::winqsb::decode(&out);
    assert!(text.contains("Uniform\r\n") && text.contains("1\tA\t\t1\t3\r\n"), "{text}");
}

#[test]
fn errores_claros() {
    let ciclo = json!({"name": "c", "problem_type": "cpm", "activities": [
        {"name": "A", "predecessors": ["C"], "normal_time": 1},
        {"name": "B", "predecessors": ["A"], "normal_time": 1},
        {"name": "C", "predecessors": ["B"], "normal_time": 1}]});
    let e = Module::solve(&Pert, &ciclo, &Value::Null).unwrap_err().to_string();
    assert!(e.contains("ciclo") && e.contains("A → B → C → A"), "{e}");
    let falta = json!({"name": "c", "problem_type": "cpm", "activities": [{"name": "A", "predecessors": ["Z"], "normal_time": 1}]});
    assert!(Module::solve(&Pert, &falta, &Value::Null).unwrap_err().to_string().contains("«Z»"));
    let tres = json!({"name": "c", "problem_type": "pert", "activities": [{"name": "A", "predecessors": [], "p1": 5, "p2": 3, "p3": 9}]});
    assert!(Module::solve(&Pert, &tres, &Value::Null).unwrap_err().to_string().contains("optimista"));
}
