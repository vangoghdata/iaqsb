//! DA: Goferbroke de Hillier (VEIP, VEIM), Bayes, juego de suma cero, árbol y archivos.

use iaqsb_core::Module;
use iaqsb_modules::da::Da;
use serde_json::{json, Value};

fn ex(name: &str) -> Value {
    Module::examples(&Da).into_iter().find(|(n, _)| n == name).unwrap().1
}

#[test]
fn goferbroke_hillier() {
    let r = Module::solve(&Da, &ex("goferbroke"), &Value::Null).unwrap();
    assert_eq!(r.data["evpi"], 142.5);
    assert_eq!(r.data["ev_with_perfect_information"], 242.5);
    // con el sondeo sísmico: 0.3·300 + 0.7·90 = 153, VEIM = 53
    assert_eq!(r.data["sample"]["ev_with_sample_information"], 153.0);
    assert_eq!(r.data["sample"]["evsi"], 53.0);
    let c = r.data["criteria"].as_array().unwrap();
    assert_eq!(c[0]["decision"], "Vender");
    assert_eq!(c[5]["decision"], "Perforar");
    assert_eq!(c[6]["value"], 142.5);
}

#[test]
fn bayes_y_arbol() {
    let r = Module::solve(&Da, &ex("bayes_sismico"), &Value::Null).unwrap();
    assert_eq!(r.data["marginal"], json!([0.3, 0.7]));
    assert_eq!(r.data["posterior"][0], json!([0.5, 0.5]));
    let r = Module::solve(&Da, &ex("arbol_decision"), &Value::Null).unwrap();
    assert_eq!(r.data["expected_value"], 123.0);
    assert!(r.headline.contains("Sondeo"));
}

#[test]
fn juego_hillier() {
    // Hillier, variante 3: valor 2/11, x = (7/11, 4/11, 0), y = (0, 5/11, 6/11)
    let r = Module::solve(&Da, &ex("juego_campana"), &json!({"games": 5000})).unwrap();
    let v = r.data["value"].as_f64().unwrap();
    assert!((v - 2.0 / 11.0).abs() < 1e-9);
    let x: Vec<f64> = serde_json::from_value(r.data["player1"].clone()).unwrap();
    let y: Vec<f64> = serde_json::from_value(r.data["player2"].clone()).unwrap();
    assert!((x[0] - 7.0 / 11.0).abs() < 1e-9 && x[2].abs() < 1e-9);
    assert!((y[2] - 6.0 / 11.0).abs() < 1e-9 && y[0].abs() < 1e-9);
    assert_eq!(r.data["saddle_point"], false);
    // punto de silla
    let m = json!({"name": "s", "problem_type": "zero_sum", "player1": [{"name": "a"}, {"name": "b"}], "player2": [{"name": "c"}, {"name": "d"}], "game": [[3, 5], [1, 4]]});
    let r = Module::solve(&Da, &m, &Value::Null).unwrap();
    assert_eq!(r.data["saddle_point"], true);
    assert_eq!(r.data["value"], 3.0);
}

#[test]
fn archivos_da() {
    for name in ["goferbroke", "bayes_sismico", "juego_campana", "arbol_decision"] {
        let m = ex(name);
        let out = Module::write_file(&Da, &m).unwrap();
        let back = Module::read_file(&Da, &out).unwrap();
        let a = Module::solve(&Da, &m, &Value::Null).unwrap();
        let b = Module::solve(&Da, &back, &Value::Null).unwrap();
        assert_eq!(a.headline, b.headline, "{}", iaqsb_core::winqsb::decode(&out));
    }
}

#[test]
fn errores_claros() {
    let mut m = ex("goferbroke");
    m["states"][0]["prior"] = json!(0.5);
    let e = Module::solve(&Da, &m, &Value::Null).unwrap_err().to_string();
    assert!(e.contains("sumar 1"), "{e}");
    let mut m = ex("arbol_decision");
    m["nodes"][7]["probability"] = json!(0.6);
    let e = Module::solve(&Da, &m, &Value::Null).unwrap_err().to_string();
    assert!(e.contains("Perforar F"), "{e}");
}
