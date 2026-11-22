//! MKP: inventario de cámaras de Hillier, cuentas por cobrar de Render, periodicidad y archivos.

use iaqsb_core::Module;
use iaqsb_modules::mkp::Mkp;
use serde_json::{json, Value};

fn ex(name: &str) -> Value {
    Module::examples(&Mkp).into_iter().find(|(n, _)| n == name).unwrap().1
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

#[test]
fn camaras_hillier() {
    let r = Module::solve(&Mkp, &ex("inventario_camaras"), &Value::Null).unwrap();
    let ss = &r.data["steady_state"];
    for (i, want) in [0.286, 0.285, 0.263, 0.166].iter().enumerate() {
        assert!((f(&ss[i]) - want).abs() < 1.5e-3, "{i}: {}", ss[i]);
    }
    // tiempos de primer paso hacia el estado 0: μ10 = 1.58, μ20 = 2.51, μ30 = 3.50
    let fp = &r.data["first_passage"];
    assert!((f(&fp[1][0]) - 1.58).abs() < 5e-3);
    assert!((f(&fp[2][0]) - 2.51).abs() < 1e-2);
    assert!((f(&fp[3][0]) - 3.50).abs() < 5e-3);
    assert!((f(&fp[0][0]) - 1.0 / f(&ss[0])).abs() < 1e-7);
    // costo esperado de largo plazo (Hillier: 5.662 con π redondeadas)
    assert!((f(&r.data["long_run_cost"]) - 5.669).abs() < 1e-3);
}

#[test]
fn cuentas_render() {
    let r = Module::solve(&Mkp, &ex("cuentas_por_cobrar"), &Value::Null).unwrap();
    assert_eq!(r.data["unique_steady_state"], false);
    let a = &r.data["absorption"];
    assert!((f(&a[0]["probabilities"][0]) - 0.966).abs() < 1e-3);
    assert!((f(&a[1]["probabilities"][1]) - 0.138).abs() < 1e-3);
}

#[test]
fn periodica_y_errores() {
    let m = json!({"name": "p", "states": [{"name": "A", "initial": 1}, {"name": "B"}, {"name": "C"}], "transitions": [[0, 1, 0], [0, 0, 1], [1, 0, 0]]});
    let r = Module::solve(&Mkp, &m, &Value::Null).unwrap();
    assert_eq!(r.data["classes"][0]["period"], 3);
    assert!((f(&r.data["steady_state"][1]) - 1.0 / 3.0).abs() < 1e-9);
    let mut bad = m.clone();
    bad["transitions"][1] = json!([0.5, 0, 0.4]);
    let e = Module::solve(&Mkp, &bad, &Value::Null).unwrap_err().to_string();
    assert!(e.contains("desde B"), "{e}");
}

#[test]
fn archivos_mkp() {
    for name in ["inventario_camaras", "cambio_de_marca", "cuentas_por_cobrar"] {
        let m = ex(name);
        let out = Module::write_file(&Mkp, &m).unwrap();
        let back = Module::read_file(&Mkp, &out).unwrap();
        let a = Module::solve(&Mkp, &m, &Value::Null).unwrap();
        let b = Module::solve(&Mkp, &back, &Value::Null).unwrap();
        assert_eq!(a.headline, b.headline, "{}", iaqsb_core::winqsb::decode(&out));
    }
}
