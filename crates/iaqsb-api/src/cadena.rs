//! Cadena jerárquica de planificación (Domínguez Machuca et al., 1995; Arango
//! Marín, 2026): el resultado de un nivel alimenta al siguiente.
//!
//! - `ap → pmp`: la producción del plan agregado (familia) se desagrega en
//!   productos según su participación y da un programa maestro básico por producto.
//! - `pmp → mrp`: el PMP (lotes por periodo) de un producto pasa a ser la fila del
//!   programa maestro de su artículo en el MRP, con los periodos ajustados.
//!
//! Todo se resuelve con el mismo motor; la app, la CLI, el MCP y el asistente lo usan igual.

use crate::{solve, ApiError, Result};
use serde_json::{json, Value};

fn num(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0)
}

/// `args`: { from, to, model, options?, products? (ap→pmp), target? y item? (pmp→mrp) }.
pub fn link(args: &Value) -> Result<Value> {
    let from = args["from"].as_str().unwrap_or("");
    let to = args["to"].as_str().unwrap_or("");
    let model = args.get("model").cloned().ok_or_else(|| ApiError::BadInput("falta model (el modelo del nivel de origen)".into()))?;
    let options = args.get("options").cloned();
    match (from, to) {
        ("ap", "pmp") => ap_a_pmp(model, options, args.get("products")),
        ("pmp", "mrp") => pmp_a_mrp(model, options, args.get("target"), args["item"].as_str()),
        _ => Err(ApiError::BadInput(format!("enlace no disponible: {from} → {to} (usa ap → pmp o pmp → mrp)"))),
    }
}

fn ap_a_pmp(model: Value, options: Option<Value>, productos: Option<&Value>) -> Result<Value> {
    let r = solve("ap", model.clone(), options)?;
    let per = r["data"]["periods"].as_array().ok_or_else(|| ApiError::BadInput("el plan agregado no devolvió la producción por periodo".into()))?;
    let prods: Vec<Value> = match productos.and_then(|p| p.as_array()) {
        Some(p) if !p.is_empty() => p.clone(),
        _ => vec![json!({"name": model["name"].as_str().unwrap_or("Familia"), "share": 1.0})],
    };
    let total: f64 = prods.iter().map(|p| num(p, "share")).sum();
    if total <= 0.0 {
        return Err(ApiError::BadInput("las participaciones (share) de los productos deben sumar más de 0".into()));
    }
    let mut modelos = vec![];
    let mut tabla = vec![];
    for p in &prods {
        let nombre = p["name"].as_str().unwrap_or("Producto").to_string();
        let parte = num(p, "share") / total;
        let periodos: Vec<Value> = per
            .iter()
            .map(|q| json!({"name": q["period"].as_str().unwrap_or(""), "forecast": (num(q, "production") * parte).round(), "orders": 0}))
            .collect();
        tabla.push(json!({"product": nombre, "share": parte, "demand": periodos.iter().map(|q| q["forecast"].clone()).collect::<Vec<_>>()}));
        modelos.push(json!({
            "module": "pmp",
            "model": {
                "name": format!("PMP de {nombre} (desde el plan agregado)"), "problem_type": "basico",
                "on_hand": num(p, "on_hand"), "lot_size": num(p, "lot_size"), "safety_stock": num(p, "safety_stock"),
                "demand_fence": 0, "planning_fence": 0, "hours_per_unit": 0, "setup_cost": 0, "holding_cost": 0,
                "periods": periodos, "requests": []
            }
        }));
    }
    Ok(json!({
        "from": "ap", "to": "pmp", "source_headline": r["headline"], "models": modelos, "disaggregation": tabla,
        "summary": format!("El plan agregado ({}) se desagregó en {} programa(s) maestro(s): la demanda de cada producto en cada periodo es la producción agregada por su participación.", r["headline"].as_str().unwrap_or(""), modelos.len())
    }))
}

fn pmp_a_mrp(model: Value, options: Option<Value>, target: Option<&Value>, item: Option<&str>) -> Result<Value> {
    let r = solve("pmp", model, options)?;
    let per = r["data"]["periods"].as_array().ok_or_else(|| ApiError::BadInput("el programa maestro no devolvió el PMP por periodo".into()))?;
    let pmp: Vec<f64> = per.iter().map(|q| num(q, "mps")).collect();
    let nombres: Vec<String> = per.iter().map(|q| q["period"].as_str().unwrap_or("").to_string()).collect();
    let mut mrp = target.cloned().ok_or_else(|| ApiError::BadInput("falta target: el modelo MRP (artículos y lista de materiales) que recibe el PMP".into()))?;
    let items = mrp["items"].as_array().cloned().unwrap_or_default();
    if items.is_empty() {
        return Err(ApiError::BadInput("el modelo MRP no tiene artículos".into()));
    }
    let idx = match item {
        Some(id) => items.iter().position(|x| x["id"].as_str() == Some(id)).ok_or_else(|| ApiError::BadInput(format!("el artículo «{id}» no está en el MRP")))?,
        None => 0,
    };
    let n = pmp.len();
    // los periodos del MRP pasan a ser los del PMP; las matrices se recortan o se alargan
    mrp["periods"] = json!(nombres.iter().map(|s| json!({"name": s})).collect::<Vec<_>>());
    let ajustar = |m: &mut Value, relleno: Value| {
        let filas = m.as_array().cloned().unwrap_or_default();
        let nuevas: Vec<Value> = (0..items.len())
            .map(|i| {
                let f = filas.get(i).and_then(|f| f.as_array()).cloned().unwrap_or_default();
                json!((0..n).map(|t| f.get(t).cloned().unwrap_or(relleno.clone())).collect::<Vec<_>>())
            })
            .collect();
        *m = json!(nuevas);
    };
    ajustar(&mut mrp["mps"], json!(0.0));
    ajustar(&mut mrp["receipts"], json!(0.0));
    ajustar(&mut mrp["capacity"], Value::Null);
    mrp["mps"][idx] = json!(pmp);
    let id = items[idx]["id"].as_str().unwrap_or("").to_string();
    Ok(json!({
        "from": "pmp", "to": "mrp", "source_headline": r["headline"], "item": id,
        "models": [{"module": "mrp", "model": mrp}],
        "summary": format!("El PMP ({}) quedó como programa maestro del artículo {} en el MRP, con {} periodos.", r["headline"].as_str().unwrap_or(""), id, n)
    }))
}
