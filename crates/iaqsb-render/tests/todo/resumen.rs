//! El resumen en Markdown para la IA es corto, trae lo esencial y deja lo
//! grande a pedido.

use iaqsb_api as api;
use iaqsb_render::digest::{self, Detail, DigestOpts};
use serde_json::json;

fn tsp(method: &str, steps: bool) -> serde_json::Value {
    let model = api::examples().into_iter().find(|e| e.1 == "clase_tsp_bebidas").unwrap().2;
    api::solve("net", model, Some(json!({"tsp_method": method, "steps": steps}))).unwrap()
}

#[test]
fn resumen_corto_con_lo_esencial() {
    let r = tsp("cheapest_insertion", true);
    let o = DigestOpts { report_id: Some("r1".into()), ..Default::default() };
    let md = digest::markdown(&r, &o);
    assert!(md.len() < 6000, "resumen de {} caracteres", md.len());
    assert!(md.contains("Distancia total: 16"));
    assert!(md.contains("Arcos destacados"));
    assert!(md.contains("report_id: r1"));
    // el paso a paso y el modelo lineal quedan a pedido
    assert!(md.contains("pasos sin mostrar"));
    assert!(md.contains("path=data.lp_model"));
    // con el paso a paso
    let full = digest::markdown(&r, &DigestOpts { detail: Detail::Steps, ..o.clone() });
    assert!(full.contains("Costo de insertar"));
    assert!(full.len() > md.len());
}

#[test]
fn partes_del_informe() {
    let r = tsp("nearest_neighbor", true);
    let p = digest::part(&r, None, Some("2"), None).unwrap();
    assert_eq!(p["type"], "step");
    assert!(digest::part_markdown(&p, 50).contains("el más cercano"));
    let p = digest::part(&r, Some("modelo"), None, None).unwrap();
    assert!(digest::part_markdown(&p, 200).contains("Relajación"));
    let t = digest::part(&r, None, None, Some("data.tour")).unwrap();
    assert_eq!(t[0], "1");
    assert!(digest::part(&r, Some("nada"), None, None).is_err());
}

#[test]
fn programa_lineal_solo_lo_distinto_de_cero() {
    let r = tsp("exact", false);
    let lp = api::solve("lp", r["data"]["lp_model"].clone(), None).unwrap();
    let md = digest::markdown(&lp, &DigestOpts::default());
    assert!(md.contains("Z = 16"), "{md}");
    assert!(md.contains("distintas de cero"));
    assert!(md.len() < 5000);
}

#[test]
fn compactar_datos() {
    let v = json!({"a": 1, "big": (0..2000).collect::<Vec<i32>>(), "obj": {"x": "y".repeat(3000)}});
    let (c, om) = digest::compact(&v, 500, "data");
    assert_eq!(c["a"], 1);
    assert!(om.contains(&"data.big".to_string()));
    assert!(serde_json::to_string(&c).unwrap().len() < 1500);
}
