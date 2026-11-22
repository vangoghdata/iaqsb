//! FLL: mediana ponderada, centro de gravedad, Weiszfeld, distribución y balanceo de Heizer y Render.

use iaqsb_core::Module;
use iaqsb_modules::fll::Fll;
use serde_json::{json, Value};

fn ex(name: &str) -> Value {
    Module::examples(&Fll).into_iter().find(|(n, _)| n == name).unwrap().1
}
fn f(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

#[test]
fn localizacion() {
    let m = ex("almacen_nuevo");
    let r = Module::solve(&Fll, &m, &Value::Null).unwrap();
    assert_eq!(r.data["locations"][0]["coordinates"], json!([2.0, 2.0]));
    assert_eq!(r.data["total_cost"], 67.0);
    // centro de gravedad: (5·1 + 3·5 + 2·2 + 4·7)/14
    let r = Module::solve(&Fll, &m, &json!({"distance": "squared"})).unwrap();
    assert!((f(&r.data["locations"][0]["coordinates"][0]) - 52.0 / 14.0).abs() < 1e-9);
    // euclidiana: no peor que las otras dos soluciones evaluadas con distancia euclidiana
    let r = Module::solve(&Fll, &m, &json!({"distance": "euclidean"})).unwrap();
    assert!(f(&r.data["total_cost"]) < 49.36);
}

#[test]
fn distribucion() {
    let r = Module::solve(&Fll, &ex("distribucion_oficinas"), &Value::Null).unwrap();
    assert!(f(&r.data["final_cost"]) <= f(&r.data["initial_cost"]));
    let r3 = Module::solve(&Fll, &ex("distribucion_oficinas"), &json!({"exchange": "three_then_two"})).unwrap();
    assert!(f(&r3.data["final_cost"]) <= f(&r3.data["initial_cost"]));
}

#[test]
fn balanceo_render() {
    for meth in ["heuristic", "best_bud", "comsoal"] {
        let r = Module::solve(&Fll, &ex("balanceo_linea"), &json!({"method": meth})).unwrap();
        assert_eq!(r.data["stations"].as_array().unwrap().len(), 6, "{meth}");
        assert!((f(&r.data["efficiency"]) - 66.0 / 72.0).abs() < 1e-3);
    }
}

#[test]
fn archivos_fll() {
    for name in ["almacen_nuevo", "dos_plantas_nuevas", "distribucion_oficinas", "balanceo_linea"] {
        let m = ex(name);
        let out = Module::write_file(&Fll, &m).unwrap();
        let back = Module::read_file(&Fll, &out).unwrap();
        let a = Module::solve(&Fll, &m, &Value::Null).unwrap();
        let b = Module::solve(&Fll, &back, &Value::Null).unwrap();
        assert_eq!(a.headline, b.headline, "{}", iaqsb_core::winqsb::decode(&out));
    }
}
