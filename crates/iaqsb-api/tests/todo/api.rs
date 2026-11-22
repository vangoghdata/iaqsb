use iaqsb_api as api;
use serde_json::json;

#[test]
fn ejemplos_resuelven() {
    for (m, name, model) in api::examples() {
        let r = api::solve(&m, model, None).unwrap_or_else(|e| panic!("{m}/{name}: {e}"));
        let st = r["status"].as_str().unwrap();
        assert!(st == "optimal" || st == "feasible" || st == "solved", "{m}/{name}: {st}");
    }
}

#[test]
fn todos_los_ejemplos_ida_y_vuelta_por_archivo() {
    for (m, name, model) in api::examples() {
        let txt = api::call("export_text", json!({"module": m, "model": model})).unwrap();
        let back = api::call("import_text", json!({"content": txt["content"]})).unwrap_or_else(|e| panic!("{m}/{name}: {e}"));
        assert_eq!(back["module"], m.as_str(), "{name}");
    }
}

#[test]
fn exportar_e_importar_por_la_api() {
    let model = api::example_model("lp").unwrap();
    for layout in ["matrix", "normal"] {
        let txt = api::call(
            "export_text",
            json!({"module": "lp", "model": model, "layout": layout}),
        )
        .unwrap();
        let back = api::call("import_text", json!({"content": txt["content"]})).unwrap();
        assert_eq!(back["module"], "lp");
        assert_eq!(back["layout"], layout);
        let a = api::solve("lp", model.clone(), None).unwrap();
        let b = api::solve("lp", back["model"].clone(), None).unwrap();
        assert_eq!(a["objective_value"], b["objective_value"]);
    }
}

#[test]
fn nombres_de_modulo_de_winqsb() {
    assert!(api::describe_module("LP-ILP").is_ok());
    // los 19 módulos de WinQSB y los 9 que agrega IAQSB (6 de logística, el programa maestro, el MRP II por matrices y las series de tiempo) están disponibles
    let list = api::list_modules();
    let mods = list["modules"].as_array().unwrap();
    assert_eq!(mods.len(), 28);
    assert_eq!(mods.iter().filter(|m| m["winqsb"] != "Nuevo").count(), 19);
    assert!(mods.iter().all(|m| m["available"] == true), "{list}");
    assert!(api::describe_module("ASA").is_ok() && api::describe_module("QCC").is_ok());
    assert_eq!(
        api::solve("xyz", json!({}), None).unwrap_err().code(),
        "unknown_module"
    );
}

#[test]
fn error_claro_con_modelo_mal_formado() {
    let e = api::solve("lp", json!({"variables": [{"name": "X", "cost": 1}], "constraints": [{"name": "C", "coefs": [1, 2], "direction": "<=", "rhs": 3}]}), None)
        .unwrap_err();
    assert!(e.to_string().contains("coeficientes"), "{e}");
}

#[test]
fn cadena_jerarquica_ap_pmp_mrp() {
    let ex = |m: &str, n: &str| api::examples().into_iter().find(|e| e.0 == m && e.1 == n).unwrap().2;
    // plan agregado → dos programas maestros (60 % y 40 % de la producción agregada)
    let r = api::call("link", json!({"from": "ap", "to": "pmp", "model": ex("ap", "planificacion_simple"), "products": [{"name": "U", "share": 60, "lot_size": 500}, {"name": "M", "share": 40}]})).unwrap();
    let ms = r["models"].as_array().unwrap();
    assert_eq!(ms.len(), 2);
    let plan = api::solve("ap", ex("ap", "planificacion_simple"), None).unwrap();
    for (t, p) in plan["data"]["periods"].as_array().unwrap().iter().enumerate() {
        let total = ms[0]["model"]["periods"][t]["forecast"].as_f64().unwrap() + ms[1]["model"]["periods"][t]["forecast"].as_f64().unwrap();
        assert!((total - p["production"].as_f64().unwrap()).abs() <= 1.0, "periodo {t}");
    }
    // programa maestro → MRP: el PMP pasa a ser el programa maestro del artículo A
    let pmp = ms[0]["model"].clone();
    let r = api::call("link", json!({"from": "pmp", "to": "mrp", "model": pmp, "target": ex("mrp", "producto_a_render"), "item": "A"})).unwrap();
    let mrp = r["models"][0]["model"].clone();
    let lotes = api::solve("pmp", pmp, None).unwrap();
    let esperado: Vec<f64> = lotes["data"]["periods"].as_array().unwrap().iter().map(|p| p["mps"].as_f64().unwrap()).collect();
    let got: Vec<f64> = mrp["mps"][0].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
    assert_eq!(got, esperado);
    assert_eq!(mrp["periods"].as_array().unwrap().len(), esperado.len());
    assert!(api::solve("mrp", mrp, None).is_ok());
    assert!(api::call("link", json!({"from": "mrp", "to": "ap", "model": {}})).is_err());
}
