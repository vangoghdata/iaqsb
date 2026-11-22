//! ITS: EOQ de Hillier, descuentos de Render, vendedor de periódicos de Hillier,
//! Wagner-Whitin, sistemas aleatorios contra simulación, y archivos.

use iaqsb_core::Module;
use iaqsb_modules::its::Its;
use serde_json::{json, Value};

fn ex(name: &str) -> Value {
    Module::examples(&Its).into_iter().find(|(n, _)| n == name).unwrap().1
}
fn f(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

#[test]
fn eoq_hillier() {
    let r = Module::solve(&Its, &ex("eoq_altavoces"), &Value::Null).unwrap();
    assert!((f(&r.data["order_quantity"]) - 25298.2).abs() < 0.1);
    let r = Module::solve(&Its, &ex("eoq_faltantes"), &Value::Null).unwrap();
    assert!((f(&r.data["order_quantity"]) - 28540.2).abs() < 0.1);
    assert!((f(&r.data["max_inventory"]) - 22424.5).abs() < 0.1);
    // producción gradual: Q* = 25298 / √(1 − 8000/10000) = 56569
    let mut m = ex("eoq_altavoces");
    m["production_rate"] = json!(10000);
    let r = Module::solve(&Its, &m, &Value::Null).unwrap();
    assert!((f(&r.data["order_quantity"]) - 56568.5).abs() < 0.1);
}

#[test]
fn descuentos_render() {
    let r = Module::solve(&Its, &ex("descuentos"), &Value::Null).unwrap();
    assert_eq!(r.data["order_quantity"], 1000.0);
    assert_eq!(r.data["total_cost"], 24725.0);
}

#[test]
fn periodicos_hillier() {
    let r = Module::solve(&Its, &ex("periodicos"), &Value::Null).unwrap();
    assert_eq!(r.data["order_quantity"], 45.0);
    assert!((f(&r.data["critical_ratio"]) - 0.5).abs() < 1e-12);
}

#[test]
fn tamano_de_lote() {
    let r = Module::solve(&Its, &ex("tamano_lote"), &Value::Null).unwrap();
    let opt = f(&r.data["optimal_cost"]);
    assert_eq!(opt, 610.0);
    for m in r.data["methods"].as_array().unwrap() {
        assert!(f(&m["total_cost"]) >= opt - 1e-9, "{m}");
    }
}

#[test]
fn sistemas_contra_simulacion() {
    for name in ["sistema_sq", "sistema_rs", "sistema_rss"] {
        let r = Module::solve(&Its, &ex(name), &json!({"simulation_time": 60000, "warmup": 100})).unwrap();
        let a = f(&r.data["related_cost"]);
        let s = f(&r.data["simulation"]["related_cost"]);
        assert!((a - s).abs() / a < 0.04, "{name}: analítico {a}, simulado {s}");
    }
    // nivel de servicio: s = μ + 1.645·σ
    let r = Module::solve(&Its, &ex("sistema_sq"), &json!({"mode": "service", "service_level": 95})).unwrap();
    assert!((f(&r.data["reorder_point"]) - (200.0 + 1.6448536 * 800f64.sqrt())).abs() < 1e-3);
}

#[test]
fn archivos_its() {
    let names = ["eoq_altavoces", "eoq_faltantes", "descuentos", "periodicos", "tamano_lote", "sistema_sq", "sistema_rs", "sistema_rss"];
    for name in names {
        let m = ex(name);
        let out = Module::write_file(&Its, &m).unwrap();
        let back = Module::read_file(&Its, &out).unwrap();
        let a = Module::solve(&Its, &m, &Value::Null).unwrap();
        let b = Module::solve(&Its, &back, &Value::Null).unwrap();
        assert_eq!(a.headline, b.headline, "{name}\n{}", iaqsb_core::winqsb::decode(&out));
    }
}
