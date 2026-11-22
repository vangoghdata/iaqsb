//! Motor de optimización por la API: texto LP/MPS, parámetros, diagnóstico,
//! conversión de formatos, motores y modelos de los demás módulos.
use iaqsb_api as api;
use serde_json::json;

const WYNDOR_LP: &str = "Maximize\n obj: 3 x + 5 y\nSubject To\n c1: x <= 4\n c2: 2 y <= 12\n c3: 3 x + 2 y <= 18\nEnd\n";

#[test]
fn resuelve_texto_lp_y_mps() {
    let r = api::solve("lp", json!({"format": "lp", "text": WYNDOR_LP, "name": "Wyndor"}), None).unwrap();
    assert_eq!(r["status"], "optimal");
    assert!((r["objective_value"].as_f64().unwrap() - 36.0).abs() < 1e-6);
    assert!(r["engine"]["solver"].as_str().unwrap().starts_with("HiGHS"));
    // los precios sombra del motor son los del símplex de WinQSB
    let c3 = r["constraints"].as_array().unwrap().iter().find(|c| c["name"] == "c3").unwrap();
    assert!((c3["shadow_price"].as_f64().unwrap() - 1.0).abs() < 1e-6);

    let mps = api::convert_lp(json!({"format": "lp", "text": WYNDOR_LP}), "mps").unwrap();
    let text = mps["content"].as_str().unwrap();
    assert!(text.contains("ROWS"));
    let r2 = api::solve("lp", json!({"format": "mps", "text": text}), None).unwrap();
    assert!((r2["objective_value"].as_f64().unwrap() - 36.0).abs() < 1e-6);
}

#[test]
fn parametros_del_motor_en_opciones() {
    let r = api::solve(
        "lp",
        json!({"format": "lp", "text": "Maximize\n obj: 5 x + 4 y\nSubject To\n c1: 6 x + 4 y <= 24\n c2: x + 2 y <= 6\nGeneral\n x y\nEnd\n"}),
        Some(json!({"time_limit": 10, "mip_gap": 0.0, "threads": 1})),
    )
    .unwrap();
    assert_eq!(r["status"], "optimal");
    assert!((r["objective_value"].as_f64().unwrap() - 20.0).abs() < 1e-6);
    assert_eq!(r["method"], "branch_and_cut");
    assert_eq!(r["engine"]["is_mip"], true);
}

#[test]
fn diagnostico_por_la_api() {
    let bad = "Maximize\n obj: x + y\nSubject To\n c1: x + y <= 4\n c2: x + y >= 6\nEnd\n";
    let d = api::call("diagnose", json!({"model": {"format": "lp", "text": bad}})).unwrap();
    assert_eq!(d["feasible"], false);
    let conflict: Vec<&str> = d["diagnosis"]["conflict"].as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert!(conflict.contains(&"c1") && conflict.contains(&"c2"), "{conflict:?}");
    assert!((d["diagnosis"]["relaxation_total"].as_f64().unwrap() - 2.0).abs() < 1e-6);
    // al resolver, el informe ya trae el diagnóstico
    let r = api::solve("lp", json!({"format": "lp", "text": bad}), None).unwrap();
    assert_eq!(r["status"], "infeasible");
    assert!(r["diagnosis"]["summary"].as_str().unwrap().contains("c2"));
    // y en inglés
    let en = api::call("diagnose", json!({"model": {"format": "lp", "text": bad}, "lang": "en"})).unwrap();
    assert!(en["diagnosis"]["summary"].as_str().unwrap().contains("cannot hold"), "{}", en["diagnosis"]["summary"]);
}

#[test]
fn convierte_entre_formatos() {
    let model = api::example_model("lp").unwrap();
    for to in ["lp", "mps", "lpp"] {
        let v = api::call("convert", json!({"model": model, "to": to})).unwrap();
        let text = v["content"].as_str().unwrap().to_string();
        // cada formato se vuelve a leer y da el mismo óptimo
        let back = if to == "lpp" {
            api::import_bytes(&api::formats::encode_ansi(&text)).unwrap()["model"].clone()
        } else {
            json!({"format": to, "text": text})
        };
        let a = api::solve("lp", model.clone(), None).unwrap();
        let b = api::solve("lp", back, None).unwrap();
        assert!((a["objective_value"].as_f64().unwrap() - b["objective_value"].as_f64().unwrap()).abs() < 1e-6, "{to}");
    }
    let j = api::call("convert", json!({"model": {"format": "lp", "text": WYNDOR_LP}, "to": "json"})).unwrap();
    assert_eq!(j["model"]["variables"].as_array().unwrap().len(), 2);
}

#[test]
fn importa_mps_y_lp_por_contenido() {
    let mps = api::convert_lp(json!({"format": "lp", "text": WYNDOR_LP}), "mps").unwrap();
    for text in [WYNDOR_LP.to_string(), mps["content"].as_str().unwrap().to_string(), format!("\\ comentario\n{WYNDOR_LP}")] {
        let v = api::import_bytes(text.as_bytes()).unwrap();
        assert_eq!(v["module"], "lp");
        assert_eq!(v["model"]["constraints"].as_array().unwrap().len(), 3);
    }
    // un modelo grande no se vuelve hoja: se queda en texto
    let n = 2100;
    let mut big = String::from("Minimize\n obj:");
    for j in 0..n {
        big += &format!(" + x{j}");
    }
    big += "\nSubject To\n";
    for i in 0..n {
        big += &format!(" r{i}: x{i} + x{} >= 1\n", (i + 1) % n);
    }
    big += "End\n";
    let v = api::import_bytes(big.as_bytes()).unwrap();
    assert_eq!(v["large"], true);
    assert_eq!(v["size"]["cols"], n);
    let r = api::solve("lp", v["model"].clone(), None).unwrap();
    assert!((r["objective_value"].as_f64().unwrap() - n as f64 / 2.0).abs() < 1e-6);
}

#[test]
fn motores_y_esquema_de_parametros() {
    let v = api::call("solver_info", json!({})).unwrap();
    let ids: Vec<&str> = v["solvers"].as_array().unwrap().iter().map(|s| s["id"].as_str().unwrap()).collect();
    assert_eq!(ids[0], "highs");
    assert!(v["params_schema"]["properties"]["time_limit"].is_object());
    let err = api::solve("lp", json!({"format": "lp", "text": WYNDOR_LP}), Some(json!({"solver": "nada"}))).unwrap();
    assert_eq!(err["status"], "error");
}

#[test]
fn modulos_genericos_informan_el_motor() {
    // el VRP exacto y la programación por metas usan el motor y lo dicen en el informe
    let (_, _, model) = api::examples().into_iter().find(|(m, n, _)| m == "gp" && n == "harrison").unwrap();
    let r = api::solve("gp", model, Some(json!({"engine": {"time_limit": 30}}))).unwrap();
    assert!(r["engine"]["runs"].as_u64().unwrap() >= 1);
    assert!(r["engine"]["solver"].as_str().unwrap().contains("HiGHS"));
}
