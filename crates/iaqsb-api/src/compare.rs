//! Comparar métodos: el mismo modelo se resuelve con cada valor de una opción
//! de método (o con las variantes que se indiquen) y se arma un informe con la
//! tabla comparativa, un gráfico y la solución de cada variante.

use crate::{available, generic, solve, ApiError, Result};
use iaqsb_core::biblio::Citation;
use iaqsb_core::report::*;
use iaqsb_core::spec::{Field, FieldKind};
use serde_json::{json, Map, Value};

/// Opciones de método que se pueden comparar en este modelo (las visibles para su tipo de problema).
pub fn comparable_fields(id: &str, model: &Value, options: &Value) -> Vec<Field> {
    let Some(g) = generic(id) else { return vec![] };
    let spec = g.spec();
    let get = |k: &str| -> Value {
        model.get(k).cloned().or_else(|| options.get(k).cloned()).or_else(|| spec.options.iter().find(|f| f.key == k).map(|f| f.default.clone())).unwrap_or(Value::Null)
    };
    let txt = |v: &Value| match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    spec.options
        .iter()
        .filter(|f| f.comparable && matches!(f.kind, FieldKind::Choice { .. }))
        .filter(|f| f.show_if.iter().all(|(k, vals)| vals.contains(&txt(&get(k)))))
        .cloned()
        .collect()
}

/// Métricas de un informe: las del primer bloque de métricas (o el objetivo de LP).
fn metrics_of(r: &Value) -> Vec<(String, f64)> {
    for sec in r["sections"].as_array().into_iter().flatten() {
        for b in sec["blocks"].as_array().into_iter().flatten() {
            if b["type"] == "metrics" {
                let v: Vec<(String, f64)> = b["items"].as_array().into_iter().flatten().filter_map(|it| Some((it["label"].as_str()?.to_string(), it["value"].as_f64()?))).collect();
                if !v.is_empty() {
                    return v;
                }
            }
        }
    }
    r["objective_value"].as_f64().map(|v| vec![("Valor de la función objetivo".to_string(), v)]).unwrap_or_default()
}

/// ¿Conviene el valor menor? Por el criterio del modelo o por el nombre de la métrica.
fn smaller_is_better(model: &Value, label: &str, emphasized: bool) -> Option<bool> {
    let l = label.to_lowercase();
    if ["mad", "mse", "mape", "error", "costo", "distancia", "tiempo", "cmax", "tardanza", "tardíos", "faltante", "perdid", "atraso", "retraso", "inventario promedio"].iter().any(|k| l.contains(k)) {
        return Some(true);
    }
    if ["utilidad", "ganancia", "beneficio", "flujo", "ingreso", "rendimiento", "nivel de servicio", "roi"].iter().any(|k| l.contains(k)) {
        return Some(false);
    }
    if emphasized {
        match model.get("objective").and_then(|v| v.as_str()).map(|s| s.to_lowercase()) {
            Some(o) if o.starts_with("max") => return Some(false),
            Some(o) if o.starts_with("min") => return Some(true),
            _ => {}
        }
    }
    None
}

fn merge(base: &Value, over: &Value) -> Value {
    let mut m: Map<String, Value> = base.as_object().cloned().unwrap_or_default();
    if let Some(o) = over.as_object() {
        for (k, v) in o {
            if k != "label" {
                m.insert(k.clone(), v.clone());
            }
        }
    }
    Value::Object(m)
}

/// Compara métodos. `vary`: la opción que cambia (vacío = la primera comparable);
/// `values`: los valores a probar (vacío = todos); `variants`: opciones completas
/// de cada variante (con "label" opcional), en lugar de `vary`.
pub fn compare(id: &str, model: Value, options: Option<Value>, vary: Option<String>, values: Option<Vec<Value>>, variants: Option<Vec<Value>>, metric_pref: Option<String>, full: bool) -> Result<Value> {
    let info = available(id)?;
    let base = options.unwrap_or(json!({}));
    let mut list: Vec<(String, Value)> = vec![];
    let what: String;
    if let Some(vs) = variants.filter(|v| !v.is_empty()) {
        for (k, v) in vs.iter().enumerate() {
            let label = v.get("label").and_then(|x| x.as_str()).map(str::to_string).unwrap_or_else(|| {
                v.as_object().map(|o| o.iter().filter(|(k, _)| *k != "label").map(|(k, x)| format!("{k} = {}", x.as_str().map(str::to_string).unwrap_or(x.to_string()))).collect::<Vec<_>>().join(", ")).filter(|s| !s.is_empty()).unwrap_or(format!("Variante {}", k + 1))
            });
            list.push((label, merge(&base, v)));
        }
        what = "las variantes indicadas".into();
    } else {
        let fields = comparable_fields(info.id, &model, &base);
        let f = match vary.as_deref() {
            Some(k) => fields.iter().find(|f| f.key == k).cloned().or_else(|| generic(info.id).and_then(|g| g.spec().options.into_iter().find(|f| f.key == k))),
            None => fields.first().cloned(),
        }
        .ok_or_else(|| ApiError::BadInput(if fields.is_empty() { "este tipo de problema no tiene métodos para comparar; indica las variantes (variants) con las opciones de cada una".into() } else { format!("la opción {} no existe; se pueden comparar: {}", vary.unwrap_or_default(), fields.iter().map(|f| f.key.clone()).collect::<Vec<_>>().join(", ")) }))?;
        let FieldKind::Choice { options: opts } = &f.kind else { return Err(ApiError::BadInput(format!("la opción {} no es una lista de métodos", f.key))) };
        let wanted: Option<Vec<String>> = values.map(|vs| vs.iter().map(|v| v.as_str().map(str::to_string).unwrap_or(v.to_string())).collect());
        for (val, label) in opts {
            if val == "compare" {
                continue;
            }
            if wanted.as_ref().is_some_and(|w| !w.contains(val)) {
                continue;
            }
            list.push((label.clone(), merge(&base, &json!({ f.key.clone(): val }))));
        }
        what = format!("cada valor de «{}»", f.label);
    }
    if list.len() < 2 {
        return Err(ApiError::BadInput("se necesitan al menos dos variantes para comparar".into()));
    }
    // resolver cada variante
    let mut rows = vec![];
    for (label, opts) in &list {
        match solve(info.id, model.clone(), Some(opts.clone())) {
            Ok(r) => rows.push((label.clone(), opts.clone(), Ok(r))),
            Err(e) => rows.push((label.clone(), opts.clone(), Err(e.to_string()))),
        }
    }
    let all: Vec<Vec<(String, f64)>> = rows.iter().map(|r| r.2.as_ref().map(metrics_of).unwrap_or_default()).collect();
    let mut labels: Vec<String> = vec![];
    for v in &all {
        for (l, _) in v {
            if !labels.contains(l) {
                labels.push(l.clone());
            }
        }
    }
    labels.dedup();
    // métrica de comparación: la pedida, o la primera cuyo sentido se conoce, o la destacada
    let metric_label = match metric_pref.as_deref() {
        Some(m) => labels.iter().find(|l| l.to_lowercase().contains(&m.to_lowercase())).cloned().ok_or_else(|| ApiError::BadInput(format!("no hay ninguna métrica «{m}»; hay: {}", labels.join(", "))))?,
        None => labels.iter().enumerate().find(|(k, l)| smaller_is_better(&model, l, *k == 0).is_some()).map(|x| x.1.clone()).or_else(|| labels.first().cloned()).unwrap_or_else(|| "Resultado".into()),
    };
    let dir = smaller_is_better(&model, &metric_label, labels.first() == Some(&metric_label));
    let val_of = |k: usize, l: &str| all[k].iter().find(|x| x.0 == l).map(|x| x.1);
    let vals: Vec<Option<f64>> = (0..rows.len()).map(|k| val_of(k, &metric_label)).collect();
    let best = dir.and_then(|small| {
        (0..vals.len()).filter(|&k| vals[k].is_some()).min_by(|&a, &b| {
            let (x, y) = (vals[a].unwrap(), vals[b].unwrap());
            if small { x.total_cmp(&y) } else { y.total_cmp(&x) }
        })
    });
    let tied: Vec<usize> = best.map(|b| (0..vals.len()).filter(|&k| vals[k].is_some_and(|v| (v - vals[b].unwrap()).abs() <= 1e-9 * (1.0 + v.abs()))).collect()).unwrap_or_default();
    let shown: Vec<String> = labels.iter().take(6).cloned().collect();
    let mut head: Vec<String> = vec!["Variante".into()];
    head.extend(shown.iter().cloned());
    head.extend(["Diferencia con la mejor".to_string(), "Estado".into(), "Resumen".into()]);
    let ncol = head.len();
    let mut t = Table::new(Some(&format!("Comparación por {}", lc(&metric_label))), &head).left(0).left(ncol - 2).left(ncol - 1);
    for (k, (label, _, r)) in rows.iter().enumerate() {
        let diff = match (best, vals[k]) {
            (Some(b), Some(v)) => Cell::Num(v - vals[b].unwrap()),
            _ => Cell::from("—"),
        };
        let (status, head) = match r {
            Ok(r) => (status_es(r["status"].as_str().unwrap_or("")).to_string(), r["headline"].as_str().unwrap_or("").to_string()),
            Err(e) => ("Error".into(), e.clone()),
        };
        let mut row = vec![Cell::from(label.as_str())];
        row.extend(shown.iter().map(|l| val_of(k, l).map(Cell::Num).unwrap_or(Cell::from("—"))));
        row.extend([diff, Cell::from(status), Cell::from(head)]);
        t.row_tone(row, if tied.contains(&k) { Tone::Highlight } else if r.is_err() { Tone::Bad } else { Tone::Normal });
    }
    let mut ch = Chart::new(Some(&format!("{metric_label} según la variante")), "Variante", &metric_label);
    ch.categories = Some(rows.iter().map(|r| r.0.clone()).collect());
    ch = ch.series(Series::new(metric_label.clone(), SeriesKind::Bar, vals.iter().enumerate().filter_map(|(k, v)| v.map(|v| [k as f64, v])).collect()).tone(Tone::Accent));
    ch.y_zero = true;
    // cada variante: su solución y su primer gráfico
    let mut st = vec![];
    for (label, _, r) in &rows {
        let mut blocks = vec![];
        let mut desc = String::new();
        if let Ok(r) = r {
            desc = r["headline"].as_str().unwrap_or("").to_string();
            let secs = r["sections"].as_array().cloned().unwrap_or_default();
            if let Some(sol) = secs.first() {
                blocks.extend(sol["blocks"].as_array().cloned().unwrap_or_default());
            }
            if let Some(g) = secs.iter().skip(1).flat_map(|s| s["blocks"].as_array().cloned().unwrap_or_default()).find(|b| matches!(b["type"].as_str(), Some("network" | "chart" | "map" | "gantt" | "layout" | "tree"))) {
                blocks.push(g);
            }
        } else if let Err(e) = r {
            desc = e.clone();
        }
        st.push(json!({"title": label, "description": desc, "blocks": blocks}));
    }
    let headline = match best {
        Some(b) if tied.len() > 1 => format!("Se compararon {} variantes ({}): empatan {} con {} = {}.", rows.len(), what, tied.iter().map(|&k| rows[k].0.clone()).collect::<Vec<_>>().join(", "), lc(&metric_label), fmt(vals[b].unwrap())),
        Some(b) => format!("Se compararon {} variantes ({}): la mejor es {} con {} = {}.", rows.len(), what, rows[b].0, lc(&metric_label), fmt(vals[b].unwrap())),
        None => format!("Se compararon {} variantes ({}) por {}.", rows.len(), what, lc(&metric_label)),
    };
    let mut rep = Report::new(info.id, model.get("name").and_then(|v| v.as_str()).unwrap_or(""), Status::Solved, headline)
        .section("comparacion", "Comparación", {
            let mut v = vec![];
            if let Some(b) = best {
                v.push(metrics(vec![metric_main(&format!("Mejor {}: {}", lc(&metric_label), if tied.len() > 1 { "empate".to_string() } else { rows[b].0.clone() }), vals[b].unwrap()), metric("Variantes", rows.len() as f64)]));
            }
            v.push(t.block());
            v.push(ch.block());
            if best.is_none() {
                v.push(text("No se sabe si conviene el valor mayor o el menor de esta métrica, así que no se marca una mejor."));
            }
            v
        });
    // las fuentes de todas las variantes
    for (_, _, r) in &rows {
        if let Ok(r) = r {
            for c in r["references"].as_array().into_iter().flatten() {
                if let (Some(id), Some(uses)) = (c["id"].as_str(), c["used_for"].as_array()) {
                    for u in uses {
                        rep.add_cite(id, u.as_str().unwrap_or(""));
                    }
                }
            }
        }
    }
    let _ = Citation::new;
    let mut v = serde_json::to_value(rep.finish_sources()).unwrap();
    // la sección de variantes va antes de las fuentes
    let var_sec = json!({"id": "variantes", "title": "Variantes", "blocks": [{"type": "steps", "title": "Solución de cada variante", "steps": st}]});
    if let Some(secs) = v["sections"].as_array_mut() {
        let pos = secs.iter().position(|s| s["id"] == "fuentes").unwrap_or(secs.len());
        secs.insert(pos, var_sec);
    }
    v["data"] = json!({
        "compared": what,
        "metric": metric_label,
        "smaller_is_better": dir,
        "best": best.map(|b| rows[b].0.clone()),
        "tied": tied.iter().map(|&k| rows[k].0.clone()).collect::<Vec<_>>(),
        "variants": rows.iter().enumerate().map(|(k, (label, opts, r))| {
            let mut o = json!({"label": label, "options": opts, "value": vals[k], "metrics": all[k].iter().map(|(l, v)| (l.clone(), json!(v))).collect::<Map<_, _>>()});
            match r {
                Ok(r) => {
                    o["headline"] = r["headline"].clone();
                    o["status"] = r["status"].clone();
                    o["data"] = r["data"].clone();
                    if full {
                        o["report"] = r.clone();
                    }
                }
                Err(e) => o["error"] = json!(e),
            }
            o
        }).collect::<Vec<_>>(),
    });
    Ok(v)
}

/// Pone en minúscula la primera letra, salvo en siglas (MAD, Cmax se deja igual si la segunda es mayúscula).
fn lc(s: &str) -> String {
    let mut ch = s.chars();
    match (ch.next(), s.chars().nth(1)) {
        (Some(a), Some(b)) if b.is_lowercase() => a.to_lowercase().collect::<String>() + ch.as_str(),
        _ => s.to_string(),
    }
}

/// Estado del informe en español, para la tabla.
fn status_es(s: &str) -> &str {
    match s {
        "optimal" => "Óptimo",
        "feasible" => "Factible",
        "infeasible" => "Infactible",
        "unbounded" => "No acotado",
        "solved" => "Resuelto",
        other => other,
    }
}
