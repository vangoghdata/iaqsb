//! ¿Qué pasa si…? Barrido de cualquier dato numérico de cualquier módulo (en
//! paralelo) y análisis paramétrico exacto de LP-ILP (costos y lados derechos,
//! con los puntos donde cambia la base, a partir de los rangos de sensibilidad).

use crate::{solve, ApiError, Result};
use serde_json::{json, Value};

/// Ruta de un dato: puntero JSON («/constraints/2/rhs») o con puntos («constraints.2.rhs»).
pub fn pointer(path: &str) -> String {
    let p = path.trim();
    if p.starts_with('/') {
        p.to_string()
    } else {
        format!("/{}", p.replace('.', "/").replace('[', "/").replace(']', ""))
    }
}

fn set_at(model: &mut Value, ptr: &str, v: f64) -> Result<()> {
    let slot = model.pointer_mut(ptr).ok_or_else(|| ApiError::BadInput(format!("el modelo no tiene el dato {ptr}")))?;
    if !(slot.is_number() || slot.is_null() || slot.as_str().is_some_and(|s| s == "M" || s == "-M" || s.parse::<f64>().is_ok())) {
        return Err(ApiError::BadInput(format!("el dato {ptr} no es un número")));
    }
    *slot = json!(v);
    Ok(())
}

/// El valor que resume un informe: el objetivo en LP-ILP o la métrica pedida
/// (por su etiqueta o un puntero en el informe); si no se pide, la primera métrica.
fn metric(report: &Value, metric: Option<&str>) -> (Option<f64>, String) {
    if let Some(z) = report.get("objective_value") {
        if metric.is_none() || metric == Some("objective") {
            return (z.as_f64(), "Z".into());
        }
    }
    if let Some(m) = metric {
        if m.starts_with('/') {
            return (report.pointer(m).and_then(num), m.to_string());
        }
    }
    for sec in report["sections"].as_array().into_iter().flatten() {
        for b in sec["blocks"].as_array().into_iter().flatten() {
            if b["type"] == "metrics" {
                for it in b["items"].as_array().into_iter().flatten() {
                    let label = it["label"].as_str().unwrap_or("");
                    let want = metric.map_or(true, |m| label.eq_ignore_ascii_case(m) || label.to_lowercase().contains(&m.to_lowercase()));
                    if want {
                        if let Some(x) = num(&it["value"]) {
                            return (Some(x), label.to_string());
                        }
                    }
                }
            }
        }
    }
    (None, metric.unwrap_or("").to_string())
}

fn num(v: &Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.replace('−', "-").trim().trim_end_matches('%').trim().parse().ok()))
}

/// Lo esencial de una solución de LP-ILP: los valores de las variables.
fn lp_solution(r: &Value) -> Value {
    let mut o = serde_json::Map::new();
    for v in r["variables"].as_array().into_iter().flatten() {
        if let (Some(n), Some(x)) = (v["name"].as_str(), v["value"].as_f64()) {
            o.insert(n.to_string(), json!((x * 1e6).round() / 1e6));
        }
    }
    Value::Object(o)
}

/// Barrido: resuelve el modelo con cada valor del dato y devuelve el resumen de cada uno.
/// `values`: lista o {from, to, steps}. Los puntos se resuelven en paralelo.
pub fn sweep(module: &str, model: &Value, options: Option<&Value>, path: &str, values: &Value, metric_name: Option<&str>) -> Result<Value> {
    let ptr = pointer(path);
    if model.pointer(&ptr).is_none() {
        return Err(ApiError::BadInput(format!("el modelo no tiene el dato {ptr}")));
    }
    let vals: Vec<f64> = match values {
        Value::Array(a) => a.iter().filter_map(num).collect(),
        Value::Object(o) => {
            let from = o.get("from").and_then(num).ok_or_else(|| ApiError::BadInput("falta values.from".into()))?;
            let to = o.get("to").and_then(num).ok_or_else(|| ApiError::BadInput("falta values.to".into()))?;
            let steps = o.get("steps").and_then(|v| v.as_u64()).unwrap_or(20).clamp(1, 400) as usize;
            (0..=steps).map(|k| from + (to - from) * k as f64 / steps as f64).collect()
        }
        _ => return Err(ApiError::BadInput("values debe ser una lista de números o {from, to, steps}".into())),
    };
    if vals.is_empty() {
        return Err(ApiError::BadInput("no hay valores para probar".into()));
    }
    if vals.len() > 1000 {
        return Err(ApiError::BadInput("como máximo 1000 valores por barrido".into()));
    }
    let base = model.pointer(&ptr).cloned();
    let mut opts = options.cloned().unwrap_or(json!({}));
    // en el barrido no hace falta el paso a paso
    if module == "lp" {
        opts["trace"] = json!(false);
        opts["bnb_trace"] = json!(false);
        opts["diagnose"] = json!(false);
    } else {
        opts["steps"] = json!(false);
    }
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).min(8).min(vals.len());
    let mut results: Vec<Option<Value>> = vec![None; vals.len()];
    let chunk = vals.len().div_ceil(threads);
    std::thread::scope(|sc| {
        let mut handles = vec![];
        for (k, part) in vals.chunks(chunk).enumerate() {
            let (opts, model, ptr) = (&opts, model, &ptr);
            handles.push((k, sc.spawn(move || {
                part.iter()
                    .map(|&v| {
                        let mut m = model.clone();
                        let r = set_at(&mut m, ptr, v).and_then(|_| solve(module, m, Some(opts.clone())));
                        match r {
                            Ok(rep) => {
                                let (z, label) = metric(&rep, metric_name);
                                let z = z.map(|x| (x * 1e6).round() / 1e6);
                                let mut p = json!({ "value": v, "status": rep["status"], "metric": z, "metric_label": label,
                                    "headline": rep.get("headline").or(rep.get("message")).cloned().unwrap_or(Value::Null) });
                                if module == "lp" {
                                    p["solution"] = lp_solution(&rep);
                                }
                                p
                            }
                            Err(e) => json!({ "value": v, "status": "error", "metric": Value::Null, "error": e.to_string() }),
                        }
                    })
                    .collect::<Vec<_>>()
            })));
        }
        for (k, h) in handles {
            for (i, p) in h.join().unwrap_or_default().into_iter().enumerate() {
                results[k * chunk + i] = Some(p);
            }
        }
    });
    let points: Vec<Value> = results.into_iter().flatten().collect();
    // en LP-ILP: dónde cambia qué variables son positivas (la estructura de la solución)
    let support = |p: &Value| -> Vec<String> {
        let mut v: Vec<String> = p["solution"].as_object().map(|o| o.iter().filter(|(_, x)| x.as_f64().unwrap_or(0.0).abs() > 1e-9).map(|(k, _)| k.clone()).collect()).unwrap_or_default();
        v.sort();
        v
    };
    let mut changes = vec![];
    if module == "lp" {
        for w in points.windows(2) {
            if w[0]["status"] != w[1]["status"] || (w[0]["status"] == "optimal" && support(&w[0]) != support(&w[1])) {
                changes.push(json!({"between": [w[0]["value"], w[1]["value"]], "before": support(&w[0]), "after": support(&w[1])}));
            }
        }
    }
    Ok(json!({ "module": module, "path": ptr, "base_value": base, "points": points, "solution_changes": changes }))
}

fn bound(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) if s == "M" => f64::INFINITY,
        Value::String(s) if s == "-M" => f64::NEG_INFINITY,
        _ => f64::NAN,
    }
}

/// Análisis paramétrico exacto de un programa lineal: el objetivo como función
/// de un costo c(j) o de un lado derecho b(i) en [from, to], por tramos lineales.
/// En cada tramo la base óptima no cambia; los extremos salen de los rangos de
/// sensibilidad. `kind`: "cost" o "rhs"; `index`: número o nombre de la variable o restricción.
pub fn parametric_lp(model: &Value, kind: &str, index: &Value, from: f64, to: f64, options: Option<&Value>) -> Result<Value> {
    let (list, field) = match kind {
        "cost" => ("variables", "cost"),
        "rhs" => ("constraints", "rhs"),
        other => return Err(ApiError::BadInput(format!("kind debe ser cost o rhs (no {other})"))),
    };
    let items = model[list].as_array().ok_or_else(|| ApiError::BadInput(format!("el modelo no tiene {list}")))?;
    let k = match index {
        Value::Number(n) => n.as_u64().map(|x| x as usize),
        Value::String(s) => items.iter().position(|x| x["name"].as_str() == Some(s.as_str())),
        _ => None,
    }
    .filter(|k| *k < items.len())
    .ok_or_else(|| ApiError::BadInput(format!("no existe {index} en {list}")))?;
    let name = items[k]["name"].as_str().unwrap_or("").to_string();
    let (from, to) = if from <= to { (from, to) } else { (to, from) };
    if !(from.is_finite() && to.is_finite()) {
        return Err(ApiError::BadInput("from y to deben ser números finitos".into()));
    }
    let mut opts = options.cloned().unwrap_or(json!({}));
    opts["trace"] = json!(false);
    opts["bnb_trace"] = json!(false);
    opts["diagnose"] = json!(false);
    let solve_at = |v: f64| -> Result<Value> {
        let mut m = model.clone();
        m[list][k][field] = json!(v);
        solve("lp", m, Some(opts.clone()))
    };
    let span = (to - from).max(1e-9);
    let eps = span * 1e-7 + 1e-9;
    let mut segs: Vec<Value> = vec![];
    let mut v = from;
    let mut guard = 0;
    while v <= to + eps && guard < 300 {
        guard += 1;
        let r = solve_at(v)?;
        let st = r["status"].as_str().unwrap_or("").to_string();
        if st != "optimal" || r["sensitivity"].as_bool() != Some(true) {
            // sin solución (o entera, sin rangos): se avanza por pasos hasta que haya
            let step = span / 50.0;
            let next = (v + step).min(to);
            segs.push(json!({ "from": v, "to": next, "status": st, "objective_from": r["objective_value"], "objective_to": Value::Null, "slope": Value::Null }));
            if next >= to {
                break;
            }
            v = next + eps;
            continue;
        }
        let z = r["objective_value"].as_f64().unwrap_or(0.0);
        let (lo, hi, slope) = if kind == "cost" {
            let x = &r["variables"][k];
            (bound(&x["cost_min"]), bound(&x["cost_max"]), x["value"].as_f64().unwrap_or(0.0))
        } else {
            let c = &r["constraints"][k];
            (bound(&c["rhs_min"]), bound(&c["rhs_max"]), c["shadow_price"].as_f64().unwrap_or(0.0))
        };
        let hi = if hi.is_nan() { v } else { hi };
        let _ = lo;
        let end = hi.min(to).max(v);
        segs.push(json!({
            "from": v, "to": end, "status": "optimal", "slope": slope,
            "objective_from": z, "objective_to": z + slope * (end - v),
            "solution": lp_solution(&r),
        }));
        if end >= to - eps {
            break;
        }
        v = end + eps.max((end.abs()) * 1e-9);
    }
    // une tramos contiguos con la misma pendiente y la misma solución
    let mut merged: Vec<Value> = vec![];
    for s in segs {
        if let Some(last) = merged.last_mut() {
            if last["status"] == s["status"] && last["slope"] == s["slope"] && last.get("solution") == s.get("solution") {
                last["to"] = s["to"].clone();
                last["objective_to"] = s["objective_to"].clone();
                continue;
            }
        }
        merged.push(s);
    }
    // los tramos de largo cero (puntos degenerados) no aportan
    if merged.len() > 1 {
        merged.retain(|s| (s["to"].as_f64().unwrap_or(0.0) - s["from"].as_f64().unwrap_or(0.0)).abs() > eps * 10.0);
    }
    let breakpoints: Vec<f64> = merged.windows(2).filter_map(|w| w[0]["to"].as_f64()).collect();
    Ok(json!({ "kind": kind, "index": k, "name": name, "from": from, "to": to, "segments": merged, "breakpoints": breakpoints }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn barrido_en_paralelo_de_wyndor() {
        let m = crate::example_model("lp").unwrap();
        let r = sweep("lp", &m, None, "constraints.2.rhs", &json!({"from": 12, "to": 24, "steps": 6}), None).unwrap();
        let pts = r["points"].as_array().unwrap();
        assert_eq!(pts.len(), 7);
        // Wyndor: con b3 = 18, Z = 36; el precio sombra de la planta 3 es 1
        let at18 = pts.iter().find(|p| p["value"] == 18.0).unwrap();
        assert!((at18["metric"].as_f64().unwrap() - 36.0).abs() < 1e-6);
        let at20 = pts.iter().find(|p| p["value"] == 20.0).unwrap();
        assert!((at20["metric"].as_f64().unwrap() - 38.0).abs() < 1e-6);
        // los puntos quedan en orden
        let v: Vec<f64> = pts.iter().map(|p| p["value"].as_f64().unwrap()).collect();
        assert!(v.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn parametrico_exacto_del_costo() {
        // Wyndor: c1 en [0, 10]; la base cambia en c1 = 7.5 (rango [0, 7.5] de Hillier)
        let m = crate::example_model("lp").unwrap();
        let p = parametric_lp(&m, "cost", &json!(0), 0.0, 10.0, None).unwrap();
        let bps = p["breakpoints"].as_array().unwrap();
        assert!(bps.iter().any(|b| (b.as_f64().unwrap() - 7.5).abs() < 1e-4), "{p}");
        // el objetivo es continuo en los quiebres
        let segs = p["segments"].as_array().unwrap();
        for w in segs.windows(2) {
            let (a, b) = (w[0]["objective_to"].as_f64().unwrap(), w[1]["objective_from"].as_f64().unwrap());
            assert!((a - b).abs() < 1e-3, "{a} {b}");
        }
    }

    #[test]
    fn parametrico_del_lado_derecho_con_infactibilidad() {
        let m = crate::example_model("lp").unwrap();
        let p = parametric_lp(&m, "rhs", &json!("Planta3"), 0.0, 40.0, None).unwrap();
        let segs = p["segments"].as_array().unwrap();
        assert!(!segs.is_empty());
        // a partir de b3 = 18, la pendiente es el precio sombra 1 hasta 24, después 0
        let s = segs.iter().find(|s| s["from"].as_f64().unwrap() <= 18.0 && s["to"].as_f64().unwrap() >= 18.0).unwrap();
        assert!((s["slope"].as_f64().unwrap() - 1.0).abs() < 1e-6, "{s}");
    }

    #[test]
    fn barrido_de_otro_modulo() {
        let (m, _, model) = crate::examples().into_iter().find(|(m, _, x)| m == "its" && x.get("annual_demand").is_some() || m == "its" && x.get("demand").is_some()).unwrap();
        let key = if model.get("annual_demand").is_some() { "annual_demand" } else { "demand" };
        let base = model[key].as_f64().unwrap();
        let r = sweep(&m, &model, None, key, &json!([base * 0.5, base, base * 2.0]), None).unwrap();
        let pts = r["points"].as_array().unwrap();
        assert_eq!(pts.len(), 3);
        assert!(pts.iter().all(|p| p["status"] != "error"), "{r}");
    }
}
