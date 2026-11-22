//! FC: regresión de Render, los doce métodos, búsqueda de parámetros y archivos.

use iaqsb_core::Module;
use iaqsb_modules::fc::{Fc, Method};
use serde_json::{json, Value};

fn ex(name: &str) -> Value {
    Module::examples(&Fc).into_iter().find(|(n, _)| n == name).unwrap().1
}
fn f(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

#[test]
fn regresion_render() {
    // Triple A: ventas = 2 + 1.25·nómina, R² = 0.6944, Se = 1.31
    let r = Module::solve(&Fc, &ex("regresion_ventas"), &json!({"predict": "6"})).unwrap();
    let c = &r.data["coefficients"];
    assert!((f(&c[0]["coefficient"]) - 2.0).abs() < 1e-9);
    assert!((f(&c[1]["coefficient"]) - 1.25).abs() < 1e-9);
    assert!((f(&r.data["r_square"]) - 15.625 / 22.5).abs() < 1e-4);
    assert!((f(&r.data["std_error"]) - (6.875f64 / 4.0).sqrt()).abs() < 1e-4);
    assert!((f(&r.data["prediction"]["value"]) - 9.5).abs() < 1e-9);
}

#[test]
fn doce_metodos() {
    let m = ex("estacional_trimestral");
    for meth in Method::ALL {
        let r = Module::solve(&Fc, &m, &json!({"method": meth.id(), "cycle": 4, "horizon": 4})).unwrap();
        assert_eq!(r.data["forecast"].as_array().unwrap().len(), 4, "{meth:?}");
    }
    // la búsqueda no empeora el error
    for meth in ["single_exponential", "single_exponential_trend", "holt_winters_multiplicative", "moving_average"] {
        let a = Module::solve(&Fc, &m, &json!({"method": meth})).unwrap();
        let b = Module::solve(&Fc, &m, &json!({"method": meth, "search": true})).unwrap();
        assert!(f(&b.data["mse"]) <= f(&a.data["mse"]) + 1e-9, "{meth}");
    }
    // Holt-Winters capta la estacionalidad mejor que el suavizamiento simple
    let hw = Module::solve(&Fc, &m, &json!({"method": "holt_winters_multiplicative", "search": true})).unwrap();
    let ses = Module::solve(&Fc, &m, &json!({"method": "single_exponential", "search": true})).unwrap();
    assert!(f(&hw.data["mse"]) < f(&ses.data["mse"]));
}

#[test]
fn archivos_fc() {
    for name in ["ventas_mensuales", "estacional_trimestral", "regresion_ventas"] {
        let m = ex(name);
        let out = Module::write_file(&Fc, &m).unwrap();
        let back = Module::read_file(&Fc, &out).unwrap();
        let a = Module::solve(&Fc, &m, &Value::Null).unwrap();
        let b = Module::solve(&Fc, &back, &Value::Null).unwrap();
        assert_eq!(a.headline, b.headline, "{}", iaqsb_core::winqsb::decode(&out));
    }
}
