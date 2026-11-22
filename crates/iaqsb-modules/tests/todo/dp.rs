//! DP: diligencia de Hillier, mochila, producción, «¿qué pasa si?» y archivos.

use iaqsb_core::Module;
use iaqsb_modules::dp::Dp;
use serde_json::{json, Value};

fn ex(name: &str) -> Value {
    Module::examples(&Dp).into_iter().find(|(n, _)| n == name).unwrap().1
}

#[test]
fn diligencia_hillier() {
    let r = Module::solve(&Dp, &ex("diligencia"), &Value::Null).unwrap();
    assert_eq!(r.data["distance"], 11.0);
    let mut routes: Vec<String> = r.data["routes"].as_array().unwrap().iter().map(|p| p.as_array().unwrap().iter().map(|x| x.as_str().unwrap()).collect::<String>()).collect();
    routes.sort();
    assert_eq!(routes, vec!["ACEHJ", "ADEHJ", "ADFIJ"]);
    // desde otro origen
    let r = Module::solve(&Dp, &ex("diligencia"), &json!({"origin": "B"})).unwrap();
    assert_eq!(r.data["distance"], 11.0);
}

#[test]
fn mochila_y_produccion() {
    let r = Module::solve(&Dp, &ex("carga_avion"), &Value::Null).unwrap();
    assert_eq!(r.data["total_return"], 62.0);
    let r = Module::solve(&Dp, &ex("carga_avion"), &json!({"preset": "0"})).unwrap();
    assert_eq!(r.data["total_return"], 61.0);
    let r = Module::solve(&Dp, &ex("produccion"), &Value::Null).unwrap();
    assert_eq!(r.data["total_cost"], 21.0);
    let r = Module::solve(&Dp, &ex("produccion"), &json!({"preset": "5"})).unwrap();
    assert_eq!(r.data["total_cost"], 21.5);
}

#[test]
fn archivos_dp() {
    for name in ["diligencia", "carga_avion", "produccion"] {
        let m = ex(name);
        let out = Module::write_file(&Dp, &m).unwrap();
        let back = Module::read_file(&Dp, &out).unwrap();
        let a = Module::solve(&Dp, &m, &Value::Null).unwrap();
        let b = Module::solve(&Dp, &back, &Value::Null).unwrap();
        assert_eq!(a.headline, b.headline, "{}", iaqsb_core::winqsb::decode(&out));
    }
    // como lo escribe WinQSB (mochila)
    let txt = "DP\tEjemplo\tKS\t2\r\nStage\tItem\tAvailable\tCapacity Required\tReturn Function\r\n1\tA\t3\t2\t10X\r\n2\tB\t \t3\t16X\r\n \tKnapsack Capacity =\t7\t \t \r\n";
    let m = Module::read_file(&Dp, txt.as_bytes()).unwrap();
    assert_eq!(m["capacity"], 7.0);
    assert_eq!(m["items"][1]["available"], Value::Null);
    let r = Module::solve(&Dp, &m, &Value::Null).unwrap();
    // 2A + 1B = 36 (2·2 + 3 = 7)
    assert_eq!(r.data["total_return"], 36.0);
}
