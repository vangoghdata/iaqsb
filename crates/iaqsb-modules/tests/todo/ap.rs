//! AP: modelo de transporte (Farnsworth Tire Company, de Heizer y Render), estrategias, PL y archivos.

use iaqsb_core::Module;
use iaqsb_modules::ap::Ap;
use serde_json::{json, Value};

fn ex(name: &str) -> Value {
    Module::examples(&Ap).into_iter().find(|(n, _)| n == name).unwrap().1
}

#[test]
fn transporte_harper() {
    let r = Module::solve(&Ap, &ex("planificacion_transporte"), &Value::Null).unwrap();
    // óptimo comprobado con un programa lineal independiente (SciPy/HiGHS)
    assert_eq!(r.data["total_cost"], 105700.0);
    assert!(r.sections.iter().any(|s| s.id == "pasos"));
}

#[test]
fn estrategias_y_pl() {
    let m = ex("planificacion_simple");
    let lp = Module::solve(&Ap, &m, &Value::Null).unwrap();
    let opt = lp.data["total_cost"].as_f64().unwrap();
    for s in ["level", "periodic_average", "constant_initial", "chase", "chase_overtime"] {
        let r = Module::solve(&Ap, &m, &json!({"strategy": s})).unwrap();
        let c = r.data["total_cost"].as_f64().unwrap();
        assert!(c >= opt - 1e-6, "{s}: {c} < {opt}");
    }
    let user = Module::solve(&Ap, &m, &json!({"strategy": "user", "production": "1400, 1400, 1200, 1100, 1500, 1900"})).unwrap();
    assert!(user.data["total_cost"].as_f64().unwrap() >= opt);
    // el modelo general con medio tiempo no puede ser más caro que el simple
    let pl = Module::solve(&Ap, &ex("planificacion_pl"), &Value::Null).unwrap();
    assert!(pl.data["total_cost"].as_f64().unwrap() <= opt + 1e-6);
}

#[test]
fn archivos_ap() {
    for name in ["planificacion_simple", "planificacion_transporte", "planificacion_pl"] {
        let m = ex(name);
        let out = Module::write_file(&Ap, &m).unwrap();
        let text = iaqsb_core::winqsb::decode(&out);
        assert!(text.contains("Forecast Demand") && text.contains("Initial Inventory (+) or Backorder (-)"), "{text}");
        let back = Module::read_file(&Ap, &out).unwrap();
        let a = Module::solve(&Ap, &m, &Value::Null).unwrap();
        let b = Module::solve(&Ap, &back, &Value::Null).unwrap();
        assert_eq!(a.data["total_cost"], b.data["total_cost"], "{name}");
    }
}
