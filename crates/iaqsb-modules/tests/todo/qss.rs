//! QSS: la simulación reproduce M/M/1, M/M/2 y M/M/1/K; ensamble, bloqueo,
//! desechos, disciplinas, validación de la red y archivos de WinQSB.

use iaqsb_core::Module;
use iaqsb_modules::qss::Qss;
use serde_json::{json, Value};

fn ex(name: &str) -> Value {
    Module::examples(&Qss).into_iter().find(|(n, _)| n == name).unwrap().1
}
fn f(v: &Value) -> f64 {
    v.as_f64().unwrap()
}
fn el<'a>(r: &'a iaqsb_core::Report, name: &str) -> &'a Value {
    r.data["elements"].as_array().unwrap().iter().find(|e| e["name"] == name).unwrap()
}

#[test]
fn reproduce_teoria() {
    let r = Module::solve(&Qss, &ex("mm1_comprobacion"), &json!({"simulation_time": 400000, "warmup": 1000, "steps": false})).unwrap();
    assert!((f(&r.data["time_in_system"]) - 20.0).abs() < 1.0, "{}", r.data["time_in_system"]);
    assert!((f(&el(&r, "Servidor")["utilization"]) - 0.8).abs() < 0.01);
    // M/M/2 con λ = 0.5 y μ = 1/3: W = 6.857
    let r = Module::solve(&Qss, &ex("banco_dos_cajeros"), &json!({"simulation_time": 400000, "warmup": 1000, "steps": false})).unwrap();
    assert!((f(&r.data["time_in_system"]) - 6.857).abs() < 0.3, "{}", r.data["time_in_system"]);
    // M/M/1/K con K = 3 (cola de 2): P(rechazo) = ρ³(1−ρ)/(1−ρ⁴) con ρ = 0.8
    let mut m = ex("mm1_comprobacion");
    m["components"][1]["capacity"] = json!(2);
    let r = Module::solve(&Qss, &m, &json!({"simulation_time": 400000, "steps": false})).unwrap();
    let rho: f64 = 0.8;
    let pk = rho.powi(3) * (1.0 - rho) / (1.0 - rho.powi(4));
    let pb = f(&r.data["balked"]) / f(&r.data["arrived"]);
    assert!((pb - pk).abs() < 0.01, "{pb} vs {pk}");
}

#[test]
fn ensamble_bloqueo_y_desechos() {
    let r = Module::solve(&Qss, &ex("ensamble_dos_lineas"), &json!({"simulation_time": 20000, "steps": true})).unwrap();
    // cada producto ensamblado consume un chasis y un motor: terminan ~ la mitad de las llegadas
    let done = f(&r.data["completed"]);
    let arrived = f(&r.data["arrived"]);
    assert!(done > 0.45 * arrived && done <= 0.5 * arrived + 60.0, "{done} de {arrived}");
    let r = Module::solve(&Qss, &ex("taller_en_serie"), &json!({"simulation_time": 200000, "steps": false})).unwrap();
    // 5 % de lo que sale de la fresa va a desechos
    let g = f(&r.data["garbage"]);
    let fresa = f(&el(&r, "Fresa")["served"]);
    assert!((g / fresa - 0.05).abs() < 0.01, "{g} / {fresa}");
    assert!(f(&el(&r, "Cola fresa")["max_length"]) <= 2.0);
    // bloqueo: la cola entre las dos máquinas tiene un lugar y la segunda es más lenta
    let m = json!({"name": "b", "time_unit": "min", "components": [
        {"name": "P", "kind": "customer", "followers": "Q1", "interarrival": "Exp/0/5"},
        {"name": "Q1", "kind": "queue", "followers": "T", "capacity": "M"},
        {"name": "T", "kind": "server", "followers": "Q2", "service": "Constant/4"},
        {"name": "Q2", "kind": "queue", "followers": "F", "capacity": 1},
        {"name": "F", "kind": "server", "service": "Exp/0/4.5"}
    ]});
    let r = Module::solve(&Qss, &m, &json!({"simulation_time": 100000, "steps": false})).unwrap();
    assert!(f(&el(&r, "T")["blocked"]) > 0.1);
    assert!(f(&el(&r, "Q2")["max_length"]) <= 1.0);
    // el tiempo del servidor T se reparte entre ocupado y bloqueado; el flujo lo fija F
    let thr = f(&r.data["completed"]) / 100000.0;
    assert!(thr < 1.0 / 4.5 + 0.005);
}

#[test]
fn maximo_de_observaciones_y_prioridad() {
    let r = Module::solve(&Qss, &ex("banco_dos_cajeros"), &json!({"simulation_time": 1e9, "max_observations": 500, "steps": false})).unwrap();
    assert_eq!(f(&r.data["completed"]), 500.0);
    // con prioridad por atributo la población de mayor atributo espera menos
    let m = json!({"name": "prio", "time_unit": "min", "components": [
        {"name": "Normales", "kind": "customer", "followers": "Cola", "interarrival": "Exp/0/2", "attribute": 0},
        {"name": "Urgentes", "kind": "customer", "followers": "Cola", "interarrival": "Exp/0/4", "attribute": 5},
        {"name": "Cola", "kind": "queue", "followers": "Doctor", "capacity": "M", "discipline": "priority_index"},
        {"name": "Doctor", "kind": "server", "service": "Exp/0/1.2"}
    ]});
    let r = Module::solve(&Qss, &m, &json!({"simulation_time": 100000, "steps": false})).unwrap();
    let c = r.data["customers"].as_array().unwrap();
    assert!(f(&c[1]["wait"]) < 0.5 * f(&c[0]["wait"]), "{:?}", c);
}

#[test]
fn red_invalida() {
    let mut m = ex("taller_en_serie");
    m["components"][4]["followers"] = json!("Cola despacho/0.8, Cola torno/0.1, Desechos/0.05");
    let e = Module::solve(&Qss, &m, &Value::Null).unwrap_err().to_string();
    assert!(e.contains("Fresa") && e.contains("suman"), "{e}");
    let mut m = ex("mm1_comprobacion");
    m["components"][1]["followers"] = json!("Nadie");
    assert!(Module::solve(&Qss, &m, &Value::Null).is_err());
    let mut m = ex("mm1_comprobacion");
    m["components"][2]["service"] = json!("Rápido/3");
    assert!(Module::solve(&Qss, &m, &Value::Null).is_err());
}

#[test]
fn archivos_qss() {
    for name in ["banco_dos_cajeros", "taller_en_serie", "mm1_comprobacion", "ensamble_dos_lineas"] {
        let m = ex(name);
        let out = Module::write_file(&Qss, &m).unwrap();
        let back = Module::read_file(&Qss, &out).unwrap();
        let a = Module::solve(&Qss, &m, &Value::Null).unwrap();
        let b = Module::solve(&Qss, &back, &Value::Null).unwrap();
        assert_eq!(a.headline, b.headline, "{}", iaqsb_core::winqsb::decode(&out));
    }
}

/// Formato real de WinQSB (QSS3 y QSSGRAPH de la distribución original).
#[test]
fn formato_winqsb_qss() {
    let qss3 = "QSS\tQSB P.260\thour\t13\tMatrix\r\nComponent Name\tType (C/S/Q/G)\tImmediate Follower (Name / Prob / TransferTime, separated by ',')\tInput Rule\tOutput Rule\tQueue Discipline\tQueue Capacity\tAttribute Value\tInterarrival Time Distribution\tBatch Size Distribution\tService Time Distribution\r\n\
A\tC\tBUFFER 1\t \t \t \t \t \tEXP/0/2\tCONSTANT/1\t\r\n\
B\tC\tBUFFER 3\t \t \t \t \t \tNORMAL/2/1\tCONSTANT/1\t\r\n\
Station 1\tS\tBUFFER 2//.2\t \t \t \t \t \t \t \tA/EXP//1.8\r\n\
Station 2\tS\tBUFFER 5//.2\t \t \t \t \t \t \t \tA/EXP/1/1\r\n\
Station 3\tS\tBUFFER 4//.1\t \t \t \t \t \t \t \tB/UNIFORM/1.5/2\r\n\
Station 4\tS\tBUFFER 6//.3\t \t \t \t \t \t \t \tB/CONST/1.5\r\n\
Station 5\tS\t \tAssembly\t \t \t \t \t \t \tA/TRI/1/1.5/2,B/TRI/1/1.5/2\r\n\
Buffer 1\tQ\tSTATION 1\t \t \tFIFO\t50\t \t \t \t\r\n\
Buffer 2\tQ\tSTATION 2\t \t \tFIFO\t50\t \t \t \t\r\n\
Buffer 3\tQ\tSTATION 3\t \t \tFIFO\t50\t \t \t \t\r\n\
Buffer 4\tQ\tSTATION 4\t \t \tFIFO\t50\t \t \t \t\r\n\
Buffer 5\tQ\tSTATION 5\t \t \tFIFO\t50\t \t \t \t\r\n\
Buffer 6\tQ\tSTATION 5\t \t \tFIFO\t50\t \t \t \t\r\n0\r\n";
    let m = Module::read_file(&Qss, qss3.as_bytes()).unwrap();
    assert_eq!(m["components"][6]["input_rule"], "assembly");
    let r = Module::solve(&Qss, &m, &json!({"simulation_time": 2000})).unwrap();
    assert!(f(&r.data["completed"]) > 400.0, "{}", r.headline);
    let back = Module::read_file(&Qss, &Module::write_file(&Qss, &m).unwrap()).unwrap();
    let r2 = Module::solve(&Qss, &back, &json!({"simulation_time": 2000})).unwrap();
    assert_eq!(r.headline, r2.headline);
    // modelo gráfico con ubicaciones
    let g = "QSS\tg\thour\t3\tGraphic\r\nComponent Name\tComponent Type\tImmediate Follower\tInput Rule\tOutput Rule\tQueue Discipline\tQueue Capacity\tAttribute Value\tInterarrival Time Distribution\tBatch Size Distribution\tService Time Distribution\r\n\
A\tC\tBuffer 1\t\t\t\t\t\tExp//2\tCons/1\t\r\nBuffer 1\tQ\tStation 1\t\t\t\t50\t\t\t\t\r\nStation 1\tS\t\t\t\t\t\t\t\t\tA/Exp//1.8\r\n-1\r\n1\t1\t1\r\n2\t1\t2\r\n3\t2\t3\r\n";
    let m = Module::read_file(&Qss, g.as_bytes()).unwrap();
    assert_eq!(m["positions"], json!([[1, 1], [1, 2], [2, 3]]));
    assert_eq!(m["graphic"], true);
    let back = Module::read_file(&Qss, &Module::write_file(&Qss, &m).unwrap()).unwrap();
    assert_eq!(back, m);
}
