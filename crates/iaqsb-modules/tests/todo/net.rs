use iaqsb_core::Module;
use iaqsb_modules::net::Net;
use serde_json::{json, Value};

fn ex(name: &str) -> Value {
    Net.examples().into_iter().find(|e| e.0 == name).unwrap().1
}
fn run(model: &Value, opts: Value) -> iaqsb_core::Report {
    Net.solve(model, &opts).unwrap_or_else(|e| panic!("{e}"))
}
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6 * (1.0 + b.abs())
}

#[test]
fn transporte_todos_los_metodos_iniciales() {
    let m = ex("transporte");
    for ini in ["northwest_corner", "minimum_cost", "vogel", "russell"] {
        let r = run(&m, json!({"initial": ini}));
        assert!(close(r.data["objective_value"].as_f64().unwrap(), 1020.0), "{ini}: {}", r.data);
    }
}

#[test]
fn transporte_desequilibrado_maximizar_y_prohibidas() {
    let mut m = ex("transporte");
    m["sources"][0]["supply"] = json!(60); // sobra oferta
    m["costs"][0][1] = Value::Null; // celda prohibida
    let r = run(&m, json!({}));
    assert_eq!(r.data["dummy"], "destination");
    m["objective"] = json!("max");
    let r = run(&m, json!({"initial": "northwest_corner"}));
    assert_eq!(r.status, iaqsb_core::Status::Optimal);
}

#[test]
fn asignacion() {
    let r = run(&ex("asignacion"), json!({}));
    assert!(close(r.data["objective_value"].as_f64().unwrap(), 13.0), "{}", r.data);
    let mut m = ex("asignacion");
    m["objective"] = json!("max");
    let r = run(&m, json!({}));
    assert!(close(r.data["objective_value"].as_f64().unwrap(), 33.0), "{}", r.data);
}

#[test]
fn seervada() {
    let r = run(&ex("ruta_mas_corta"), json!({}));
    assert!(close(r.data["distance"].as_f64().unwrap(), 13.0));
    let r = run(&ex("arbol_minimo"), json!({}));
    assert!(close(r.data["total"].as_f64().unwrap(), 14.0));
    let r = run(&ex("arbol_minimo"), json!({"mst_method": "prim"}));
    assert!(close(r.data["total"].as_f64().unwrap(), 14.0));
    let r = run(&ex("flujo_maximo"), json!({}));
    assert!(close(r.data["max_flow"].as_f64().unwrap(), 14.0), "{}", r.data);
}

#[test]
fn agente_viajero() {
    let m = ex("agente_viajero");
    let opt = run(&m, json!({})).data["distance"].as_f64().unwrap();
    for meth in ["nearest_neighbor", "cheapest_insertion", "two_opt"] {
        let h = run(&m, json!({"tsp_method": meth})).data["distance"].as_f64().unwrap();
        assert!(h >= opt - 1e-9, "{meth}");
    }
}

#[test]
fn transbordo() {
    let r = run(&ex("transbordo"), json!({}));
    assert_eq!(r.status, iaqsb_core::Status::Optimal);
}

#[test]
fn archivo_ida_y_vuelta() {
    for (name, m) in Net.examples() {
        let bytes = Net.write_file(&m).unwrap();
        let back = Net.read_file(&bytes).unwrap();
        let a = run(&m, json!({"steps": false}));
        let b = run(&back, json!({"steps": false}));
        // con costo o capacidad en los nodos se guarda la red equivalente: el resumen cambia, el costo no
        if a.headline != b.headline {
            let v = |r: &iaqsb_core::Report| r.data["objective_value"].as_f64().unwrap();
            assert!(close(v(&a), v(&b)), "{name}: {} / {}", a.headline, b.headline);
        }
    }
}

// ---------------------------------------------------------------- ejercicios de la clase de logística

fn obj(r: &iaqsb_core::Report) -> f64 {
    r.data["objective_value"].as_f64().or(r.data["distance"].as_f64()).or(r.data["max_flow"].as_f64()).or(r.data["total"].as_f64()).unwrap_or(f64::NAN)
}

#[test]
fn clase_transbordo_casos_1_2_y_3() {
    // caso 1: sin costo en los nodos (20.700 en WinQSB)
    assert!(close(obj(&run(&ex("clase_transbordo_caso1"), json!({}))), 20700.0));
    // caso 2: costo de 4 y 6 en los nodos de transbordo (4.920); la oferta no alcanza
    let r = run(&ex("clase_transbordo_caso2"), json!({}));
    assert!(close(r.data["objective_value"].as_f64().unwrap(), 4920.0), "{}", r.data);
    assert!(close(r.data["unmet"].as_f64().unwrap(), 90.0));
    // caso 3: capacidad de 400 y 200 en C y D (5.600, con los mismos envíos de la clase)
    let r = run(&ex("clase_transbordo_caso3"), json!({}));
    assert!(close(r.data["objective_value"].as_f64().unwrap(), 5600.0), "{}", r.data);
    let f = |a: &str, b: &str| r.data["flows"].as_array().unwrap().iter().find(|x| x["from"] == a && x["to"] == b).map(|x| x["flow"].as_f64().unwrap()).unwrap_or(0.0);
    assert!(close(f("A", "C"), 100.0) && close(f("B", "C"), 300.0) && close(f("B", "D"), 200.0) && close(f("C", "E"), 300.0) && close(f("C", "F"), 100.0) && close(f("D", "G"), 200.0));
}

#[test]
fn clase_problema1_y_red_equivalente_para_winqsb() {
    let m = ex("clase_problema1");
    let r = run(&m, json!({}));
    // comprobado con scipy (HiGHS) por separado: 11.574
    assert!(close(r.data["objective_value"].as_f64().unwrap(), 11574.0), "{}", r.data);
    assert!(r.data["lp_model"]["variables"].as_array().unwrap().len() == 14);
    // el archivo .NET lleva la red equivalente (nodos divididos y destino ficticio) y da lo mismo
    let bytes = Net.write_file(&m).unwrap();
    let back = Net.read_file(&bytes).unwrap();
    assert!(back["nodes"].as_array().unwrap().iter().any(|n| n["name"] == "Ficticio"));
    assert!(close(obj(&run(&back, json!({}))), 11574.0));
}

#[test]
fn clase_rutas_arboles_flujo_y_agente_viajero() {
    let r = run(&ex("clase_ruta_binaria"), json!({}));
    assert!(close(obj(&r), 14.0));
    assert_eq!(r.data["path"], json!(["A", "C", "E", "G", "H"]));
    // el modelo binario es el de la clase: 10 variables y 8 restricciones
    assert_eq!(r.data["lp_model"]["variables"].as_array().unwrap().len(), 10);
    assert_eq!(r.data["lp_model"]["constraints"].as_array().unwrap().len(), 8);
    assert!(close(obj(&run(&ex("clase_ruta_ejercicio1"), json!({}))), 47.0));
    let t3 = run(&ex("clase_arbol_ejercicio3"), json!({}));
    let t4 = run(&ex("clase_arbol_ejercicio4"), json!({}));
    assert!(t3.headline.contains("51") && t4.headline.contains("28"), "{} / {}", t3.headline, t4.headline);
    assert!(run(&ex("clase_flujo_maximo"), json!({})).headline.contains(": 25."));
    assert!(close(obj(&run(&ex("clase_tsp_4"), json!({}))), 11.0));
    let r6 = run(&ex("clase_tsp_6"), json!({}));
    assert!(close(obj(&r6), 17.0));
    // relajación de subtours de la clase: 12 de asignación + (n−1)(n−2) = 20
    assert_eq!(r6.data["lp_model"]["constraints"].as_array().unwrap().len(), 32);
    // vecino más cercano de la red de bebidas: 1-5-4-3-2-6-1 = 16
    let nn = run(&ex("clase_tsp_bebidas"), json!({"tsp_method": "nearest_neighbor"}));
    assert_eq!(nn.data["tour"], json!(["1", "5", "4", "3", "2", "6", "1"]));
    assert!(close(obj(&nn), 16.0));
}

#[test]
fn posiciones_del_editor_visual() {
    // las posiciones del editor visual (x, y de cada nodo) no cambian la solución,
    // se usan en el dibujo de la red y no van al archivo de WinQSB
    let mut m = ex("ruta_mas_corta");
    let base = run(&m, json!({}));
    let n = m["nodes"].as_array().unwrap().len();
    for (i, nd) in m["nodes"].as_array_mut().unwrap().iter_mut().enumerate() {
        nd["x"] = json!(100.0 + 80.0 * i as f64);
        nd["y"] = json!(if i % 2 == 0 { 120.0 } else { 360.0 });
    }
    let r = run(&m, json!({}));
    assert_eq!(r.headline, base.headline);
    let rep = serde_json::to_value(&r).unwrap();
    let net = rep["sections"].as_array().unwrap().iter().flat_map(|s| s["blocks"].as_array().unwrap().iter()).find(|b| b["type"] == "network").unwrap().clone();
    let xs: Vec<f64> = net["nodes"].as_array().unwrap().iter().map(|nd| nd["x"].as_f64().unwrap()).collect();
    assert!((xs[0] - 0.05).abs() < 1e-9 && (xs[n - 1] - 0.95).abs() < 1e-9, "{xs:?}");
    assert!(xs.windows(2).all(|w| w[0] < w[1]), "el dibujo respeta el orden de izquierda a derecha: {xs:?}");
    // el modelo tipado conserva las posiciones (archivo JSON de IAQSB)
    let typed: iaqsb_modules::net::NetModel = serde_json::from_value(m.clone()).unwrap();
    assert_eq!(serde_json::to_value(&typed).unwrap()["nodes"][3]["x"], json!(340.0));
    // y el archivo de WinQSB se escribe y se lee igual que sin ellas
    let back = Net.read_file(&Net.write_file(&m).unwrap()).unwrap();
    assert!(back["nodes"][0].get("x").is_none());
    assert_eq!(back["arcs"], m["arcs"]);
}
