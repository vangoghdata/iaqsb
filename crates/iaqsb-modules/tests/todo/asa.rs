//! ASA: probabilidades de aceptación contra fórmulas cerradas, Wald (Montgomery),
//! búsqueda de planes, proceso secuencial y archivos de WinQSB.

use iaqsb_core::Module;
use iaqsb_modules::asa::Asa;
use serde_json::{json, Value};

fn ex(name: &str) -> Value {
    Module::examples(&Asa).into_iter().find(|(n, _)| n == name).unwrap().1
}
fn f(v: &Value) -> f64 {
    v.as_f64().unwrap()
}
fn binom_cdf(c: u32, n: u32, p: f64) -> f64 {
    let mut s = 0.0;
    let mut t = (1.0 - p).powi(n as i32);
    for x in 0..=c {
        s += t;
        t *= (n - x) as f64 / (x + 1) as f64 * p / (1.0 - p);
    }
    s
}

#[test]
fn simple_binomial() {
    let r = Module::solve(&Asa, &ex("muestreo_simple"), &Value::Null).unwrap();
    let pa = binom_cdf(1, 50, 0.01);
    assert!((f(&r.data["pa_aql"]) - pa).abs() < 1e-4);
    assert!((f(&r.data["beta"]) - 100.0 * binom_cdf(1, 50, 0.08)).abs() < 1e-2);
    assert!(f(&r.data["aoql"]) > 0.0 && r.data["cost_aql"].is_number());
}

#[test]
fn wald_montgomery() {
    let r = Module::solve(&Asa, &ex("secuencial_atributos"), &json!({"observations": "0,0,1,0,0,0,0,1,0,1,1"})).unwrap();
    // la OC de Wald pasa por (p1, 1 − α) y (p2, β)
    assert!((f(&r.data["alpha"]) - 5.0).abs() < 1e-3 && (f(&r.data["beta"]) - 10.0).abs() < 1e-3);
    // con 4 defectuosos en 11 unidades se rechaza (h2 + s·n = 1.57 + 0.0281·n)
    assert!(r.data["sequential"]["decision"].as_str().unwrap().starts_with("rechazar"), "{}", r.data["sequential"]);
}

#[test]
fn busqueda_simple_y_variables() {
    let m = json!({"name": "s", "problem_type": "single", "stages": [{"n": 10, "c": 0}], "aql": 1, "rql": 8, "alpha": 5, "beta": 10});
    let r = Module::solve(&Asa, &m, &json!({"mode": "search"})).unwrap();
    let (n, c) = (f(&r.data["plan"][0]["n"]) as u32, f(&r.data["plan"][0]["c"]) as u32);
    assert!(binom_cdf(c, n, 0.08) <= 0.10 && binom_cdf(c, n - 1, 0.08) > 0.10 && binom_cdf(c, n, 0.01) >= 0.95, "n = {n}, c = {c}");
    // método k: k = (z1·zβ + z2·zα)/(zα + zβ)
    let r = Module::solve(&Asa, &ex("metodo_k"), &json!({"mode": "search"})).unwrap();
    assert!((f(&r.data["k"]) - 1.8927).abs() < 1e-3, "{}", r.data["k"]);
    assert_eq!(f(&r.data["n"]), 15.0);
}

#[test]
fn csp1_y_cadena() {
    let r = Module::solve(&Asa, &ex("continuo_csp1"), &Value::Null).unwrap();
    // AOQL de CSP-1 (fórmulas de Dodge) con f = 1/50 e i = 38: 5.392 % (calculado aparte)
    let a = f(&r.data["aoql"]);
    assert!((a - 5.392).abs() < 0.005, "{a}");
    let m = json!({"name": "c", "problem_type": "chain", "stages": [{"n": 20, "c": 0}], "clearance": 2, "aql": 1, "rql": 8, "lot_size": 1000});
    let r = Module::solve(&Asa, &m, &Value::Null).unwrap();
    let p0: f64 = 0.99f64.powi(20);
    let p1 = 20.0 * 0.01 * 0.99f64.powi(19);
    assert!((f(&r.data["pa_aql"]) - (p0 + p1 * p0 * p0)).abs() < 1e-4);
}

#[test]
fn archivos_asa() {
    for (name, _) in Module::examples(&Asa) {
        let m = ex(&name);
        let out = Module::write_file(&Asa, &m).unwrap();
        let back = Module::read_file(&Asa, &out).unwrap();
        let a = Module::solve(&Asa, &m, &Value::Null).unwrap();
        let b = Module::solve(&Asa, &back, &Value::Null).unwrap();
        assert_eq!(a.headline, b.headline, "{name}\n{}", iaqsb_core::winqsb::decode(&out));
    }
    // ejemplo original ASA2.AS (muestreo doble)
    let asa2 = "ASA\tAcceptance Sampling Problem\t0\t1\r\n0\t0\t2\t0\t0\r\n                    DATA  ITEM\tENTRY\r\nFirst sample size (n1)\t40\r\nFirst acceptance number (c1)\t1\r\nFirst rejection number (r1)\t5\r\nSecond sample size (n2)\t80\r\nSecond acceptance number (c2)\t5\r\nAcceptable quality level (AQL, in % defective)\t1\r\nRejectable quality level (RQL, LQL, or LTPD, in % defective)\t6\r\nProducer's risk level (alpha, type I error in %)\t5\r\nConsumer's risk level (beta, type II error in %)\t10\r\nLot size (N)\t3000\r\nProbability distribution\tBinomial\r\nUnit sampling cost\t5\r\nUnit inspection cost\t5\r\nUnit producer's cost if unit is found defective\t10\r\nUnit consumer's cost if unit is found defective\t50\r\nInspection error: probability (%) of good item is classified as bad\t\r\nInspection error: probability (%) of bad item is classified as good\t\r\n";
    let m = Module::read_file(&Asa, asa2.as_bytes()).unwrap();
    assert_eq!(m["stages"], json!([{"n": 40.0, "c": 1.0, "r": 5.0}, {"n": 80.0, "c": 5.0, "r": null}]));
    assert_eq!(m["rectified"], false);
    let r = Module::solve(&Asa, &m, &Value::Null).unwrap();
    assert!(f(&r.data["pa_aql"]) > 0.99);
}
