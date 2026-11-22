//! QP-IQP: ejemplo de Hillier (método de Wolfe), cartera, archivos y casos límite.

use iaqsb_core::Module;
use iaqsb_modules::qp::QpMod;
use serde_json::{json, Value};

fn ex(name: &str) -> Value {
    Module::examples(&QpMod).into_iter().find(|(n, _)| n == name).unwrap().1
}

fn x(r: &iaqsb_core::Report) -> Vec<f64> {
    r.data["variables"].as_array().unwrap().iter().map(|v| v["value"].as_f64().unwrap()).collect()
}

#[test]
fn hillier_wolfe() {
    let r = Module::solve(&QpMod, &ex("wolfe"), &Value::Null).unwrap();
    // máx 15x1 + 30x2 + 4x1x2 − 2x1² − 4x2², x1 + 2x2 ≤ 30 → (12, 9), Z = 270, u = 3
    assert_eq!(x(&r), vec![12.0, 9.0]);
    assert_eq!(r.data["objective_value"], 270.0);
    assert_eq!(r.data["shadow_prices"], json!([3.0]));
    assert_eq!(r.data["convexity"], "cóncava");
    let pasos = serde_json::to_string(&r.sections.iter().find(|s| s.id == "pasos").unwrap().blocks).unwrap();
    assert!(pasos.contains("Método de Wolfe") && pasos.contains("Iteración 3"));
}

#[test]
fn cartera_markowitz() {
    let r = Module::solve(&QpMod, &ex("cartera"), &Value::Null).unwrap();
    let v = x(&r);
    // referencia: SciPy SLSQP
    for (a, b) in v.iter().zip([0.114260794, 0.554295683, 0.331443524]) {
        assert!((a - b).abs() < 1e-6, "{v:?}");
    }
    assert!((v.iter().sum::<f64>() - 1.0).abs() < 1e-9);
}

#[test]
fn archivos_y_forma_normal() {
    let m = ex("wolfe");
    for normal in [false, true] {
        let mut mm = m.clone();
        mm["normal_form"] = json!(normal);
        let out = Module::write_file(&QpMod, &mm).unwrap();
        let back = Module::read_file(&QpMod, &out).unwrap();
        let r = Module::solve(&QpMod, &back, &Value::Null).unwrap();
        assert_eq!(r.data["objective_value"], 270.0, "{}", iaqsb_core::winqsb::decode(&out));
    }
    let txt = "QP\tNormalModel\tP\t2\t1\r\nOBJ/Constraint/VariableType/Bound\t\r\nMinimize\tX^2 + Y^2 - 2X - 4Y\r\nC1\tX+Y<=2\r\n\
               Integer:\t\r\nBinary:\t\r\nUnrestricted:\t\r\nX\t>=0, <=M\r\nY\t>=0, <=M\r\n";
    let m = Module::read_file(&QpMod, txt.as_bytes()).unwrap();
    let r = Module::solve(&QpMod, &m, &Value::Null).unwrap();
    assert_eq!(x(&r), vec![0.5, 1.5]);
    assert_eq!(r.data["objective_value"], -4.5);
}

#[test]
fn no_convexa_entera_y_limites() {
    // mín −x² − y², x + y ≤ 3, 0 ≤ x, y ≤ 2 → −5 (global, en un vértice)
    let m = json!({"name": "nc", "objective": "min",
        "variables": [{"name": "X", "upper": 2}, {"name": "Y", "upper": 2}],
        "quad": [[-1, 0], [0, -1]], "constraints": [{"name": "C", "direction": "<=", "rhs": 3}], "coefs": [[1, 1]]});
    let r = Module::solve(&QpMod, &m, &Value::Null).unwrap();
    assert_eq!(r.data["objective_value"], -5.0);
    assert_eq!(r.data["convexity"], "cóncava");
    // entera
    let m = json!({"name": "e", "objective": "min",
        "variables": [{"name": "X", "c": -2.8, "type": "integer", "upper": 10}, {"name": "Y", "c": -5.2, "type": "integer", "upper": 10}],
        "quad": [[1, 0], [0, 1]], "constraints": [], "coefs": []});
    assert_eq!(x(&Module::solve(&QpMod, &m, &Value::Null).unwrap()), vec![1.0, 3.0]);
    // no acotado y no factible
    let unb = json!({"name": "u", "objective": "max", "variables": [{"name": "X"}], "quad": [[1]], "constraints": [], "coefs": []});
    assert_eq!(Module::solve(&QpMod, &unb, &Value::Null).unwrap().status, iaqsb_core::Status::Unbounded);
    let inf = json!({"name": "i", "objective": "min", "variables": [{"name": "X"}], "quad": [[1]],
        "constraints": [{"name": "A", "direction": ">=", "rhs": 5}, {"name": "B", "direction": "<=", "rhs": 1}], "coefs": [[1], [1]]});
    assert_eq!(Module::solve(&QpMod, &inf, &Value::Null).unwrap().status, iaqsb_core::Status::Infeasible);
}
