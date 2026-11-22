//! QA: sala de emergencias de Hillier, M/D/1 de Render, población y capacidad finitas,
//! comparación con la simulación y archivos.

use iaqsb_core::Module;
use iaqsb_modules::qa::Qa;
use serde_json::{json, Value};

fn ex(name: &str) -> Value {
    Module::examples(&Qa).into_iter().find(|(n, _)| n == name).unwrap().1
}
fn f(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

#[test]
fn emergencias_hillier() {
    let r = Module::solve(&Qa, &ex("sala_de_emergencias"), &Value::Null).unwrap();
    assert!((f(&r.data["l"]) - 2.0).abs() < 1e-9 && (f(&r.data["wq"]) - 2.0 / 3.0).abs() < 1e-9);
    // con dos médicos: L = 0.75, Wq = 1/24
    let mut m = ex("sala_de_emergencias");
    m["servers"] = json!(2);
    let r = Module::solve(&Qa, &m, &Value::Null).unwrap();
    assert!((f(&r.data["l"]) - 0.75).abs() < 1e-9 && (f(&r.data["wq"]) - 1.0 / 24.0).abs() < 1e-9);
}

#[test]
fn md1_render() {
    let r = Module::solve(&Qa, &ex("reciclaje_md1"), &Value::Null).unwrap();
    assert!((f(&r.data["lq"]) - 2.0 / 3.0).abs() < 1e-6);
    assert!((f(&r.data["wq"]) - 1.0 / 12.0).abs() < 1e-6);
}

#[test]
fn formulas_contra_simulacion() {
    for name in ["reparacion_maquinas", "barberia_capacidad", "reciclaje_md1"] {
        let r = Module::solve(&Qa, &ex(name), &json!({"simulation_time": 100000})).unwrap();
        let (a, s) = (f(&r.data["l"]), f(&r.data["simulation"]["l"]));
        assert!((a - s).abs() / a < 0.03, "{name}: {a} vs {s}");
    }
    // sin fórmula (servicio constante y cola finita): se resuelve por simulación
    let mut m = ex("reciclaje_md1");
    m["queue_capacity"] = json!(2);
    let r = Module::solve(&Qa, &m, &Value::Null).unwrap();
    assert_eq!(r.data["model"], "simulación");
}

#[test]
fn archivos_qa() {
    for name in ["sala_de_emergencias", "reciclaje_md1", "reparacion_maquinas", "barberia_capacidad", "llegadas_en_lotes", "presion_y_desaliento"] {
        let m = ex(name);
        let out = Module::write_file(&Qa, &m).unwrap();
        let back = Module::read_file(&Qa, &out).unwrap();
        let a = Module::solve(&Qa, &m, &Value::Null).unwrap();
        let b = Module::solve(&Qa, &back, &Value::Null).unwrap();
        assert_eq!(a.headline, b.headline, "{}", iaqsb_core::winqsb::decode(&out));
    }
}

/// Cadena de Markov con lotes, presión y desaliento contra la simulación.
#[test]
fn lotes_presion_y_desaliento() {
    for name in ["llegadas_en_lotes", "presion_y_desaliento"] {
        let r = Module::solve(&Qa, &ex(name), &json!({"simulation_time": 300000, "steps": true})).unwrap();
        assert_eq!(r.data["exact"], true, "{name}");
        let (a, s) = (f(&r.data["l"]), f(&r.data["simulation"]["l"]));
        assert!((a - s).abs() / a < 0.03, "{name}: {a} vs {s}");
    }
    // sin presión ni desaliento ni lotes coincide con M/M/1
    let mut m = ex("presion_y_desaliento");
    m["pressure"] = json!(0);
    m["discourage"] = json!(0);
    m["service"]["p2"] = json!(0.5);
    let r = Module::solve(&Qa, &m, &Value::Null).unwrap();
    assert!((f(&r.data["l"]) - 1.0).abs() < 1e-9);
}

/// Formato real de WinQSB (ejemplos QUEUE1.QA y QUEUE2.QA de la distribución original).
#[test]
fn formato_winqsb_qa() {
    let q1 = "QA\tSample M/M/1 Problem\thour\t0\r\nData Description\tENTRY\r\nNumber of servers\t1\r\nService rate (per server per hour)\t3\r\nCustomer arrival rate (per hour)\t2\r\nQueue capacity (maximum waiting space)\t\r\nCustomer population\t\r\nBusy server cost per hour\t200\r\nIdle server cost per hour\t200\r\nCustomer waiting cost per hour\t50\r\nCustomer being served cost per hour\t50\r\nCost of customer being balked\t \r\nUnit queue capacity cost\t\r\n\r\n";
    let m = Module::read_file(&Qa, q1.as_bytes()).unwrap();
    let r = Module::solve(&Qa, &m, &Value::Null).unwrap();
    assert!((f(&r.data["l"]) - 2.0).abs() < 1e-9 && (f(&r.data["total_cost"]) - 300.0).abs() < 1e-9);
    let q2 = "QA\tQueuing Sample Problem 2\thour\t1\r\nData Description\tENTRY\r\nNumber of servers\t2\r\nService time distribution (in hour)\tNormal\r\nMean (u)\t.66666667\r\nStandard deviation (s>0)\t.2\r\n(Not used)\t\r\nService pressure coefficient\t\r\nInterarrival time distribution (in hour)\tExponential\r\nLocation parameter (a)\t\r\nScale parameter (b>0) (b=mean if a=0)\t.5\r\n(Not used)\t\r\nArrival discourage coefficient\t\r\nBatch (bulk) size distribution\tConstant\r\nConstant value\t1\r\n(Not used)\t\r\n(Not used)\t\r\nQueue capacity (maximum waiting space)\t3\r\nCustomer population\t\r\nBusy server cost per hour\t100\r\nIdle server cost per hour\t100\r\nCustomer waiting cost per hour\t50\r\nCustomer being served cost per hour\t50\r\nCost of customer being balked\t60\r\nUnit queue capacity cost\t15\r\n\r\n";
    let m = Module::read_file(&Qa, q2.as_bytes()).unwrap();
    assert_eq!(m["servers"], 2);
    assert_eq!(m["queue_capacity"], 3.0);
    assert_eq!(m["service"]["distribution"], "normal");
    assert_eq!(m["interarrival"]["p2"], 0.5);
    let r = Module::solve(&Qa, &m, &Value::Null).unwrap();
    assert_eq!(r.data["model"], "simulación");
    let back = Module::read_file(&Qa, &Module::write_file(&Qa, &m).unwrap()).unwrap();
    assert_eq!(back, m);
}
