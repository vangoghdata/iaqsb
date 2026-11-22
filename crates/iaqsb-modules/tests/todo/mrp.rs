//! MRP: el producto A de Heizer y Render, lotes, ciclos y archivos.

use iaqsb_core::Module;
use iaqsb_modules::mrp::Mrp;
use serde_json::{json, Value};

fn ex(name: &str) -> Value {
    Module::examples(&Mrp).into_iter().find(|(n, _)| n == name).unwrap().1
}

#[test]
fn producto_a_render() {
    let r = Module::solve(&Mrp, &ex("producto_a_render"), &Value::Null).unwrap();
    let got: Vec<(String, String, f64)> = r.data["orders"].as_array().unwrap().iter().map(|o| (o["item"].as_str().unwrap().into(), o["period"].as_str().unwrap().into(), o["quantity"].as_f64().unwrap())).collect();
    let want = [("A", "7", 40.0), ("B", "5", 65.0), ("C", "6", 100.0), ("D", "4", 120.0), ("E", "3", 120.0), ("E", "4", 100.0), ("F", "3", 195.0)];
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(want.iter()) {
        assert_eq!((g.0.as_str(), g.1.as_str(), g.2), (w.0, w.1, w.2));
    }
}

#[test]
fn lotes_y_ciclos() {
    let mut m = ex("producto_a_render");
    m["items"][5]["components"] = json!("A");
    let e = Module::solve(&Mrp, &m, &Value::Null).unwrap_err().to_string();
    assert!(e.contains("ciclo"), "{e}");
    // FOQ de 100 para D: la recepción planeada es un múltiplo de 100
    let mut m = ex("producto_a_render");
    m["items"][3]["lot_size"] = json!("FOQ");
    m["items"][3]["ls_multiplier"] = json!(100);
    let r = Module::solve(&Mrp, &m, &Value::Null).unwrap();
    let d = r.data["records"].as_array().unwrap().iter().find(|x| x["item"] == "D").unwrap();
    let total: f64 = d["planned_receipts"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).sum();
    assert_eq!(total, 200.0);
    let r = Module::solve(&Mrp, &ex("bicicletas_lotes"), &Value::Null).unwrap();
    assert!(r.data["orders"].as_array().unwrap().len() > 5);
}

#[test]
fn archivos_mrp() {
    for name in ["producto_a_render", "bicicletas_lotes"] {
        let m = ex(name);
        let out = Module::write_file(&Mrp, &m).unwrap();
        let back = Module::read_file(&Mrp, &out).unwrap();
        let a = Module::solve(&Mrp, &m, &Value::Null).unwrap();
        let b = Module::solve(&Mrp, &back, &Value::Null).unwrap();
        assert_eq!(a.data["orders"], b.data["orders"], "{}", iaqsb_core::winqsb::decode(&out));
    }
}

#[test]
fn siete_articulos_igual_a_la_hoja_de_arango() {
    // «solución ejemplo programacion mrp.xlsx»: columna G (lanzamiento de órdenes) de cada artículo
    let r = Module::solve(&Mrp, &ex("siete_articulos_arango"), &Value::Null).unwrap();
    let rel = |id: &str| -> Vec<f64> {
        let rec = r.data["records"].as_array().unwrap().iter().find(|x| x["item"] == id).unwrap();
        rec["planned_releases"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect()
    };
    assert_eq!(rel("A"), [5., 0., 0., 30., 0., 30., 30., 30., 0., 30., 0., 30., 0.]);
    assert_eq!(rel("B"), [0., 0., 0., 0., 0., 50., 0., 60., 0., 60., 0., 0., 0.]);
    assert_eq!(rel("C"), [0., 0., 0., 75., 90., 90., 0., 90., 0., 90., 0., 0., 0.]);
    assert_eq!(rel("D"), [0., 0., 0., 0., 0., 0., 130., 0., 240., 0., 0., 0., 0.]);
    assert_eq!(rel("E"), [0., 0., 5., 270., 0., 270., 0., 270., 0., 0., 0., 0., 0.]);
    assert_eq!(rel("F").iter().sum::<f64>(), 0.0);
    assert_eq!(rel("G"), [0., 0., 0., 0., 0., 0., 85., 0., 120., 0., 0., 0., 0.]);
    let g = r.data["records"].as_array().unwrap().iter().find(|x| x["item"] == "G").unwrap();
    let disp: Vec<f64> = g["projected_on_hand"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
    assert_eq!(disp, [500., 980., 975., 705., 705., 335., 335., 30., 30., 30., 30., 30., 30.]);
}
