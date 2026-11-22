//! JOB: regla de Johnson (Render), enumeración, heurísticas de flujo, despacho y archivos.

use iaqsb_core::Module;
use iaqsb_modules::job::JobMod;
use serde_json::{json, Value};

fn ex(name: &str) -> Value {
    Module::examples(&JobMod).into_iter().find(|(n, _)| n == name).unwrap().1
}
fn f(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

#[test]
fn johnson_render() {
    let r = Module::solve(&JobMod, &ex("flujo_johnson"), &json!({"flow_method": "johnson"})).unwrap();
    assert_eq!(r.data["sequence"], json!(["B", "E", "D", "C", "A"]));
    assert_eq!(r.data["measures"]["cmax"], 35.0);
}

#[test]
fn heuristicas_y_optimo() {
    let m = ex("flujo_cinco_maquinas");
    let opt = f(&Module::solve(&JobMod, &m, &json!({"flow_method": "enumeration"})).unwrap().data["measures"]["cmax"]);
    assert_eq!(opt, 62.0);
    for h in ["cds", "johnson", "palmer", "gupta", "dannenbring", "hundal_rajgopal", "ho_chang", "neh", "all_heuristics", "random"] {
        let c = f(&Module::solve(&JobMod, &m, &json!({"flow_method": h, "schedules": 200})).unwrap().data["measures"]["cmax"]);
        assert!(c >= opt, "{h}: {c} < {opt}");
    }
}

#[test]
fn taller_reglas() {
    let m = ex("taller_cuatro_trabajos");
    for rule in ["fcfs", "lcfs", "spt", "lpt", "edd", "cr", "slack", "slack_per_op", "mwkr", "lwkr", "mopnr", "lopnr", "wspt", "priority_index", "random"] {
        let r = Module::solve(&JobMod, &m, &json!({"rule": rule})).unwrap();
        assert!(f(&r.data["measures"]["cmax"]) >= 10.0, "{rule}");
    }
    let all = Module::solve(&JobMod, &m, &json!({"shop_method": "all_rules", "criterion": "mt"})).unwrap();
    let spt = Module::solve(&JobMod, &m, &json!({"rule": "spt", "criterion": "mt"})).unwrap();
    assert!(f(&all.data["measures"]["mt"]) <= f(&spt.data["measures"]["mt"]));
}

#[test]
fn archivos_job() {
    for name in ["taller_cuatro_trabajos", "flujo_johnson", "flujo_cinco_maquinas"] {
        let m = ex(name);
        let out = Module::write_file(&JobMod, &m).unwrap();
        let back = Module::read_file(&JobMod, &out).unwrap();
        let a = Module::solve(&JobMod, &m, &Value::Null).unwrap();
        let b = Module::solve(&JobMod, &back, &Value::Null).unwrap();
        assert_eq!(a.headline, b.headline, "{}", iaqsb_core::winqsb::decode(&out));
    }
}
