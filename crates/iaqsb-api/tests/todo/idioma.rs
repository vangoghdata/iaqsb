//! La app en inglés: los informes de todos los ejemplos salen en inglés, con
//! los mismos números, y los errores también se traducen.

use iaqsb_api as api;
use serde_json::{json, Value};

fn numbers(s: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    for ch in s.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_digit() || (ch == '.' && !cur.is_empty()) {
            cur.push(ch);
        } else {
            if !cur.is_empty() {
                out.push(cur.trim_end_matches('.').to_string());
            }
            cur.clear();
        }
    }
    out.sort();
    out
}

#[test]
fn los_informes_salen_en_ingles_con_los_mismos_numeros() {
    const ES: &[&str] = &[" de ", " en ", " por ", " con ", " para ", " del ", " los ", " las ", "Costo", "Pedir", "Máximo", "Mínimo", "ruta crítica"];
    let ex_en = api::call("examples", json!({"lang": "en"})).unwrap();
    for (m, n, model) in api::examples() {
        let es = api::solve(&m, model, None).unwrap();
        let model_en = ex_en.as_array().unwrap().iter().find(|e| e["module"] == m.as_str() && e["name"] == n.as_str()).unwrap()["model"].clone();
        let en = api::call("solve", json!({"module": m, "model": model_en, "lang": "en"})).unwrap();
        let (h_es, h_en) = (es["headline"].as_str().unwrap(), en["headline"].as_str().unwrap());
        assert_eq!(numbers(h_es), numbers(h_en), "{m}/{n}: {h_es:?} → {h_en:?}");
        for w in ES {
            assert!(!h_en.contains(w), "{m}/{n}: queda «{w}» en {h_en:?}");
        }
        // el ejemplo traducido se sigue reconociendo (fuente de los datos)
        assert_eq!(en["data_source"]["key"], json!(format!("{m}/{n}")), "{m}/{n}");
        let sec: &Value = en["sections"].as_array().unwrap().iter().find(|s| s["id"] == "fuentes").unwrap();
        assert_eq!(sec["title"], "Sources");
    }
}

#[test]
fn apa_y_errores_en_ingles() {
    let b = api::call("bibliography", json!({"lang": "en"})).unwrap();
    let h = b.as_array().unwrap().iter().find(|r| r["id"] == "hillier2015en").unwrap();
    assert!(h["apa"].as_str().unwrap().starts_with("Hillier, F. S., & Lieberman, G. J. (2015a)."), "{}", h["apa"]);
    assert!(h["apa"].as_str().unwrap().contains("(10th ed.)"));
    assert_eq!(h["cite"], "Hillier & Lieberman, 2015a");
    let e = api::call("solve", json!({"module": "nada", "model": {}, "lang": "en"})).unwrap_err();
    assert!(e.to_string().starts_with("unknown module"), "{e}");
    let sp = api::call("spec", json!({"module": "its", "lang": "en"})).unwrap();
    assert_eq!(sp["name"], "Inventory");
}
