//! Guías «Guíame» de cada módulo: qué es, para qué sirve en la vida real, historia con
//! fuentes, qué datos necesita, cómo usarlo en IAQSB, cómo lo resuelve el motor, cómo leer los
//! resultados, qué se usa hoy en la industria, alternativas, errores comunes, una sección para
//! el profesor, glosario y referencias. El formato está en `guias/ESQUEMA.md`.
//!
//! Las referencias son identificadores de la bibliografía central: aquí se resuelven a la cita
//! (Autor, año) y a la referencia completa en APA 7.

use crate::biblio;
use crate::i18n::Lang;
use serde_json::{json, Value};
use std::fmt::Write;

macro_rules! guias {
    ($($id:literal),* $(,)?) => {
        &[$(($id, include_str!(concat!("../guias/", $id, ".json")))),*]
    };
}

const GUIAS: &[(&str, &str)] = guias!(
    "lp", "gp", "qp", "nlp", "dp", "net", "da", "pert", "mkp", "qa", "qss", "its", "inv", "fin", "fc", "st", "ap", "pmp", "mrp", "mrp2", "job", "fll", "qcc", "asa", "vrp", "alm", "kpi", "modo",
);

/// Módulos que tienen guía.
pub fn modulos() -> Vec<&'static str> {
    GUIAS.iter().map(|(id, _)| *id).collect()
}

/// La guía cruda (como está en el archivo).
pub fn cruda(modulo: &str) -> Option<Value> {
    let (_, txt) = GUIAS.iter().find(|(id, _)| *id == modulo)?;
    serde_json::from_str(txt).ok()
}

fn cita(id: &str, lang: Lang) -> Value {
    match biblio::get(id) {
        Some(r) => json!({"id": id, "cita": biblio::cite_l(r, lang), "apa": biblio::apa_l(r, lang), "verificada": biblio::verified(r), "enlace": biblio::link(r)}),
        None => json!({"id": id, "cita": id, "apa": id, "verificada": false}),
    }
}

/// La guía con las referencias resueltas: `historia[].citas` y `referencias_apa`.
pub fn guia(modulo: &str, lang: Lang) -> Option<Value> {
    let mut g = cruda(modulo)?;
    let mut ids: Vec<String> = g["referencias"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    if let Some(h) = g["historia"].as_array_mut() {
        for e in h.iter_mut() {
            let refs: Vec<String> = e["refs"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
            for r in &refs {
                if !ids.contains(r) {
                    ids.push(r.clone());
                }
            }
            e["citas"] = Value::Array(refs.iter().map(|r| cita(r, lang)).collect());
        }
    }
    let mut apa: Vec<Value> = ids.iter().map(|r| cita(r, lang)).collect();
    apa.sort_by(|a, b| a["apa"].as_str().unwrap_or("").to_lowercase().cmp(&b["apa"].as_str().unwrap_or("").to_lowercase()));
    g["referencias_apa"] = Value::Array(apa);
    Some(g)
}

fn textos(v: &Value) -> Vec<&str> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str()).collect()).unwrap_or_default()
}

/// La guía en Markdown (para la IA, la CLI y el MCP).
pub fn markdown(g: &Value) -> String {
    let mut m = String::new();
    let s = |k: &str| g[k].as_str().unwrap_or("").to_string();
    let _ = writeln!(m, "# {}\n\n{}\n", s("titulo"), s("resumen"));
    let parrafos = textos(&g["que_es"]);
    if !parrafos.is_empty() {
        let _ = writeln!(m, "## Qué es\n\n{}\n", parrafos.join("\n\n"));
    }
    if !s("analogia").is_empty() {
        let _ = writeln!(m, "> {}\n", s("analogia"));
    }
    if let Some(a) = g["para_que_sirve"].as_array().filter(|a| !a.is_empty()) {
        let _ = writeln!(m, "## Para qué sirve en la vida real\n");
        for x in a {
            let _ = writeln!(m, "- **{}**: {}", x["sector"].as_str().unwrap_or(""), x["ejemplo"].as_str().unwrap_or(""));
        }
        m.push('\n');
    }
    if let Some(a) = g["historia"].as_array().filter(|a| !a.is_empty()) {
        let _ = writeln!(m, "## Historia\n");
        for x in a {
            let citas: Vec<&str> = x["citas"].as_array().map(|c| c.iter().filter_map(|y| y["cita"].as_str()).collect()).unwrap_or_default();
            let _ = writeln!(m, "- **{}**: {}{}", x["anio"].as_str().unwrap_or(""), x["hecho"].as_str().unwrap_or(""), if citas.is_empty() { String::new() } else { format!(" ({})", citas.join("; ")) });
        }
        m.push('\n');
    }
    if let Some(a) = g["necesitas"].as_array().filter(|a| !a.is_empty()) {
        let _ = writeln!(m, "## Qué datos necesitas\n");
        for x in a {
            let _ = writeln!(m, "- **{}**: {}", x["dato"].as_str().unwrap_or(""), x["para_que"].as_str().unwrap_or(""));
        }
        m.push('\n');
    }
    let lista = |m: &mut String, titulo: &str, v: &Value| {
        let t = textos(v);
        if !t.is_empty() {
            let _ = writeln!(m, "## {titulo}\n");
            for x in t {
                let _ = writeln!(m, "- {x}");
            }
            m.push('\n');
        }
    };
    lista(&mut m, "Supuestos y límites", &g["supuestos"]);
    if let Some(a) = g["pasos"].as_array().filter(|a| !a.is_empty()) {
        let _ = writeln!(m, "## Cómo usarlo en IAQSB\n");
        for (k, x) in a.iter().enumerate() {
            let _ = writeln!(m, "{}. **{}**: {}", k + 1, x["titulo"].as_str().unwrap_or(""), x["texto"].as_str().unwrap_or(""));
        }
        m.push('\n');
    }
    lista(&mut m, "Cómo lo resuelve IAQSB", &g["como_resuelve"]);
    lista(&mut m, "Cómo leer los resultados", &g["interpretar"]);
    if let Some(a) = g["industria"].as_array().filter(|a| !a.is_empty()) {
        let _ = writeln!(m, "## En la industria hoy\n");
        for x in a {
            let _ = writeln!(m, "- **{}** ({}): {}", x["nombre"].as_str().unwrap_or(""), x["tipo"].as_str().unwrap_or(""), x["uso"].as_str().unwrap_or(""));
        }
        m.push('\n');
    }
    lista(&mut m, "Alternativas", &g["alternativas"]);
    lista(&mut m, "Errores comunes", &g["errores_comunes"]);
    let p = &g["profesor"];
    if p.is_object() {
        let _ = writeln!(m, "## Para el profesor\n");
        for (k, t) in [("objetivos", "Objetivos"), ("actividades", "Actividades"), ("preguntas", "Preguntas")] {
            let v = textos(&p[k]);
            if !v.is_empty() {
                let _ = writeln!(m, "**{t}**\n");
                for x in v {
                    let _ = writeln!(m, "- {x}");
                }
                m.push('\n');
            }
        }
    }
    if let Some(a) = g["glosario"].as_array().filter(|a| !a.is_empty()) {
        let _ = writeln!(m, "## Glosario\n");
        for x in a {
            let _ = writeln!(m, "- **{}**: {}", x["termino"].as_str().unwrap_or(""), x["definicion"].as_str().unwrap_or(""));
        }
        m.push('\n');
    }
    if let Some(a) = g["referencias_apa"].as_array().filter(|a| !a.is_empty()) {
        let _ = writeln!(m, "## Referencias (APA 7)\n");
        for x in a {
            let pv = if x["verificada"].as_bool() == Some(false) { " (por verificar)" } else { "" };
            let _ = writeln!(m, "- {}{pv}", biblio::plain(x["apa"].as_str().unwrap_or("")));
        }
    }
    m
}
