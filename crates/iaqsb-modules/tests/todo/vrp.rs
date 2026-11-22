use iaqsb_core::Module;
use iaqsb_modules::vrp::Vrp;
use serde_json::{json, Value};

fn ex(name: &str) -> Value {
    Vrp.examples().into_iter().find(|e| e.0 == name).unwrap().1
}
fn run(model: &Value, opts: Value) -> iaqsb_core::Report {
    Vrp.solve(model, &opts).unwrap_or_else(|e| panic!("{e}"))
}

fn check_feasible(m: &Value, r: &iaqsb_core::Report) {
    let cap = m["capacity"].as_f64().unwrap();
    let mut seen: Vec<String> = r.data["routes"].as_array().unwrap().iter().flat_map(|x| x["stops"].as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string())).collect();
    seen.sort();
    let mut all: Vec<String> = m["points"].as_array().unwrap()[1..].iter().map(|p| p["name"].as_str().unwrap().to_string()).collect();
    all.sort();
    assert_eq!(seen, all, "cada cliente se visita una sola vez");
    for x in r.data["routes"].as_array().unwrap() {
        assert!(x["load"].as_f64().unwrap() <= cap + 1e-9);
    }
}

#[test]
fn barrido_de_la_clase() {
    let m = ex("barrido_clase");
    // 7.900 kg con camiones de 3 toneladas: 3 camiones
    let r = run(&m, json!({"method": "sweep"}));
    assert_eq!(r.data["vehicles"], 3);
    check_feasible(&m, &r);
    for meth in ["savings", "nearest_neighbor", "exact"] {
        let r = run(&m, json!({"method": meth}));
        check_feasible(&m, &r);
    }
    // el óptimo (comprobado por enumeración en Python) es 47
    let r = run(&m, json!({"method": "exact", "time_limit": 60}));
    assert!((r.data["total_distance"].as_f64().unwrap() - 47.0).abs() < 1e-6, "{}", r.data);
}

#[test]
fn comparar_y_capacidad() {
    let m = ex("reparto_ciudad");
    let r = run(&m, json!({}));
    check_feasible(&m, &r);
    let comp = r.data["comparison"].as_array().unwrap();
    let best = comp.iter().map(|c| c["distance"].as_f64().unwrap()).fold(f64::INFINITY, f64::min);
    // el programa entero arranca con la mejor heurística, así que siempre responde
    // (aunque en un equipo lento se le acabe el tiempo) y nunca queda peor que ella
    let ip = comp
        .iter()
        .find(|c| {
            let t = c["method"].as_str().unwrap();
            t.starts_with("Óptimo") || t.starts_with("Programación entera")
        })
        .unwrap_or_else(|| panic!("falta el programa entero en la comparación: {}", r.data));
    let opt = ip["distance"].as_f64().unwrap();
    assert!((best - opt).abs() < 1e-6, "ninguna heurística supera al programa entero");
    // un cliente con más demanda que la capacidad no se puede atender
    let mut bad = m.clone();
    bad["points"][1]["demand"] = json!(5000);
    assert!(Vrp.solve(&bad, &json!({})).is_err());
}

#[test]
fn archivo_de_ruteo() {
    let m = ex("barrido_clase");
    let bytes = Vrp.write_file(&m).unwrap();
    assert!(String::from_utf8_lossy(&bytes).starts_with("IAQSB-VRP"));
    let back = Vrp.read_file(&bytes).unwrap();
    assert_eq!(back["points"], m["points"]);
}

#[test]
fn distancia_geografica_haversine() {
    use iaqsb_modules::vrp::haversine_km;
    // un grado de meridiano mide unos 111,2 km
    assert!((haversine_km(0.0, 0.0, 1.0, 0.0) - 111.195).abs() < 0.01);
    // Bogotá (Plaza de Bolívar) – Medellín (Parque Berrío): unos 240 km en línea recta
    let d = haversine_km(4.5981, -74.0760, 6.2518, -75.5636);
    assert!((d - 245.0).abs() < 8.0, "{d}");
    // simétrica y cero en el mismo punto
    assert!((haversine_km(4.6, -74.1, -34.6, -58.4) - haversine_km(-34.6, -58.4, 4.6, -74.1)).abs() < 1e-9);
    assert_eq!(haversine_km(4.6, -74.1, 4.6, -74.1), 0.0);
}

#[test]
fn reparto_en_bogota_con_coordenadas_reales() {
    let m = ex("reparto_bogota");
    assert_eq!(m["metric"], "haversine");
    let r = run(&m, json!({}));
    check_feasible(&m, &r);
    // 243 cajas con furgonetas de 100: al menos 3 vehículos
    assert!(r.data["vehicles"].as_u64().unwrap() >= 3);
    assert_eq!(r.data["distance_unit"], "km");
    let total = r.data["total_distance"].as_f64().unwrap();
    // todo queda dentro de la ciudad: unas decenas de km en total
    assert!(total > 20.0 && total < 120.0, "{total}");
    assert!(r.headline.contains("km"), "{}", r.headline);
    // el mapa del informe recibe longitud y latitud y se marca como geográfico
    let rep = serde_json::to_value(&r).unwrap();
    let map = rep["sections"].as_array().unwrap().iter().flat_map(|s| s["blocks"].as_array().unwrap().iter()).find(|b| b["type"] == "map").unwrap().clone();
    assert_eq!(map["geo"], true);
    assert!((map["points"][0]["x"].as_f64().unwrap() + 74.114).abs() < 1e-9);
    assert!((map["points"][0]["y"].as_f64().unwrap() - 4.6187).abs() < 1e-9);
    // la distancia de la tabla es la del gran círculo, redondeada al metro
    let d01 = iaqsb_modules::vrp::haversine_km(4.6187, -74.114, 4.6458, -74.064);
    let tabla = rep["sections"].as_array().unwrap().iter().find(|s| s["id"] == "distancias").unwrap()["blocks"][0].clone();
    let cell = tabla["rows"][0]["cells"][2].as_f64().unwrap();
    assert!((cell - (d01 * 1000.0).round() / 1000.0).abs() < 1e-9, "{cell} vs {d01}");
    // el barrido y los demás métodos también funcionan con latitud y longitud
    for meth in ["sweep", "savings", "nearest_neighbor", "exact"] {
        let r = run(&m, json!({"method": meth, "best_start": true}));
        check_feasible(&m, &r);
    }
}

#[test]
fn geografica_valida_latitud_y_longitud() {
    let mut m = ex("reparto_bogota");
    m["points"][2]["y"] = json!(95.0);
    let e = Vrp.solve(&m, &json!({})).unwrap_err().to_string();
    assert!(e.contains("latitud"), "{e}");
    let mut m = ex("reparto_bogota");
    m["points"][2]["x"] = json!(-200.0);
    assert!(Vrp.solve(&m, &json!({})).unwrap_err().to_string().contains("longitud"));
}

#[test]
fn plantilla_geografica_en_bogota() {
    let m = Vrp.template("coordinates", &json!({"customers": 5, "metric": "haversine"})).unwrap();
    assert_eq!(m["metric"], "haversine");
    assert_eq!(m["points"].as_array().unwrap().len(), 6);
    for p in m["points"].as_array().unwrap() {
        let (lat, lon) = (p["y"].as_f64().unwrap(), p["x"].as_f64().unwrap());
        assert!((lat - 4.61).abs() < 0.05 && (lon + 74.08).abs() < 0.05, "{p}");
    }
    // sin la opción, la plantilla sigue en el plano
    let m = Vrp.template("coordinates", &json!({"customers": 4})).unwrap();
    assert_eq!(m["metric"], "euclidean");
    // y el archivo de IAQSB conserva la distancia geográfica
    let g = ex("reparto_bogota");
    let back = Vrp.read_file(&Vrp.write_file(&g).unwrap()).unwrap();
    assert_eq!(back["metric"], "haversine");
    assert_eq!(back["points"], g["points"]);
}
