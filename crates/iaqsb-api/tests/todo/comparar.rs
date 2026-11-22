//! Comparación de métodos (la misma función para todos los módulos) y paso a
//! paso de las heurísticas del agente viajero.

use iaqsb_api as api;
use serde_json::{json, Value};

fn ex(module: &str, name: &str) -> Value {
    api::examples().into_iter().find(|e| e.0 == module && e.1 == name).unwrap_or_else(|| panic!("no existe {module}/{name}")).2
}
fn compare(module: &str, name: &str, extra: Value) -> Value {
    let mut a = json!({"module": module, "model": ex(module, name)});
    for (k, v) in extra.as_object().unwrap() {
        a[k] = v.clone();
    }
    api::call("compare", a).unwrap_or_else(|e| panic!("{module}/{name}: {e}"))
}
fn values(r: &Value) -> Vec<(String, f64)> {
    r["data"]["variants"].as_array().unwrap().iter().map(|v| (v["label"].as_str().unwrap().to_string(), v["value"].as_f64().unwrap_or(f64::NAN))).collect()
}

#[test]
fn agente_viajero_cuatro_metodos() {
    let r = compare("net", "clase_tsp_bebidas", json!({}));
    let d = &r["data"];
    assert_eq!(d["metric"], "Distancia total");
    assert_eq!(d["smaller_is_better"], true);
    let v = values(&r);
    assert_eq!(v.len(), 4);
    assert!(v.iter().all(|x| (x.1 - 16.0).abs() < 1e-9), "{v:?}");
    assert_eq!(d["tied"].as_array().unwrap().len(), 4);
    assert!(r["headline"].as_str().unwrap().contains("empatan"));
    // solo dos métodos, elegidos por su valor
    let r = compare("net", "clase_tsp_bebidas", json!({"vary": "tsp_method", "values": ["nearest_neighbor", "cheapest_insertion"]}));
    assert_eq!(values(&r).len(), 2);
    // con el informe completo de cada variante
    let r = compare("net", "clase_tsp_bebidas", json!({"full": true}));
    assert!(r["data"]["variants"][1]["report"]["sections"].is_array());
    let sec: Vec<&str> = r["sections"].as_array().unwrap().iter().map(|s| s["id"].as_str().unwrap()).collect();
    assert_eq!(sec.first(), Some(&"comparacion"));
    assert!(sec.contains(&"variantes") && sec.contains(&"fuentes"));
}

#[test]
fn tamano_de_lote_y_pronosticos() {
    let r = compare("its", "tamano_lote", json!({}));
    assert_eq!(r["data"]["metric"], "Costo total");
    let v = values(&r);
    assert_eq!(v.len(), 10);
    let tied: Vec<&str> = r["data"]["tied"].as_array().unwrap().iter().map(|x| x.as_str().unwrap()).collect();
    assert_eq!(tied, ["Wagner-Whitin (óptimo)", "Silver-Meal"]);
    assert!(v.iter().all(|x| x.1 >= 610.0 - 1e-9));
    // en pronósticos se decide por el MAD (menor es mejor)
    let r = compare("fc", "ventas_mensuales", json!({}));
    assert_eq!(r["data"]["metric"], "MAD");
    assert_eq!(r["data"]["best"], "Regresión lineal con el tiempo (LR)");
    let best = values(&r).iter().map(|x| x.1).fold(f64::INFINITY, f64::min);
    assert!((best - 2.289238539).abs() < 1e-6);
    // la métrica se puede elegir
    let r = compare("fc", "ventas_mensuales", json!({"metric": "MSE"}));
    assert_eq!(r["data"]["metric"], "MSE");
}

#[test]
fn variantes_libres_y_errores() {
    // sin métodos que comparar: pide las variantes
    let e = api::call("compare", json!({"module": "fll", "model": ex("fll", "almacen_nuevo")})).unwrap_err().to_string();
    assert!(e.contains("variants"), "{e}");
    // variantes libres con etiqueta
    let r = compare(
        "net",
        "clase_tsp_bebidas",
        json!({"variants": [{"label": "NNH", "tsp_method": "nearest_neighbor"}, {"label": "CIH", "tsp_method": "cheapest_insertion"}]}),
    );
    let v = values(&r);
    assert_eq!(v.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), ["NNH", "CIH"]);
    // qué se puede comparar en un modelo
    let c = api::call("comparable", json!({"module": "net", "model": ex("net", "clase_tsp_bebidas")})).unwrap();
    assert_eq!(c[0]["key"], "tsp_method");
}

fn steps_of(r: &Value) -> Vec<Value> {
    r["sections"].as_array().unwrap().iter().flat_map(|s| s["blocks"].as_array().unwrap().iter()).filter(|b| b["type"] == "steps").flat_map(|b| b["steps"].as_array().unwrap().clone()).collect()
}

#[test]
fn paso_a_paso_nnh_y_cih() {
    let r = api::solve("net", ex("net", "clase_tsp_bebidas"), Some(json!({"tsp_method": "nearest_neighbor", "steps": true}))).unwrap();
    assert_eq!(r["data"]["distance"], json!(16.0));
    let st = steps_of(&r);
    let titles: Vec<&str> = st.iter().map(|s| s["title"].as_str().unwrap()).collect();
    assert_eq!(titles.first(), Some(&"Inicio en 1"));
    assert_eq!(titles.last(), Some(&"Regreso a 1"));
    assert_eq!(titles.iter().filter(|t| t.starts_with("Paso ")).count(), 5);
    // cada iteración trae la tabla de distancias y el recorrido parcial
    for s in st.iter().filter(|s| s["title"].as_str().unwrap().starts_with("Paso ")) {
        let kinds: Vec<&str> = s["blocks"].as_array().unwrap().iter().map(|b| b["type"].as_str().unwrap()).collect();
        assert!(kinds.contains(&"table") && kinds.contains(&"network"), "{kinds:?}");
    }
    assert_eq!(titles[1], "Paso 1: desde 1 a 5");

    let r = api::solve("net", ex("net", "clase_tsp_bebidas"), Some(json!({"tsp_method": "cheapest_insertion", "steps": true}))).unwrap();
    assert_eq!(r["data"]["distance"], json!(16.0));
    let st = steps_of(&r);
    assert_eq!(st[0]["title"], "Subtour inicial");
    assert_eq!(st.last().unwrap()["title"], "Resumen");
    let inserts = st.iter().filter(|s| s["title"].as_str().unwrap().starts_with("Paso ")).count();
    assert_eq!(inserts, 4);
    // el empate del paso 2 queda explicado
    let all = serde_json::to_string(&st).unwrap();
    assert!(all.contains("con el mismo aumento"), "falta la nota del empate");
}
