//! Excel: el modelo de cualquier módulo a un libro .xlsx (una hoja por tabla del
//! editor, como en la aplicación) y de vuelta, y los resultados de un informe a
//! un libro (una hoja por tabla). El libro lleva una hoja «IAQSB» con el modelo
//! completo, así que al abrirlo de nuevo no se pierde nada de lo que no es tabla;
//! lo que se cambió en las hojas visibles tiene prioridad.

use crate::{generic, i18n, template, ApiError, Result};
use iaqsb_core::i18n::Lang;
use iaqsb_core::spec::{Field, Widget};
use iaqsb_core::xlsx::{self, Cell, Sheet};
use serde_json::{json, Map, Value};

const META: &str = "IAQSB";
/// Largo máximo de una celda de Excel.
const CELL_MAX: usize = 30_000;

fn tr(s: &str, lang: Lang) -> String {
    i18n::translate(s, lang)
}

fn val_cell(v: &Value) -> Cell {
    match v {
        Value::Null => Cell::Empty,
        Value::Bool(b) => Cell::Bool(*b),
        Value::Number(n) => Cell::Num(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => Cell::Text(s.clone()),
        Value::Array(a) => Cell::Text(a.iter().map(|x| x.as_str().map(str::to_string).unwrap_or_else(|| x.to_string())).collect::<Vec<_>>().join(", ")),
        Value::Object(_) => Cell::Text(v.to_string()),
    }
}

/// Valor de una celda según el tipo del campo.
fn cell_val(c: &Cell, f: &Field) -> Value {
    cell_val_as(c, f, None)
}

/// Como [`cell_val`], conservando el tipo del valor anterior (una lista puede
/// guardarse como texto «A, B» o como arreglo).
fn cell_val_as(c: &Cell, f: &Field, old: Option<&Value>) -> Value {
    use iaqsb_core::spec::FieldKind as K;
    match (&f.kind, c) {
        (K::Text | K::Expr, Cell::Empty) => json!(""),
        (_, Cell::Empty) => Value::Null,
        (K::Number, c) => c.num().map(Value::from).unwrap_or(Value::Null),
        (K::Integer, c) => c.num().map(|x| Value::from(x.round() as i64)).unwrap_or(Value::Null),
        (K::Bool, Cell::Bool(b)) => Value::Bool(*b),
        (K::Bool, c) => {
            let t = c.text().to_lowercase();
            Value::Bool(matches!(t.trim(), "1" | "true" | "verdadero" | "sí" | "si" | "yes" | "sim" | "x" | "✓"))
        }
        (K::Bound, Cell::Num(x)) => Value::from(*x),
        (K::Bound, c) => {
            let t = c.text();
            let u = t.trim().to_uppercase();
            if u == "M" || u == "INF" || u == "∞" {
                json!("M")
            } else if u == "-M" || u == "−M" || u == "-INF" || u == "-∞" {
                json!("-M")
            } else {
                xlsx::parse_num(&t).map(Value::from).unwrap_or(json!(t))
            }
        }
        (K::List, c) if !old.is_some_and(|o| o.is_array()) => json!(c.text().trim()),
        (K::List, c) => Value::Array(c.text().split([',', ';']).map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| json!(s)).collect()),
        (K::Choice { options }, c) => {
            // el código o la etiqueta (en cualquier idioma)
            let t = c.text();
            let t = t.trim();
            options
                .iter()
                .find(|(code, label)| code == t || label.eq_ignore_ascii_case(t) || Lang::ALL.iter().any(|l| tr(label, *l).eq_ignore_ascii_case(t)))
                .map(|(code, _)| json!(code))
                .unwrap_or(json!(t))
        }
        (_, c) => json!(c.text()),
    }
}

/// ¿El encabezado nombra este campo? (por su clave o su etiqueta en cualquier idioma)
fn names_field(h: &str, f: &Field) -> bool {
    let h = h.trim();
    h.eq_ignore_ascii_case(&f.key) || h.eq_ignore_ascii_case(&f.label) || Lang::ALL.iter().any(|l| tr(&f.label, *l).eq_ignore_ascii_case(h))
}

fn widget_title(w: &Widget, k: usize, lang: Lang) -> String {
    let t = match w {
        Widget::Params { title, .. } => title.clone().unwrap_or_else(|| "Parámetros".into()),
        Widget::Records { title, path, .. } => title.clone().unwrap_or_else(|| path.clone()),
        Widget::Matrix { title, path, .. } => title.clone().unwrap_or_else(|| path.clone()),
    };
    let t = tr(&t, lang);
    if t.trim().is_empty() { format!("{} {}", tr("Tabla", lang), k + 1) } else { t }
}

/// Campos sin repetir clave (algunas hojas cambian de columnas según una opción).
fn dedup(fields: &[Field], model: &Value) -> Vec<Field> {
    let get = |k: &str| model.get(k).map(|v| v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())).unwrap_or_default();
    let mut out: Vec<Field> = vec![];
    for f in fields {
        let shown = f.show_if.iter().all(|(k, vals)| vals.contains(&get(k)));
        if let Some(p) = out.iter().position(|x| x.key == f.key) {
            if shown {
                out[p] = f.clone();
            }
        } else {
            out.push(f.clone());
        }
    }
    out
}

fn meta_sheet(module: &str, model: &Value) -> Sheet {
    let mut s = Sheet::new(META);
    s.rows.push(vec!["IAQSB".into(), "Modelo completo (no edites esta hoja; las demás sí)".into()]);
    s.rows.push(vec!["module".into(), module.into()]);
    let txt = model.to_string();
    let chars: Vec<char> = txt.chars().collect();
    for (i, chunk) in chars.chunks(CELL_MAX).enumerate() {
        s.rows.push(vec![if i == 0 { "model".into() } else { "".into() }, Cell::Text(chunk.iter().collect())]);
    }
    s
}

fn read_meta(sheets: &[Sheet]) -> Option<(String, Value)> {
    let s = sheets.iter().find(|s| s.name == META)?;
    let module = s.rows.iter().find(|r| r.first().map(|c| c.text()) == Some("module".into()))?.get(1)?.text();
    let start = s.rows.iter().position(|r| r.first().map(|c| c.text()) == Some("model".into()))?;
    let txt: String = s.rows[start..].iter().filter_map(|r| r.get(1)).map(|c| c.text()).collect();
    Some((module, serde_json::from_str(&txt).ok()?))
}

// ---------------------------------------------------------------- LP-ILP

fn lp_sheet(model: &Value, lang: Lang) -> Sheet {
    let vars = model["variables"].as_array().cloned().unwrap_or_default();
    let cons = model["constraints"].as_array().cloned().unwrap_or_default();
    let n = vars.len();
    let mut s = Sheet::new(&tr("Modelo", lang));
    s.header_rows = 1;
    let mut h: Vec<Cell> = vec![Cell::Text(tr("Variable", lang))];
    h.extend(vars.iter().map(|v| val_cell(&v["name"])));
    h.push(Cell::Text(tr("Sentido", lang)));
    h.push(Cell::Text(tr("Lado derecho", lang)));
    s.rows.push(h);
    let mut obj: Vec<Cell> = vec![Cell::Text(if model["objective"] == "min" { tr("Minimizar", lang) } else { tr("Maximizar", lang) })];
    obj.extend(vars.iter().map(|v| val_cell(&v["cost"])));
    s.rows.push(obj);
    for c in &cons {
        let mut r: Vec<Cell> = vec![val_cell(&c["name"])];
        let coefs = c["coefs"].as_array().cloned().unwrap_or_default();
        for j in 0..n {
            r.push(coefs.get(j).map(val_cell).unwrap_or(Cell::Num(0.0)));
        }
        r.push(val_cell(&c["direction"]));
        r.push(val_cell(&c["rhs"]));
        s.rows.push(r);
    }
    let bound = |v: &Value, k: &str, def: &str| -> Cell {
        match v.get(k) {
            Some(Value::Number(x)) => Cell::Num(x.as_f64().unwrap_or(0.0)),
            Some(Value::String(t)) => Cell::Text(t.clone()),
            _ => Cell::Text(def.into()),
        }
    };
    let mut lo: Vec<Cell> = vec![Cell::Text(tr("Cota inferior", lang))];
    lo.extend(vars.iter().map(|v| if v.get("lower").is_none() { Cell::Num(0.0) } else { bound(v, "lower", "0") }));
    let mut hi: Vec<Cell> = vec![Cell::Text(tr("Cota superior", lang))];
    hi.extend(vars.iter().map(|v| bound(v, "upper", "M")));
    let mut ty: Vec<Cell> = vec![Cell::Text(tr("Tipo", lang))];
    ty.extend(vars.iter().map(|v| Cell::Text(v["type"].as_str().unwrap_or("continuous").into())));
    s.rows.push(vec![]);
    s.rows.push(lo);
    s.rows.push(hi);
    s.rows.push(ty);
    s
}

fn is_dir(c: &Cell) -> Option<&'static str> {
    match c.text().trim().replace('≤', "<=").replace('≥', ">=").as_str() {
        "<=" | "=<" | "<" => Some("<="),
        ">=" | "=>" | ">" => Some(">="),
        "=" | "==" => Some("="),
        _ => None,
    }
}

fn row_is(label: &Cell, es: &str) -> bool {
    let t = label.text();
    let t = t.trim();
    t.eq_ignore_ascii_case(es) || Lang::ALL.iter().any(|l| tr(es, *l).eq_ignore_ascii_case(t))
}

/// Lee la hoja del modelo de LP-ILP (la forma de WinQSB: variables en columnas).
fn lp_from_sheet(s: &Sheet, base: Option<&Value>) -> Result<Value> {
    let rows = s.trimmed();
    let head = rows.first().ok_or_else(|| ApiError::BadInput("la hoja del modelo está vacía".into()))?;
    // columnas de variables: entre la primera y la de «Sentido» (o la penúltima)
    let dir_col = rows.iter().skip(1).filter_map(|r| r.iter().position(|c| is_dir(c).is_some())).max().ok_or_else(|| ApiError::BadInput("no se encontró la columna del sentido (<=, >=, =) de las restricciones".into()))?;
    let n = dir_col - 1;
    let names: Vec<String> = (1..=n).map(|j| head.get(j).map(|c| c.text()).filter(|t| !t.trim().is_empty()).unwrap_or_else(|| format!("X{j}"))).collect();
    let obj_row = rows.get(1).ok_or_else(|| ApiError::BadInput("falta la fila del objetivo".into()))?;
    let lower = obj_row[0].text().to_lowercase();
    let objective = if ["min", "minimizar", "minimize", "minimise", "minimizar"].iter().any(|w| lower.starts_with(w)) || row_is(&obj_row[0], "Minimizar") { "min" } else { "max" };
    let mut variables: Vec<Value> = names.iter().enumerate().map(|(j, nm)| json!({"name": nm, "cost": obj_row.get(j + 1).and_then(|c| c.num()).unwrap_or(0.0)})).collect();
    let mut constraints = vec![];
    for r in rows.iter().skip(2) {
        let Some(first) = r.first() else { continue };
        if let Some(d) = r.get(dir_col).and_then(is_dir) {
            let coefs: Vec<f64> = (1..=n).map(|j| r.get(j).and_then(|c| c.num()).unwrap_or(0.0)).collect();
            let rhs = r.get(dir_col + 1).and_then(|c| c.num()).unwrap_or(0.0);
            let name = if first.is_empty() { format!("C{}", constraints.len() + 1) } else { first.text() };
            constraints.push(json!({"name": name, "coefs": coefs, "direction": d, "rhs": rhs}));
            continue;
        }
        let key = if row_is(first, "Cota inferior") || first.text().to_lowercase().starts_with("lower") {
            "lower"
        } else if row_is(first, "Cota superior") || first.text().to_lowercase().starts_with("upper") {
            "upper"
        } else if row_is(first, "Tipo") || first.text().eq_ignore_ascii_case("type") {
            "type"
        } else {
            continue;
        };
        for (j, v) in variables.iter_mut().enumerate() {
            let c = r.get(j + 1).cloned().unwrap_or(Cell::Empty);
            if c.is_empty() {
                continue;
            }
            v[key] = if key == "type" {
                let t = c.text().to_lowercase();
                json!(match t.trim() {
                    x if x.starts_with("ent") || x.starts_with("int") || x.starts_with("inteir") => "integer",
                    x if x.starts_with("bin") => "binary",
                    x if x.starts_with("lib") || x.starts_with("unres") || x.starts_with("free") || x.starts_with("livre") => "unrestricted",
                    x if x.starts_with("semicont") => "semicontinuous",
                    x if x.starts_with("semient") || x.starts_with("semiint") || x.starts_with("semiinteir") => "semiinteger",
                    _ => "continuous",
                })
            } else {
                cell_val(&c, &Field::bound(key, key, Value::Null))
            };
        }
    }
    let name = base.and_then(|b| b.get("name")).cloned().unwrap_or(json!(s.name));
    let mut m = base.cloned().unwrap_or(json!({}));
    m["name"] = name;
    m["objective"] = json!(objective);
    m["variables"] = Value::Array(variables);
    m["constraints"] = Value::Array(constraints);
    if let Some(o) = m.as_object_mut() {
        o.remove("objectives");
    }
    Ok(m)
}

// ---------------------------------------------------------------- módulos genéricos

fn layout_of(module: &str, model: &Value) -> Result<Vec<Widget>> {
    let g = generic(module).ok_or_else(|| ApiError::BadInput(format!("módulo desconocido: {module}")))?;
    let spec = g.spec();
    let pt = model.get("problem_type").and_then(|v| v.as_str()).unwrap_or("");
    let p = spec.problem_types.iter().find(|p| p.id == pt).or(spec.problem_types.first()).ok_or_else(|| ApiError::BadInput("el módulo no tiene tipos de problema".into()))?;
    Ok(p.layout.clone())
}

fn generic_sheets(module: &str, model: &Value, lang: Lang) -> Result<Vec<Sheet>> {
    let mut out = vec![];
    for (k, w) in layout_of(module, model)?.iter().enumerate() {
        let mut s = Sheet::new(&widget_title(w, k, lang));
        s.header_rows = 1;
        match w {
            Widget::Params { path, fields, .. } => {
                let obj = path.as_ref().map(|p| &model[p.as_str()]).unwrap_or(model);
                s.rows.push(vec![Cell::Text(tr("Dato", lang)), Cell::Text(tr("Valor", lang)), Cell::Text(tr("Clave", lang))]);
                for f in dedup(fields, model) {
                    s.rows.push(vec![Cell::Text(tr(&f.label, lang)), val_cell(&obj[f.key.as_str()]), Cell::Text(f.key.clone())]);
                }
            }
            Widget::Records { path, columns, .. } => {
                let cols = dedup(columns, model);
                s.rows.push(cols.iter().map(|f| Cell::Text(tr(&f.label, lang))).collect());
                for rec in model[path.as_str()].as_array().into_iter().flatten() {
                    s.rows.push(cols.iter().map(|f| val_cell(&rec[f.key.as_str()])).collect());
                }
            }
            Widget::Matrix { path, rows, cols, .. } => {
                let rl = model[rows.path.as_str()].as_array().cloned().unwrap_or_default();
                let cl = model[cols.path.as_str()].as_array().cloned().unwrap_or_default();
                let mtx = model[path.as_str()].as_array().cloned().unwrap_or_default();
                let mut h: Vec<Cell> = vec![Cell::Text(format!("{} \\ {}", tr(&rows.label, lang), tr(&cols.label, lang)))];
                h.extend(cl.iter().enumerate().map(|(j, c)| val_cell(&[cols.label_key.as_str(), "name", "id", "label"].iter().map(|k| &c[*k]).find(|v| !v.is_null()).cloned().unwrap_or(json!(j + 1)))));
                h.extend(rows.extra.iter().map(|f| Cell::Text(tr(&f.label, lang))));
                s.rows.push(h);
                for (i, r) in rl.iter().enumerate() {
                    let lab = [rows.label_key.as_str(), "name", "id", "label"].iter().map(|k| &r[*k]).find(|v| !v.is_null()).cloned().unwrap_or(json!(i + 1));
                    let mut row: Vec<Cell> = vec![val_cell(&lab)];
                    let line = mtx.get(i).and_then(|x| x.as_array()).cloned().unwrap_or_default();
                    row.extend((0..cl.len()).map(|j| line.get(j).map(val_cell).unwrap_or(Cell::Empty)));
                    row.extend(rows.extra.iter().map(|f| val_cell(&r[f.key.as_str()])));
                    s.rows.push(row);
                }
                for f in &cols.extra {
                    let mut row: Vec<Cell> = vec![Cell::Text(tr(&f.label, lang))];
                    row.extend(cl.iter().map(|c| val_cell(&c[f.key.as_str()])));
                    s.rows.push(row);
                }
            }
        }
        out.push(s);
    }
    Ok(out)
}

fn find_sheet<'a>(sheets: &'a [Sheet], title: &str, k: usize) -> Option<&'a Sheet> {
    let visible: Vec<&Sheet> = sheets.iter().filter(|s| s.name != META).collect();
    let clean = |s: &str| s.chars().filter(|c| !":\\/?*[]".contains(*c)).collect::<String>().trim().chars().take(31).collect::<String>().to_lowercase();
    let want: Vec<String> = Lang::ALL.iter().map(|l| clean(&tr(title, *l))).collect();
    visible.iter().find(|s| want.contains(&clean(&s.name))).copied().or_else(|| visible.get(k).copied())
}

fn generic_from_sheets(module: &str, sheets: &[Sheet], mut model: Value) -> Result<Value> {
    let layout = layout_of(module, &model)?;
    for (k, w) in layout.iter().enumerate() {
        let title = match w {
            Widget::Params { title, .. } => title.clone().unwrap_or_else(|| "Parámetros".into()),
            Widget::Records { title, path, .. } | Widget::Matrix { title, path, .. } => title.clone().unwrap_or_else(|| path.clone()),
        };
        let Some(s) = find_sheet(sheets, &title, k) else { continue };
        let rows = s.trimmed();
        if rows.is_empty() {
            continue;
        }
        match w {
            Widget::Params { path, fields, .. } => {
                let fields = dedup(fields, &model);
                // un grupo opcional que no existía y sigue vacío no se crea
                if let Some(p) = path {
                    let absent = model.get(p.as_str()).map_or(true, |v| v.is_null());
                    let empty = rows.iter().skip(1).all(|r| r.get(1).map_or(true, |c| c.is_empty()));
                    if absent && empty {
                        continue;
                    }
                }
                for r in rows.iter().skip(1) {
                    let (Some(label), Some(v)) = (r.first(), r.get(1)) else { continue };
                    let key_cell = r.get(2).map(|c| c.text()).unwrap_or_default();
                    let Some(f) = fields.iter().find(|f| f.key == key_cell.trim() || names_field(&label.text(), f)) else { continue };
                    let prev = match path {
                        Some(p) => model[p.as_str()].get(&f.key).cloned(),
                        None => model.get(&f.key).cloned(),
                    };
                    let val = cell_val_as(v, f, prev.as_ref());
                    if val.is_null() && !f.optional {
                        continue;
                    }
                    let target = match path {
                        Some(p) => {
                            if !model[p.as_str()].is_object() {
                                model[p.as_str()] = json!({});
                            }
                            &mut model[p.as_str()]
                        }
                        None => &mut model,
                    };
                    target[f.key.as_str()] = val;
                }
            }
            Widget::Records { path, columns, .. } => {
                let cols = dedup(columns, &model);
                let head = &rows[0];
                let map: Vec<Option<&Field>> = head.iter().map(|h| cols.iter().find(|f| names_field(&h.text(), f))).collect();
                let old = model[path.as_str()].as_array().cloned().unwrap_or_default();
                let mut list = vec![];
                for (i, r) in rows.iter().skip(1).enumerate() {
                    if r.iter().all(|c| c.is_empty()) {
                        continue;
                    }
                    // se parte del registro anterior (conserva campos que no son columnas)
                    let mut rec = old.get(i).cloned().filter(|v| v.is_object()).unwrap_or_else(|| Value::Object(Map::new()));
                    for (c, f) in r.iter().zip(&map) {
                        if let Some(f) = f {
                            let v = cell_val_as(c, f, rec.get(&f.key));
                            if !v.is_null() || f.optional {
                                rec[f.key.as_str()] = v;
                            }
                        }
                    }
                    for f in &cols {
                        if rec.get(&f.key).is_none() && !f.default.is_null() {
                            rec[f.key.as_str()] = f.default.clone();
                        }
                    }
                    list.push(rec);
                }
                model[path.as_str()] = Value::Array(list);
            }
            Widget::Matrix { path, rows: ra, cols: ca, cell, .. } => {
                // las listas de filas y columnas que también edita una hoja de registros
                // (p. ej. los artículos del MRP) no cambian de largo aquí
                let owned = |p: &str| !layout.iter().any(|w| matches!(w, Widget::Records { path, .. } if path == p));
                let head = &rows[0];
                let col_extra_rows: Vec<(usize, &Field)> = rows.iter().enumerate().skip(1).filter_map(|(i, r)| r.first().and_then(|c| ca.extra.iter().find(|f| names_field(&c.text(), f))).map(|f| (i, f))).collect();
                let data_rows: Vec<&Vec<Cell>> = rows.iter().enumerate().skip(1).filter(|(i, _)| !col_extra_rows.iter().any(|(k, _)| k == i)).map(|(_, r)| r).collect();
                let old_rows = model[ra.path.as_str()].as_array().cloned().unwrap_or_default();
                let old_cols = model[ca.path.as_str()].as_array().cloned().unwrap_or_default();
                let square = ra.path == ca.path;
                let ncol = if owned(&ca.path) && !square { head.len().saturating_sub(1 + ra.extra.len()) } else { old_cols.len() };
                let nrow = if owned(&ra.path) { data_rows.len().max(if square { ncol } else { 0 }) } else { old_rows.len() };
                let nrow = if square { ncol.max(nrow.min(ncol.max(1))) } else { nrow };
                let label = |o: &mut Value, key: &str, c: Option<&Cell>, own: bool| {
                    if let Some(c) = c.filter(|c| !c.is_empty()) {
                        if own || o.get(key).is_some() {
                            o[key] = json!(c.text());
                        }
                    }
                };
                let mut cl: Vec<Value> = (0..ncol)
                    .map(|j| {
                        let mut o = old_cols.get(j).cloned().filter(|v| v.is_object()).unwrap_or_else(|| json!({ (ca.label_key.as_str()): format!("{} {}", ca.label, j + 1) }));
                        label(&mut o, &ca.label_key, head.get(j + 1), owned(&ca.path));
                        for f in &ca.extra {
                            if o.get(&f.key).is_none() {
                                o[f.key.as_str()] = f.default.clone();
                            }
                        }
                        o
                    })
                    .collect();
                for (i, f) in &col_extra_rows {
                    for (j, c) in cl.iter_mut().enumerate() {
                        let v = cell_val_as(rows[*i].get(j + 1).unwrap_or(&Cell::Empty), f, c.get(&f.key));
                        if !v.is_null() || f.optional {
                            c[f.key.as_str()] = v;
                        }
                    }
                }
                let mut rl = vec![];
                let mut mtx = vec![];
                let empty: Vec<Cell> = vec![];
                for i in 0..nrow {
                    let r = data_rows.get(i).copied().unwrap_or(&empty);
                    let mut o = old_rows.get(i).cloned().filter(|v| v.is_object()).unwrap_or_else(|| json!({ (ra.label_key.as_str()): format!("{} {}", ra.label, i + 1) }));
                    label(&mut o, &ra.label_key, r.first(), owned(&ra.path));
                    for (k, f) in ra.extra.iter().enumerate() {
                        let v = cell_val_as(r.get(1 + ncol + k).unwrap_or(&Cell::Empty), f, o.get(&f.key));
                        if !v.is_null() || f.optional {
                            o[f.key.as_str()] = v;
                        }
                    }
                    rl.push(o);
                    mtx.push(Value::Array((0..ncol).map(|j| cell_val(r.get(j + 1).unwrap_or(&Cell::Empty), cell)).collect()));
                }
                if square {
                    model[ra.path.as_str()] = Value::Array(rl);
                } else {
                    model[ra.path.as_str()] = Value::Array(rl);
                    model[ca.path.as_str()] = Value::Array(cl);
                }
                model[path.as_str()] = Value::Array(mtx);
            }
        }
    }
    Ok(model)
}

// ---------------------------------------------------------------- API

/// El modelo de un módulo como libro .xlsx.
pub fn model_to_xlsx(module: &str, model: Value, lang: Lang) -> Result<Vec<u8>> {
    let mut sheets = if module == "lp" {
        // un modelo en texto (MPS/LP) se pasa primero a la hoja
        let m = if model.get("format").is_some() { crate::convert_lp(model.clone(), "json")?["model"].clone() } else { model.clone() };
        vec![lp_sheet(&m, lang)]
    } else {
        generic_sheets(module, &model, lang)?
    };
    sheets.push(meta_sheet(module, &model));
    Ok(xlsx::write(&sheets))
}

/// Un libro .xlsx como modelo. Si el libro lo hizo IAQSB, trae el módulo; si no,
/// hay que decir de qué módulo es (y, si se quiere, el tipo de problema).
pub fn xlsx_to_model(bytes: &[u8], module: Option<&str>, problem_type: Option<&str>) -> Result<Value> {
    let sheets = xlsx::read(bytes).map_err(ApiError::BadInput)?;
    let meta = read_meta(&sheets);
    let module = module.map(str::to_string).or_else(|| meta.as_ref().map(|m| m.0.clone())).ok_or_else(|| {
        ApiError::BadInput("el libro no lo hizo IAQSB: indica el módulo (module) para leerlo".into())
    })?;
    let base = meta.filter(|m| m.0 == module).map(|m| m.1);
    let model = if module == "lp" {
        let s = sheets.iter().find(|s| s.name != META).ok_or_else(|| ApiError::BadInput("el libro no tiene hojas".into()))?;
        lp_from_sheet(s, base.as_ref())?
    } else {
        let start = match base {
            Some(b) => b,
            None => {
                let g = generic(&module).ok_or_else(|| ApiError::BadInput(format!("módulo desconocido: {module}")))?;
                let pt = problem_type.map(str::to_string).unwrap_or_else(|| g.spec().problem_types.first().map(|p| p.id.clone()).unwrap_or_default());
                template(&module, &pt, &Value::Null)?
            }
        };
        generic_from_sheets(&module, &sheets, start)?
    };
    Ok(json!({"module": module, "layout": "matrix", "model": model}))
}

fn cell_of(v: &Value) -> Cell {
    match v {
        Value::Number(n) => Cell::Num(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => {
            // números con signo «−» de los informes
            let t = s.replace('−', "-");
            if let Ok(x) = t.trim().parse::<f64>() { Cell::Num(x) } else { Cell::Text(s.clone()) }
        }
        Value::Null => Cell::Empty,
        other => val_cell(other),
    }
}

/// Las tablas de un informe (de cualquier módulo) como libro .xlsx: una hoja de
/// resumen y una por tabla. Con `steps`, también las del paso a paso.
pub fn report_to_xlsx(report: &Value, steps: bool, lang: Lang) -> Vec<u8> {
    let doc = iaqsb_render::export::document(report, &iaqsb_render::export::Options { steps, ..Default::default() }, &|s| tr(s, lang));
    let mut sheets = vec![];
    let mut sum = Sheet::new(&tr("Resumen", lang));
    sum.rows.push(vec![Cell::Text(doc.title.clone())]);
    if !doc.subtitle.is_empty() {
        sum.rows.push(vec![Cell::Text(doc.subtitle.clone())]);
    }
    for b in &doc.blocks {
        match b {
            iaqsb_render::export::Item::Para(t) if sum.rows.len() < 60 => sum.rows.push(vec![Cell::Text(iaqsb_core::biblio::plain(t))]),
            iaqsb_render::export::Item::Metrics(items) => {
                sum.rows.push(vec![]);
                for (k, v) in items {
                    sum.rows.push(vec![Cell::Text(k.clone()), cell_of(&json!(v))]);
                }
            }
            iaqsb_render::export::Item::Table(t) if sheets.len() < 120 => {
                let mut s = Sheet::new(if t.title.is_empty() { "Tabla" } else { &t.title });
                s.header_rows = 1;
                s.rows.push(t.header.iter().map(|h| Cell::Text(h.clone())).collect());
                for r in &t.rows {
                    s.rows.push(r.iter().map(|c| cell_of(&json!(c))).collect());
                }
                if !t.note.is_empty() {
                    s.rows.push(vec![]);
                    s.rows.push(vec![Cell::Text(t.note.clone())]);
                }
                sheets.push(s);
            }
            _ => {}
        }
    }
    if !doc.references.is_empty() {
        let mut r = Sheet::new(&tr("Referencias", lang));
        r.header_rows = 1;
        r.rows.push(vec![Cell::Text(tr("Referencia (APA 7)", lang))]);
        for a in &doc.references {
            r.rows.push(vec![Cell::Text(iaqsb_core::biblio::plain(a))]);
        }
        sheets.push(r);
    }
    sheets.insert(0, sum);
    xlsx::write(&sheets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lp_ida_y_vuelta() {
        let m = crate::example_model("lp").unwrap();
        let b = model_to_xlsx("lp", m.clone(), Lang::Es).unwrap();
        let back = xlsx_to_model(&b, None, None).unwrap();
        assert_eq!(back["module"], "lp");
        let (a, b) = (m["variables"].as_array().unwrap(), back["model"]["variables"].as_array().unwrap());
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b) {
            assert_eq!(x["name"], y["name"]);
            assert_eq!(x["cost"].as_f64(), y["cost"].as_f64());
        }
        let r1 = crate::solve("lp", m.clone(), None).unwrap();
        let r2 = crate::solve("lp", back["model"].clone(), None).unwrap();
        assert_eq!(r1["objective_value"], r2["objective_value"]);
        assert_eq!(back["model"]["constraints"].as_array().unwrap().len(), m["constraints"].as_array().unwrap().len());
    }

    #[test]
    fn libro_ajeno_de_lp_en_ingles() {
        let mut s = Sheet::new("Sheet1");
        s.rows.push(vec!["".into(), "x".into(), "y".into(), "Direction".into(), "RHS".into()]);
        s.rows.push(vec!["Maximize".into(), 3.0.into(), 5.0.into()]);
        s.rows.push(vec!["c1".into(), 1.0.into(), 0.0.into(), "<=".into(), 4.0.into()]);
        s.rows.push(vec!["c2".into(), 0.0.into(), 2.0.into(), "≤".into(), 12.0.into()]);
        s.rows.push(vec!["c3".into(), 3.0.into(), 2.0.into(), "<=".into(), 18.0.into()]);
        s.rows.push(vec!["Type".into(), "Integer".into(), "continuous".into()]);
        let b = xlsx::write(&[s]);
        let v = xlsx_to_model(&b, Some("lp"), None).unwrap();
        assert_eq!(v["model"]["variables"][0]["type"], "integer");
        let r = crate::solve("lp", v["model"].clone(), None).unwrap();
        assert!((r["objective_value"].as_f64().unwrap() - 36.0).abs() < 1e-6);
    }

    #[test]
    fn todos_los_ejemplos_por_excel() {
        for (m, name, model) in crate::examples() {
            let b = model_to_xlsx(&m, model.clone(), Lang::Es).unwrap_or_else(|e| panic!("{m}/{name}: {e}"));
            let back = xlsx_to_model(&b, None, None).unwrap_or_else(|e| panic!("{m}/{name}: {e}"));
            let a = crate::solve(&m, model.clone(), None).unwrap();
            let r = crate::solve(&m, back["model"].clone(), None).unwrap_or_else(|e| panic!("{m}/{name}: {e}"));
            assert_eq!(a["status"], r["status"], "{m}/{name}");
            assert_eq!(a.get("objective_value"), r.get("objective_value"), "{m}/{name}");
            assert_eq!(a.get("headline"), r.get("headline"), "{m}/{name}");
        }
    }

    #[test]
    fn libro_sin_hoja_iaqsb_de_un_modulo() {
        // se exporta, se borra la hoja IAQSB y se lee indicando el módulo
        let (m, _, model) = crate::examples().into_iter().find(|(m, n, _)| m == "net" && n == "transporte").unwrap();
        let b = model_to_xlsx(&m, model.clone(), Lang::En).unwrap();
        let mut sheets = xlsx::read(&b).unwrap();
        sheets.retain(|s| s.name != META);
        let b2 = xlsx::write(&sheets);
        let pt = model["problem_type"].as_str().unwrap();
        let back = xlsx_to_model(&b2, Some("net"), Some(pt)).unwrap();
        let a = crate::solve("net", model, None).unwrap();
        let r = crate::solve("net", back["model"].clone(), None).unwrap();
        assert_eq!(a["headline"], r["headline"]);
    }

    #[test]
    fn lo_editado_en_la_hoja_manda() {
        // se cambia la oferta del primer origen y un costo en la hoja visible
        let (m, _, model) = crate::examples().into_iter().find(|(m, n, _)| m == "net" && n == "transporte").unwrap();
        let b = model_to_xlsx(&m, model.clone(), Lang::Es).unwrap();
        let mut sheets = xlsx::read(&b).unwrap();
        let k = sheets.iter().position(|s| s.name != META && s.rows.len() > 2).unwrap();
        let before = sheets[k].rows[1][1].clone();
        assert!(before.num().unwrap_or(0.0) > 0.0);
        sheets[k].rows[1][1] = Cell::Num(0.0);
        let back = xlsx_to_model(&xlsx::write(&sheets), None, None).unwrap();
        assert_ne!(back["model"], model);
        let a = crate::solve("net", model, None).unwrap();
        let r = crate::solve("net", back["model"].clone(), None).unwrap();
        assert_ne!(a["headline"], r["headline"]);
    }

    #[test]
    fn resultados_a_excel() {
        let m = crate::example_model("lp").unwrap();
        let r = crate::solve("lp", m, None).unwrap();
        let b = report_to_xlsx(&r, false, Lang::Pt);
        let sheets = xlsx::read(&b).unwrap();
        assert!(sheets.len() >= 3, "{}", sheets.len());
    }
}
