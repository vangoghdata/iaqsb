//! NLP: ejemplos de libro, archivos .NLP de WinQSB, análisis de un punto y límites.

use iaqsb_core::Module;
use iaqsb_modules::nlp::NlpMod;
use serde_json::{json, Value};

fn ex(name: &str) -> Value {
    Module::examples(&NlpMod).into_iter().find(|(n, _)| n == name).unwrap().1
}

fn xs(r: &iaqsb_core::Report) -> Vec<f64> {
    r.data["variables"].as_array().unwrap().iter().map(|v| v["value"].as_f64().unwrap()).collect()
}

#[test]
fn ejemplos() {
    // Hillier: máx 126x1 − 9x1² + 182x2 − 13x2² → (8/3, 5), Z = 857 (convexo: óptimo global)
    let r = Module::solve(&NlpMod, &ex("hillier_nlp"), &Value::Null).unwrap();
    let x = xs(&r);
    assert!((x[0] - 8.0 / 3.0).abs() < 1e-9 && (x[1] - 5.0).abs() < 1e-9, "{x:?}");
    assert!((r.data["objective_value"].as_f64().unwrap() - 857.0).abs() < 1e-9);
    assert_eq!(r.status, iaqsb_core::Status::Optimal);
    assert_eq!(r.data["constraints"][2]["multiplier"], 26.0);
    // Cobb-Douglas: K = 15, L = 30
    let r = Module::solve(&NlpMod, &ex("cobb_douglas"), &Value::Null).unwrap();
    assert_eq!(xs(&r), vec![15.0, 30.0]);
    // Rosenbrock
    let r = Module::solve(&NlpMod, &ex("rosenbrock"), &Value::Null).unwrap();
    assert_eq!(xs(&r), vec![1.0, 1.0]);
    let graf = r.sections.iter().find(|s| s.id == "grafico").unwrap();
    assert!(serde_json::to_string(&graf.blocks).unwrap().contains("Curvas de nivel"));
}

#[test]
fn archivo_de_winqsb() {
    // Como lo guarda WinQSB: números con Str$ (espacio delante), hoja de dos columnas
    let txt = "NLP\tPrueba\t 2\t 1\r\n\tOBJ / Constraint / Variable Bound\r\nMinimize\t(X-1)^2 + (Y-2)^2 + LOG(EXP(1))\r\nC1\tX + Y <= 2\r\nX\t>=0, <=M\r\nY\t>=-M, <=10\r\n";
    let m = Module::read_file(&NlpMod, txt.as_bytes()).unwrap();
    assert_eq!(m["variables"][1]["lower"], "-M");
    assert_eq!(m["variables"][1]["upper"], 10.0);
    let r = Module::solve(&NlpMod, &m, &Value::Null).unwrap();
    assert_eq!(xs(&r), vec![0.5, 1.5]);
    // LOG es el logaritmo natural, como en WinQSB: f = 0.5 + 1
    assert!((r.data["objective_value"].as_f64().unwrap() - 1.5).abs() < 1e-9);
    let out = Module::write_file(&NlpMod, &m).unwrap();
    let text = iaqsb_core::winqsb::decode(&out);
    assert!(text.starts_with("NLP\tPrueba\t 2\t 1\r\n\tOBJ / Constraint / Variable Bound\r\n"), "{text}");
    assert_eq!(Module::read_file(&NlpMod, &out).unwrap(), m);
}

#[test]
fn analisis_de_un_punto_y_errores() {
    let r = Module::solve(&NlpMod, &ex("hillier_nlp"), &json!({"evaluate": "X1 = 4, X2 = 6", "steps": true})).unwrap();
    let an = r.sections.iter().find(|s| s.id == "analisis").unwrap();
    let t = serde_json::to_string(&an.blocks).unwrap();
    assert!(t.contains("violada"), "{t}");
    let pasos = serde_json::to_string(&r.sections.iter().find(|s| s.id == "pasos").unwrap().blocks).unwrap();
    assert!(pasos.contains("Método de penalización") && pasos.contains("Karush-Kuhn-Tucker"));
    let bad = json!({"name": "b", "function": "X + Z", "variables": [{"name": "X"}]});
    let e = Module::solve(&NlpMod, &bad, &Value::Null).unwrap_err().to_string();
    assert!(e.contains("Z") && e.contains("añádela"), "{e}");
    let unb = json!({"name": "u", "objective": "max", "function": "X^2", "variables": [{"name": "X"}]});
    assert_eq!(Module::solve(&NlpMod, &unb, &Value::Null).unwrap().status, iaqsb_core::Status::Unbounded);
}
