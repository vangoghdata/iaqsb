//! Informes para entregar (Word, PDF, HTML, Excel o Markdown) y utilidades de la
//! API: base64, fecha de hoy y la preparación en segundo plano de las traducciones.

use crate::{i18n, ApiError, Result};
use iaqsb_core::i18n::{translate_value, Lang};
use iaqsb_render::export;
use serde_json::{json, Value};

/// Decodifica base64 (estándar, con o sin relleno; ignora espacios y saltos).
pub fn base64_decode(s: &str) -> Result<Vec<u8>> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a' + 26) as u32,
            b'0'..=b'9' => (c - b'0' + 52) as u32,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        })
    };
    // admite «data:…;base64,» al principio
    let s = s.split_once("base64,").map(|x| x.1).unwrap_or(s);
    let clean: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=').collect();
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    for ch in clean.chunks(4) {
        let mut n = 0u32;
        for (i, c) in ch.iter().enumerate() {
            n |= val(*c).ok_or_else(|| ApiError::BadInput("base64 no válido".into()))? << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if ch.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if ch.len() > 3 {
            out.push(n as u8);
        }
    }
    Ok(out)
}

/// Fecha de hoy escrita en el idioma («21 de octubre de 2026», «October 21, 2026»).
pub fn today(lang: Lang) -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    // días desde 1970 → fecha civil (algoritmo de Howard Hinnant)
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    const ES: [&str; 12] = ["enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre"];
    const EN: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    const PT: [&str; 12] = ["janeiro", "fevereiro", "março", "abril", "maio", "junho", "julho", "agosto", "setembro", "outubro", "novembro", "dezembro"];
    let k = (m - 1) as usize;
    match lang {
        Lang::En => format!("{} {d}, {y}", EN[k]),
        Lang::Pt => format!("{d} de {} de {y}", PT[k]),
        Lang::Es => format!("{d} de {} de {y}", ES[k]),
    }
}

/// Prepara en segundo plano los catálogos y los textos traducidos que más se
/// piden (módulos, ejemplos y especificaciones): después, cambiar de idioma es
/// inmediato.
pub fn warm_up() {
    i18n::warm_up();
    for l in ["en", "pt"] {
        let _ = crate::call("list_modules", json!({ "lang": l }));
        let _ = crate::call("examples", json!({ "lang": l }));
        if let Ok(v) = crate::call("list_modules", json!({})) {
            for m in v["modules"].as_array().into_iter().flatten() {
                if m["available"].as_bool() == Some(true) {
                    if let Some(id) = m["id"].as_str() {
                        let _ = crate::call("spec", json!({ "module": id, "lang": l }));
                    }
                }
            }
        }
    }
}

/// Formatos de informe.
pub const FORMATS: &[&str] = &["docx", "pdf", "html", "xlsx", "md"];

/// Informe de un problema para entregar. `args`: module, report (ya en el idioma
/// pedido) o model (+ options) para resolverlo aquí, format (docx, pdf, html,
/// xlsx, md), steps, author, course, institution, date, decimal_comma.
pub fn export_report(args: &Value, lang: Lang) -> Result<Value> {
    let s = |k: &str| args.get(k).and_then(|v| v.as_str()).map(str::to_string);
    let module = s("module").unwrap_or_else(|| "lp".into());
    let format = s("format").unwrap_or_else(|| "docx".into()).trim_start_matches('.').to_lowercase();
    if !FORMATS.contains(&format.as_str()) {
        return Err(ApiError::BadInput(format!("formato de informe desconocido: {format} (usa docx, pdf, html, xlsx o md)")));
    }
    let steps = args.get("steps").and_then(|v| v.as_bool()).unwrap_or(false);
    let model = args.get("model").cloned();
    let report = match args.get("report").filter(|r| r.is_object()) {
        Some(r) => r.clone(),
        None => {
            let m = model.clone().ok_or_else(|| ApiError::BadInput("falta report (o model para resolverlo)".into()))?;
            let mut opts = args.get("options").cloned().unwrap_or(json!({}));
            if steps && module == "lp" {
                opts["trace"] = json!(true);
                opts["bnb_trace"] = json!(true);
            }
            if steps && module != "lp" {
                opts["steps"] = json!(true);
            }
            let mut r = crate::solve(&module, m, Some(opts))?;
            translate_value(&mut r, lang);
            r
        }
    };
    let tr = |x: &str| i18n::translate(x, lang);
    let module_name = crate::list_modules()["modules"]
        .as_array()
        .and_then(|a| a.iter().find(|m| m["id"] == module.as_str()).and_then(|m| m["name"].as_str()).map(|n| tr(n)))
        .unwrap_or_default();
    // en el planteamiento, el modelo de LP-ILP en la hoja (si vino como texto MPS/LP, se pasa)
    let lp_model = if module == "lp" {
        model.as_ref().and_then(|m| if m.get("format").is_some() { crate::convert_lp(m.clone(), "json").ok().map(|v| v["model"].clone()) } else { Some(m.clone()) })
    } else {
        None
    };
    let opts = export::Options {
        steps,
        author: s("author").unwrap_or_default(),
        course: s("course").unwrap_or_default(),
        institution: s("institution").unwrap_or_default(),
        date: s("date").unwrap_or_else(|| today(lang)),
        module_name,
        model: lp_model,
        decimal_comma: args.get("decimal_comma").and_then(|v| v.as_bool()).unwrap_or(false),
        lang: lang.code().into(),
        version: crate::VERSION.into(),
    };
    let base = sanitize(report.get("problem").and_then(|p| p.as_str()).unwrap_or("informe"));
    if format == "xlsx" {
        let b = crate::excel::report_to_xlsx(&report, steps, lang);
        return Ok(file(&b, &format!("{base}.xlsx"), "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"));
    }
    if format == "md" {
        let md = iaqsb_render::digest::markdown(&report, &iaqsb_render::digest::DigestOpts { detail: if steps { iaqsb_render::digest::Detail::Full } else { iaqsb_render::digest::Detail::Summary }, max_rows: 200, report_id: None });
        let md = i18n::translate_markdown(&md, lang);
        return Ok(json!({ "content": md, "filename": format!("{base}.md"), "mime": "text/markdown" }));
    }
    let mut doc = export::document(&report, &opts, &tr);
    // los textos de las figuras en el idioma del informe
    for b in &mut doc.blocks {
        if let export::Item::Figure { svg, .. } = b {
            *svg = i18n::translate_svg(svg, lang);
        }
    }
    match format.as_str() {
        "docx" => {
            let b = export::docx(&doc, &tr).map_err(ApiError::BadInput)?;
            Ok(file(&b, &format!("{base}.docx"), "application/vnd.openxmlformats-officedocument.wordprocessingml.document"))
        }
        "pdf" => {
            let b = export::pdf(&doc, &tr).map_err(ApiError::BadInput)?;
            Ok(file(&b, &format!("{base}.pdf"), "application/pdf"))
        }
        _ => {
            let h = export::html(&doc, &tr);
            Ok(json!({ "content": h, "filename": format!("{base}.html"), "mime": "text/html" }))
        }
    }
}

fn file(bytes: &[u8], name: &str, mime: &str) -> Value {
    json!({ "base64": iaqsb_render::base64(bytes), "filename": name, "mime": mime, "bytes": bytes.len() })
}

/// Nombre de archivo sin caracteres problemáticos.
pub fn sanitize(name: &str) -> String {
    let s: String = name.chars().map(|c| if c.is_alphanumeric() || " -_.()".contains(c) { c } else { ' ' }).collect();
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.is_empty() { "informe".into() } else { s.chars().take(80).collect() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_ida_y_vuelta() {
        for data in [&b""[..], b"a", b"ab", b"abc", b"\x00\xffhola mundo\x10"] {
            let e = iaqsb_render::base64(data);
            assert_eq!(base64_decode(&e).unwrap(), data);
        }
    }

    #[test]
    fn fecha_de_hoy() {
        let d = today(Lang::Es);
        assert!(d.contains(" de 20"), "{d}");
        assert!(today(Lang::En).contains(", 20"));
    }

    #[test]
    fn informes_de_todos_los_formatos() {
        let m = crate::example_model("lp").unwrap();
        for f in FORMATS {
            for (lang, l) in [("es", Lang::Es), ("en", Lang::En), ("pt", Lang::Pt)] {
                let v = export_report(&json!({"module": "lp", "model": m, "format": f, "steps": true, "author": "Grupo 8", "lang": lang}), l).unwrap_or_else(|e| panic!("{f}/{lang}: {e}"));
                let bytes = match v.get("base64") {
                    Some(b) => base64_decode(b.as_str().unwrap()).unwrap(),
                    None => v["content"].as_str().unwrap().as_bytes().to_vec(),
                };
                assert!(bytes.len() > 500, "{f}/{lang}: {}", bytes.len());
                match *f {
                    "pdf" => assert!(bytes.starts_with(b"%PDF")),
                    "docx" | "xlsx" => assert!(iaqsb_core::zip::is_zip(&bytes)),
                    "html" => assert!(String::from_utf8_lossy(&bytes).contains(&format!("lang=\"{lang}\""))),
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn informes_de_todos_los_ejemplos_en_pdf_y_word() {
        // cada ejemplo de cada módulo produce un PDF y un Word válidos (con figuras)
        for (m, name, model) in crate::examples() {
            for f in ["pdf", "docx"] {
                let v = export_report(&json!({"module": m, "model": model, "format": f}), Lang::Es).unwrap_or_else(|e| panic!("{m}/{name}/{f}: {e}"));
                let b = base64_decode(v["base64"].as_str().unwrap()).unwrap();
                assert!(b.len() > 1000, "{m}/{name}/{f}");
            }
        }
    }
}
