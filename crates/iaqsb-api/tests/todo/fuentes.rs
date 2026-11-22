//! Fuentes: cada ejemplo dice de dónde salen sus datos, cada informe cita sus
//! métodos y los resultados publicados se reproducen.

use iaqsb_api as api;
use iaqsb_api::core::biblio;
use serde_json::{json, Value};

fn numbers(s: &str) -> Vec<f64> {
    let mut out = vec![];
    let mut cur = String::new();
    for ch in s.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_digit() || (ch == '.' && !cur.is_empty()) {
            cur.push(ch);
        } else {
            if let Ok(x) = cur.trim_end_matches('.').parse::<f64>() {
                out.push(x);
            }
            cur.clear();
        }
    }
    out
}

#[test]
fn cada_ejemplo_tiene_fuente_y_cita_sus_metodos() {
    let ex = api::examples();
    for (m, n, model) in &ex {
        let src = biblio::data_source(m, n);
        assert!(src.is_some(), "falta la fuente del ejemplo {m}/{n} en fuentes/ejemplos.json");
        let o = &biblio::example_origins()[&format!("{m}/{n}")];
        assert!(["libro", "clase", "adaptado", "winqsb", "original"].contains(&o.origen.as_str()), "{m}/{n}: origen {}", o.origen);
        if o.origen != "original" {
            assert!(!o.refs.is_empty(), "{m}/{n}: un ejemplo de {} debe citar de dónde sale", o.origen);
        }
        if o.estado != "verificada" {
            assert!(!o.pendiente.is_empty(), "{m}/{n}: por verificar sin decir qué falta");
        }
        let r = api::solve(m, model.clone(), None).unwrap();
        let refs = r["references"].as_array().unwrap_or_else(|| panic!("{m}/{n}: sin referencias de métodos"));
        assert!(!refs.is_empty(), "{m}/{n}: no cita ningún método");
        for c in refs {
            assert!(biblio::get(c["id"].as_str().unwrap()).is_some(), "{m}/{n}: cita {}, que no está en la bibliografía", c["id"]);
        }
    }
    for k in biblio::example_origins().keys() {
        let (m, n) = k.split_once('/').unwrap();
        assert!(ex.iter().any(|e| e.0 == m && e.1 == n), "fuentes/ejemplos.json menciona {k}, que no existe");
    }
}

#[test]
fn los_resultados_de_las_fuentes_se_reproducen() {
    for (m, n, model) in api::examples() {
        let Some(src) = biblio::data_source(&m, &n) else { continue };
        let r = api::solve(&m, model, None).unwrap();
        // el informe sabe de dónde salen los datos y lo muestra en «Fuentes»
        assert_eq!(r["data_source"]["key"], json!(format!("{m}/{n}")), "{m}/{n}");
        let sec = r["sections"].as_array().unwrap().iter().find(|s| s["id"] == "fuentes").expect("sin sección de fuentes");
        assert_eq!(sec["blocks"][0]["type"], "sources");
        if src.expected.is_empty() {
            continue;
        }
        // números del resumen y de la sección de solución (métricas, textos y tablas)
        let mut pool = numbers(&r["headline"].as_str().unwrap().replace(['→', '-'], " "));
        let sol = r["sections"].as_array().unwrap().iter().find(|s| s["id"] == "solucion").cloned().unwrap_or(json!({"blocks": []}));
        let mut walk = vec![sol];
        while let Some(v) = walk.pop() {
            match v {
                Value::Number(n) => pool.push(n.as_f64().unwrap()),
                Value::String(s) => pool.extend(numbers(&s.replace(['→', '-'], " "))),
                Value::Array(a) => walk.extend(a),
                Value::Object(o) => walk.extend(o.into_iter().map(|x| x.1)),
                _ => {}
            }
        }
        for tok in src.expected.replace(['→', '-'], " ").split(|c: char| !(c.is_ascii_digit() || c == '.')).filter(|t| !t.is_empty() && t.chars().any(|c| c.is_ascii_digit())) {
            let tok = tok.trim_end_matches('.');
            let x: f64 = tok.parse().unwrap();
            let dec = tok.split_once('.').map(|(_, d)| d.len() as i32).unwrap_or(0);
            let f = 10f64.powi(dec);
            assert!(
                pool.iter().any(|h| ((h * f).round() / f - x).abs() < 1e-9 || (h - x).abs() < 1e-6),
                "{m}/{n}: la fuente dice {:?} ({x}) y no aparece en el informe: {:?}",
                src.expected,
                r["headline"]
            );
        }
    }
}

#[test]
fn cada_metodo_cita_fuentes_existentes() {
    let ex = api::examples();
    let get = |n: &str| ex.iter().find(|e| e.1 == n).unwrap().2.clone();
    let cases: Vec<(&str, &str, Value)> = vec![
        ("net", "transporte", json!({"initial": "northwest_corner"})),
        ("net", "transporte", json!({"initial": "russell"})),
        ("net", "transporte", json!({})),
        ("net", "asignacion", json!({})),
        ("net", "transbordo", json!({})),
        ("net", "clase_transbordo_caso2", json!({})),
        ("net", "ruta_mas_corta", json!({})),
        ("net", "flujo_maximo", json!({})),
        ("net", "arbol_minimo", json!({"mst_method": "prim"})),
        ("net", "agente_viajero", json!({"tsp_method": "two_opt", "mtz_form": "libro"})),
        ("net", "clase_tsp_bebidas", json!({"tsp_method": "nearest_neighbor"})),
        ("vrp", "barrido_clase", json!({})),
        ("vrp", "reparto_ciudad", json!({"method": "savings"})),
        ("vrp", "reparto_bogota", json!({"method": "sweep"})),
        ("modo", "tren_camion_avion", json!({})),
        ("inv", "parcial_poq_entregas", json!({})),
        ("inv", "abc_15_articulos", json!({})),
        ("inv", "sell_rite", json!({})),
        ("inv", "taller2_qr", json!({"search": false})),
        ("fin", "roi_escenarios", json!({})),
        ("fin", "capital_de_trabajo", json!({})),
        ("alm", "envasadora", json!({})),
        ("alm", "taller3_dimensionado", json!({})),
        ("kpi", "taller1_distribuidor", json!({})),
    ];
    for (m, n, o) in cases {
        let r = api::solve(m, get(n), Some(o.clone())).unwrap();
        let refs = r["references"].as_array().unwrap_or_else(|| panic!("{m}/{n} {o}: sin referencias"));
        assert!(!refs.is_empty());
        for c in refs {
            let id = c["id"].as_str().unwrap();
            assert!(biblio::get(id).is_some(), "{m}/{n}: cita {id}, que no está en la bibliografía");
            assert!(!c["used_for"].as_array().unwrap().is_empty());
        }
        let ids: Vec<&str> = refs.iter().map(|c| c["id"].as_str().unwrap()).collect();
        match (n, o.get("initial").and_then(|v| v.as_str()), o.get("tsp_method").and_then(|v| v.as_str())) {
            ("transporte", Some("russell"), _) => assert!(ids.contains(&"russell1969")),
            ("transporte", None, _) => assert!(ids.contains(&"reinfeldvogel1958")),
            ("asignacion", _, _) => assert!(ids.contains(&"kuhn1955")),
            ("arbol_minimo", _, _) => assert!(ids.contains(&"prim1957") && !ids.contains(&"kruskal1956")),
            ("agente_viajero", _, Some("two_opt")) => assert!(ids.contains(&"croes1958") && ids.contains(&"winston2004")),
            ("clase_transbordo_caso2", _, _) => assert!(ids.contains(&"sarache2026redes") && ids.contains(&"orden1956")),
            ("barrido_clase", _, _) => assert!(ids.contains(&"gillettmiller1974") && ids.contains(&"clarkewright1964")),
            ("reparto_bogota", _, _) => assert!(ids.contains(&"sinnott1984") && ids.contains(&"gillettmiller1974")),
            ("tren_camion_avion", _, _) => assert!(ids.contains(&"ballou2004")),
            ("parcial_poq_entregas", _, _) => assert!(ids.contains(&"harris1913") && ids.contains(&"taft1918")),
            ("abc_15_articulos", _, _) => assert!(ids.contains(&"dickie1951")),
            ("taller2_qr", _, _) => assert!(ids.contains(&"sarache2026taller2")),
            ("capital_de_trabajo", _, _) => assert!(ids.contains(&"richards1980")),
            ("taller3_dimensionado", _, _) => assert!(ids.contains(&"blanktarquin")),
            ("taller1_distribuidor", _, _) => assert!(ids.contains(&"welch1947")),
            _ => {}
        }
    }
}

#[test]
fn bibliografia_y_bibtex() {
    let b = api::bibliography();
    let all = b.as_array().unwrap();
    assert!(all.len() >= 40);
    // ordenada como una lista APA
    let keys: Vec<String> = all.iter().map(|r| r["apa"].as_str().unwrap().replace('*', "").to_lowercase()).collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
    let bt = api::call("bibtex", json!({})).unwrap();
    let t = bt["content"].as_str().unwrap();
    assert!(t.contains("author = {Rosenkrantz, D. J. and Stearns, R. E. and Lewis, II, P. M.}"), "{t}");
    assert!(t.contains("@incollection{dantzig1951,"));
}
