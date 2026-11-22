//! Lenguaje algebraico de punta a punta (por la API): casos que antes fallaban.

use iaqsb_api::call;
use serde_json::{json, Value};

fn resolver(texto: &str) -> Value {
    call("solve", json!({ "module": "lp", "model": { "format": "iaqsb", "text": texto } })).unwrap_or_else(|e| panic!("{e}: {texto}"))
}

fn z(v: &Value) -> f64 {
    v["objective_value"].as_f64().unwrap()
}

fn valor(v: &Value, nombre: &str) -> f64 {
    v["variables"].as_array().unwrap().iter().find(|x| x["name"] == nombre).unwrap()["value"].as_f64().unwrap()
}

#[test]
fn disyuncion_con_o_despues_de_un_numero() {
    // «3 o x» es el conector, no 3·o
    let r = resolver("var x >= 0 <= 10; max z: x; d1: x <= 3 o x >= 8;");
    assert!((z(&r) - 10.0).abs() < 1e-6);
    let r = resolver("var x >= 0 <= 10; min z: x; d1: x >= 3 o x <= -1; ");
    assert!((z(&r) - 3.0).abs() < 1e-6);
    // y una letra que sí es variable sigue multiplicando: 3 y + 2
    let r = resolver("var y >= 0; max z: y; e1: 3 y + 2 <= 20;");
    assert!((z(&r) - 6.0).abs() < 1e-6);
}

#[test]
fn cuadratico_no_convexo_con_busqueda_global() {
    // máx x·y con x + y <= 4: el óptimo es 4 en (2, 2), no 0
    let r = resolver("var x >= 0 <= 5; var y >= 0 <= 5; maximize f: x*y; c: x + y <= 4;");
    assert_eq!(r["status"], "optimal");
    assert!((z(&r) - 4.0).abs() < 1e-6, "{}", z(&r));
    assert!((valor(&r, "x") - 2.0).abs() < 1e-4);
    // mín de una cóncava: en un vértice
    let r = resolver("var x >= 0 <= 5; var y >= 0 <= 5; minimize f: -(x-1)^2 - (y-2)^2; c: x + y <= 2;");
    assert!((z(&r) + 5.0).abs() < 1e-6, "{}", z(&r));
}

#[test]
fn cuadratico_con_enteras() {
    let r = resolver("var x >= 0 <= 5 integer; var y >= 0; minimize f: (x - 2.4)^2 + (y - 1)^2; c: x + y <= 3;");
    assert_eq!(r["status"], "optimal");
    assert!((z(&r) - 0.16).abs() < 1e-6, "{}", z(&r));
    assert!((valor(&r, "x") - 2.0).abs() < 1e-9 && (valor(&r, "y") - 1.0).abs() < 1e-6);
}

#[test]
fn cuadratico_convexo_sigue_en_el_motor() {
    let r = resolver("var x >= 0; var y >= 0; minimize f: (x - 1)^2 + (y - 2)^2; c: x + y <= 2;");
    assert!((z(&r) - 0.5).abs() < 1e-6);
    assert!(r["method"].as_str().unwrap_or("").find("QP-IQP").is_none());
}

#[test]
fn varios_objetivos_conservan_el_nombre() {
    let r = resolver("var x >= 0; var y >= 0; maximize ventas: 3x + 2y; maximize margen: x + y; c: x + y <= 4; d: x <= 3;");
    let s = r.to_string();
    assert!(s.contains("ventas") && s.contains("margen"), "{s}");
}

#[test]
fn hoja_con_palabras_reservadas_pasa_al_lenguaje() {
    let hoja = json!({
        "name": "reservadas", "objective": "max",
        "variables": [{"name": "Suma", "cost": 1}, {"name": "en", "cost": 2}, {"name": "o", "cost": 1}, {"name": "x 1", "cost": 1}, {"name": "x_1", "cost": 1}],
        "constraints": [{"name": "max", "coefs": [1, 1, 1, 1, 1], "direction": "<=", "rhs": 4}, {"name": "Suma", "coefs": [0, 1, 0, 0, 0], "direction": "<=", "rhs": 1}]
    });
    let txt = call("convert", json!({ "model": hoja, "to": "iaqsb" })).unwrap()["content"].as_str().unwrap().to_string();
    let r = resolver(&txt);
    assert!((z(&r) - 5.0).abs() < 1e-6, "{txt}");
}

#[test]
fn hoja_con_nombres_raros_y_saltos_de_linea() {
    let hoja = json!({
        "name": "Prueba\nvar evil >= 0;", "objective": "max",
        "variables": [{"name": "²a", "cost": 1}, {"name": "b", "cost": 1}],
        "constraints": [{"name": "c", "coefs": [1, 1], "direction": "<=", "rhs": 3}]
    });
    let txt = call("convert", json!({ "model": hoja, "to": "iaqsb" })).unwrap()["content"].as_str().unwrap().to_string();
    let r = resolver(&txt);
    assert!((z(&r) - 3.0).abs() < 1e-6, "{txt}");
    assert!(!r.to_string().contains("evil"), "{txt}");
}

#[test]
fn cuadratico_no_convexo_grande_es_optimo_local() {
    // 15 variables: no hay enumeración exhaustiva; se dice que es un óptimo local y no «óptimo»
    let r = resolver("set I = 1..15; param c{I} = [3,7,0,4,8,1,5,9,2,6,10,3,7,0,4]; var x{I} >= 0 <= 10; maximizar z: suma{i en I} x[i]*x[i] - suma{i en I} c[i]*x[i]; r: suma{i en I} x[i] <= 100;");
    assert_eq!(r["status"], "feasible");
    assert!(z(&r) >= 500.0, "{}", z(&r));
}

#[test]
fn cuadratico_grande_y_disperso_no_arma_matrices_densas() {
    // 5000 variables y un solo término cuadrático (convexo): va directo al motor
    let r = resolver("set I = 1..5000; var x{I} >= 0 <= 1; minimizar z: x[1]*x[1] - suma{i en I} x[i]; r: suma{i en I} x[i] <= 10;");
    assert_eq!(r["status"], "optimal");
}
