//! Borradores de cursos: la carpeta de trabajo del asistente de creación de la app.
//!
//! Un borrador es un curso a medio hacer en `<carpeta de datos>/borradores/<id>` (la misma
//! estructura que un `.ncourse`). El asistente (o la persona) lee y escribe sus archivos, lo
//! valida, lo instala como vista previa para verlo en la app, prueba sus ejemplos y lo exporta.

use super::{archivos, carpeta, empaquetar, ejemplos, id_valido, leer, leer_json, malo, resolver, texto, validar, EXTENSIONES, MAX_BYTES};
use crate::{ApiError, Result};
use iaqsb_core::i18n::Lang;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Archivo oculto del borrador (no entra en el paquete): con qué id se instaló la vista previa.
const ESTADO: &str = ".borrador.json";
/// Tamaño máximo de un archivo que escribe el asistente.
const MAX_ARCHIVO: usize = 20 * 1024 * 1024;
const TEXTOS: &[&str] = &["json", "md", "txt", "py", "r", "iaqsb", "csv", "tsv", "svg"];

fn io(e: std::io::Error) -> ApiError {
    ApiError::BadInput(e.to_string())
}

/// Carpeta de los borradores, al lado de la de los cursos instalados.
pub fn carpeta_borradores() -> PathBuf {
    if let Ok(p) = std::env::var("IAQSB_BORRADORES") {
        return PathBuf::from(p);
    }
    let c = carpeta();
    c.parent().map(|p| p.join("borradores")).unwrap_or_else(|| c.join("..").join("borradores"))
}

fn dir(id: &str) -> Result<PathBuf> {
    if !id_valido(id) {
        return malo(format!("id de borrador no válido: «{id}»"));
    }
    let d = carpeta_borradores().join(id);
    if !d.is_dir() {
        return malo(format!("no hay un borrador «{id}» (mira los que hay con draft_list)"));
    }
    Ok(d)
}

/// Ruta relativa segura dentro del borrador.
fn ruta(d: &Path, rel: &str) -> Result<PathBuf> {
    let rel = rel.trim().replace('\\', "/");
    let rel = rel.trim_start_matches("./");
    if rel.is_empty() || rel.starts_with('/') || rel.split('/').any(|p| p.is_empty() || p == ".." || p == "." || p.contains(':') || p.starts_with('.')) {
        return malo(format!("ruta no permitida: «{rel}» (usa rutas relativas como temas/mi-tema/tema.json)"));
    }
    Ok(d.join(rel))
}

fn extension(p: &Path) -> String {
    p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase()
}

fn modificado(d: &Path) -> u128 {
    archivos(d)
        .iter()
        .filter_map(|f| std::fs::metadata(f).ok()?.modified().ok())
        .max()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|t| t.as_millis())
        .unwrap_or(0)
}

fn estado(d: &Path) -> Value {
    leer_json(&d.join(ESTADO)).unwrap_or(json!({}))
}

/// Los borradores guardados.
pub fn lista() -> Value {
    let mut v: Vec<Value> = std::fs::read_dir(carpeta_borradores())
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let d = e.path();
            let id = e.file_name().to_string_lossy().to_string();
            if !id_valido(&id) {
                return None;
            }
            let def = leer_json(&d.join("curso.json")).unwrap_or(json!({}));
            let instalado = texto(&estado(&d), "instalado");
            Some(json!({
                "id": id, "titulo": texto(&def, "titulo"), "curso": texto(&def, "id"),
                "temas": def["temas"].as_array().map(|a| a.len()).unwrap_or(0),
                "modificado": modificado(&d) as u64,
                "instalado": !instalado.is_empty() && carpeta().join(&instalado).join("curso.json").is_file()
            }))
        })
        .collect();
    v.sort_by_key(|x| std::cmp::Reverse(x["modificado"].as_u64().unwrap_or(0)));
    Value::Array(v)
}

fn id_libre(base: &str) -> String {
    let mut base: String = base.chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-').collect();
    if !id_valido(&base) {
        base = "mi-curso".into();
    }
    base.truncate(40);
    let mut id = base.clone();
    let mut n = 2;
    while carpeta_borradores().join(&id).exists() {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// Un id a partir de un título («Producción y logística» → «produccion-y-logistica»).
pub fn id_de(titulo: &str) -> String {
    let mut s = String::new();
    for c in titulo.to_lowercase().chars() {
        let c = match c {
            'á' | 'à' | 'ä' | 'â' | 'ã' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'õ' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.ends_with('-') && !s.is_empty() {
            s.push('-');
        }
    }
    let s = s.trim_matches('-').to_string();
    if id_valido(&s) { s } else { "mi-curso".into() }
}

fn copiar(origen: &Path, destino: &Path) -> Result<()> {
    for f in archivos(origen) {
        let p = destino.join(f.strip_prefix(origen).unwrap());
        std::fs::create_dir_all(p.parent().unwrap()).map_err(io)?;
        std::fs::copy(&f, &p).map_err(io)?;
    }
    Ok(())
}

/// Crea un borrador: vacío, con la plantilla de ejemplo, o copia de un curso instalado.
pub fn nuevo(titulo: &str, desde: &str) -> Result<Value> {
    let titulo = if titulo.trim().is_empty() { "Mi curso" } else { titulo.trim() };
    std::fs::create_dir_all(carpeta_borradores()).map_err(io)?;
    if let Some(cid) = desde.strip_prefix("curso:") {
        if !id_valido(cid) || !carpeta().join(cid).join("curso.json").is_file() {
            return malo(format!("no hay un curso instalado «{cid}»"));
        }
        let id = id_libre(cid);
        let d = carpeta_borradores().join(&id);
        copiar(&carpeta().join(cid), &d)?;
        std::fs::write(d.join(ESTADO), json!({"instalado": cid}).to_string()).map_err(io)?;
        return Ok(json!({"id": id, "carpeta": d.to_string_lossy()}));
    }
    let cid = id_de(titulo);
    let id = id_libre(&cid);
    let d = carpeta_borradores().join(&id);
    if desde == "plantilla" {
        super::nuevo(&d, &id, titulo)?;
    } else {
        std::fs::create_dir_all(d.join("temas")).map_err(io)?;
        let j = |v: Value| serde_json::to_string_pretty(&v).unwrap() + "\n";
        std::fs::write(
            d.join("curso.json"),
            j(json!({
                "formato": 1, "id": id, "titulo": titulo, "version": "0.1.0",
                "descripcion": "", "autores": [], "licencia": "CC BY 4.0", "idioma": "es", "area": "Ingeniería industrial", "temas": []
            })),
        )
        .map_err(io)?;
        std::fs::write(d.join("fuentes.json"), "[]\n").map_err(io)?;
    }
    Ok(json!({"id": id, "carpeta": d.to_string_lossy()}))
}

/// Abre un `.ncourse` como borrador (para modificar un curso que alguien compartió).
pub fn importar(bytes: &[u8]) -> Result<Value> {
    let tmp = super::desempacar(bytes)?;
    let def = match leer_json(&tmp.join("curso.json")) {
        Ok(d) => d,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp);
            return malo(format!("el paquete no tiene un curso.json válido: {e}"));
        }
    };
    std::fs::create_dir_all(carpeta_borradores()).map_err(io)?;
    let id = id_libre(&texto(&def, "id"));
    let d = carpeta_borradores().join(&id);
    let r = copiar(&tmp, &d);
    let _ = std::fs::remove_dir_all(&tmp);
    r?;
    Ok(json!({"id": id, "titulo": texto(&def, "titulo"), "carpeta": d.to_string_lossy()}))
}

/// El borrador completo (archivos, temas y validación) o el contenido de un archivo.
pub fn ver(id: &str, rel: Option<&str>) -> Result<Value> {
    let d = dir(id)?;
    if let Some(rel) = rel.filter(|r| !r.trim().is_empty()) {
        let p = ruta(&d, rel)?;
        if !p.is_file() {
            return malo(format!("el borrador no tiene el archivo «{rel}»"));
        }
        let ext = extension(&p);
        let datos = std::fs::read(&p).map_err(io)?;
        if TEXTOS.contains(&ext.as_str()) {
            return Ok(json!({"ruta": rel, "texto": String::from_utf8_lossy(&datos), "bytes": datos.len()}));
        }
        return Ok(json!({"ruta": rel, "base64": iaqsb_render::base64(&datos), "bytes": datos.len()}));
    }
    let def = leer_json(&d.join("curso.json")).ok();
    let lista_archivos: Vec<Value> = archivos(&d)
        .iter()
        .map(|f| json!({"ruta": f.strip_prefix(&d).unwrap().to_string_lossy().replace('\\', "/"), "bytes": std::fs::metadata(f).map(|m| m.len()).unwrap_or(0)}))
        .collect();
    let mut temas = vec![];
    for t in def.as_ref().and_then(|x| x["temas"].as_array()).into_iter().flatten() {
        let tid = t.as_str().unwrap_or("");
        let td = leer_json(&d.join("temas").join(tid).join("tema.json")).unwrap_or(json!({}));
        temas.push(json!({
            "id": tid, "titulo": texto(&td, "titulo"), "resumen": texto(&td, "resumen"), "tipo": texto(&td, "tipo"),
            "lenguaje": td["calculo"]["lenguaje"], "base": td["base"], "origen": td["origen"], "ejemplos": td["ejemplos"].as_array().map(|a| a.len()).unwrap_or(0),
            "lecciones": td["lecciones"].as_array().map(|a| a.len()).unwrap_or(0)
        }));
    }
    let (errores, avisos) = validar(&d);
    let inst = texto(&estado(&d), "instalado");
    let instalado = !inst.is_empty() && carpeta().join(&inst).join("curso.json").is_file();
    Ok(json!({
        "id": id, "carpeta": d.to_string_lossy(), "curso": def, "temas": temas, "archivos": lista_archivos,
        "validacion": {"ok": errores.is_empty(), "errores": errores, "avisos": avisos},
        "instalado": instalado, "instalado_como": if instalado { json!(inst) } else { Value::Null },
        "modificado": modificado(&d) as u64
    }))
}

/// Escribe un archivo del borrador (texto o base64). Los JSON se revisan antes de guardarlos.
pub fn escribir(id: &str, rel: &str, texto_: Option<&str>, base64: Option<&str>) -> Result<Value> {
    let d = dir(id)?;
    let p = ruta(&d, rel)?;
    let ext = extension(&p);
    if !EXTENSIONES.contains(&ext.as_str()) {
        return malo(format!("extensión no permitida en un curso: «.{ext}» (se aceptan {})", EXTENSIONES.join(", ")));
    }
    let datos = match (texto_, base64) {
        (Some(t), _) => {
            if ext == "json" {
                if let Err(e) = serde_json::from_str::<Value>(t.trim_start_matches('\u{feff}')) {
                    return malo(format!("{rel}: JSON no válido ({e}); no se guardó"));
                }
            }
            t.as_bytes().to_vec()
        }
        (None, Some(b)) => crate::base64_decode(b)?,
        _ => return malo("falta content (texto) o base64"),
    };
    if datos.len() > MAX_ARCHIVO {
        return malo(format!("{rel}: el archivo pesa más de {} MB", MAX_ARCHIVO / 1_048_576));
    }
    let total: u64 = archivos(&d).iter().filter(|f| **f != p).filter_map(|f| std::fs::metadata(f).ok()).map(|m| m.len()).sum();
    if total as usize + datos.len() > MAX_BYTES {
        return malo(format!("el curso pasaría de {} MB", MAX_BYTES / 1_048_576));
    }
    let nuevo = !p.exists();
    std::fs::create_dir_all(p.parent().unwrap()).map_err(io)?;
    std::fs::write(&p, &datos).map_err(io)?;
    Ok(json!({"ruta": rel, "bytes": datos.len(), "nuevo": nuevo}))
}

/// Borra un archivo o la carpeta de un tema del borrador.
pub fn borrar(id: &str, rel: &str) -> Result<Value> {
    let d = dir(id)?;
    let p = ruta(&d, rel.trim_end_matches('/'))?;
    if p.is_dir() {
        if !p.starts_with(d.join("temas")) || p == d.join("temas") {
            return malo("solo se pueden borrar carpetas de temas (temas/<tema>)");
        }
        std::fs::remove_dir_all(&p).map_err(io)?;
    } else if p.is_file() {
        if p == d.join("curso.json") {
            return malo("curso.json no se puede borrar");
        }
        std::fs::remove_file(&p).map_err(io)?;
    } else {
        return malo(format!("el borrador no tiene «{rel}»"));
    }
    Ok(json!({"ruta": rel, "borrado": true}))
}

pub fn revisar(id: &str) -> Result<Value> {
    let (errores, avisos) = validar(&dir(id)?);
    Ok(json!({"ok": errores.is_empty(), "errores": errores, "avisos": avisos}))
}

/// Instala el borrador como curso (vista previa): sus temas aparecen en la app como módulos.
pub fn instalar(id: &str) -> Result<Value> {
    let d = dir(id)?;
    let bytes = empaquetar(&d)?;
    let antes = texto(&estado(&d), "instalado");
    let r = super::instalar(&bytes)?;
    let cid = texto(&r, "id");
    // si el curso cambió de id, se quita la vista previa vieja
    if !antes.is_empty() && antes != cid && id_valido(&antes) && carpeta().join(&antes).join("curso.json").is_file() {
        let _ = super::quitar(&antes);
    }
    std::fs::write(d.join(ESTADO), json!({"instalado": cid}).to_string()).map_err(io)?;
    Ok(r)
}

/// Instala el borrador y resuelve los ejemplos de sus temas (o de uno).
pub fn probar(id: &str, tema: Option<&str>, lang: Lang) -> Result<Value> {
    let inst = instalar(id)?;
    let cid = texto(&inst, "id");
    let c = leer(&carpeta().join(&cid)).map_err(ApiError::BadInput)?;
    let mut v = vec![];
    for t in &c.temas {
        if tema.is_some_and(|x| !x.is_empty() && x != t.id) || !t.calcula() {
            continue;
        }
        let m = t.modulo();
        let exs = match ejemplos(&m) {
            Ok(e) => e.as_array().cloned().unwrap_or_default(),
            Err(e) => {
                v.push(json!({"tema": t.id, "modulo": m, "ok": false, "error": e.to_string()}));
                continue;
            }
        };
        if exs.is_empty() {
            v.push(json!({"tema": t.id, "modulo": m, "ok": false, "error": "el tema no tiene ejemplos para probar"}));
        }
        for e in exs {
            v.push(match resolver(&m, e["model"].clone(), None, lang) {
                Ok(r) => {
                    let metricas: Vec<Value> = r["sections"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .flat_map(|s| s["blocks"].as_array().cloned().unwrap_or_default())
                        .filter(|b| b["type"] == "metrics")
                        .flat_map(|b| b["items"].as_array().cloned().unwrap_or_default())
                        .take(12)
                        .map(|i| json!({"label": i["label"], "value": i["value"]}))
                        .collect();
                    json!({"tema": t.id, "modulo": m, "ejemplo": e["name"], "ok": true, "headline": r["headline"], "metricas": metricas})
                }
                Err(x) => json!({"tema": t.id, "modulo": m, "ejemplo": e["name"], "ok": false, "error": x.to_string()}),
            });
        }
    }
    Ok(json!({"curso": cid, "avisos": inst["avisos"], "resultados": v}))
}

pub fn paquete(id: &str) -> Result<Vec<u8>> {
    empaquetar(&dir(id)?)
}

/// Borra el borrador (y su vista previa instalada, si se pide).
pub fn quitar(id: &str, con_vista: bool) -> Result<Value> {
    let d = dir(id)?;
    let inst = texto(&estado(&d), "instalado");
    if con_vista && id_valido(&inst) && carpeta().join(&inst).join("curso.json").is_file() {
        let _ = super::quitar(&inst);
    }
    std::fs::remove_dir_all(&d).map_err(io)?;
    Ok(json!({"id": id, "quitado": true}))
}

// ------------------------------------------------------------------ texto de diapositivas y documentos

fn texto_xml(xml: &str, etiqueta: &str, parrafo: &str) -> String {
    // texto de las etiquetas <a:t>…</a:t> (o <w:t …>…</w:t>), con un salto en cada fin de párrafo
    let abre = format!("<{etiqueta}");
    let cierra = format!("</{etiqueta}>");
    let fin_parrafo = format!("</{parrafo}>");
    let mut out = String::new();
    let mut i = 0;
    let b = xml.as_bytes();
    while i < xml.len() {
        if xml[i..].starts_with(&fin_parrafo) {
            if !out.ends_with('\n') && !out.is_empty() {
                out.push('\n');
            }
            i += fin_parrafo.len();
        } else if xml[i..].starts_with(&abre) && matches!(b.get(i + abre.len()), Some(b'>') | Some(b' ')) {
            let Some(j) = xml[i..].find('>') else { break };
            let ini = i + j + 1;
            let Some(k) = xml[ini..].find(&cierra) else { break };
            out.push_str(&xml[ini..ini + k]);
            i = ini + k + cierra.len();
        } else {
            i += xml[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        }
    }
    entidades(&out)
}

/// Cambia las entidades de XML (&amp; &lt; &#243; &#xF3;…) por sus caracteres.
fn entidades(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut resto = s;
    while let Some(i) = resto.find('&') {
        out.push_str(&resto[..i]);
        let tras = &resto[i..];
        let fin = tras.find(';').filter(|&j| j <= 10);
        let c = fin.and_then(|j| match &tras[1..j] {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            e if e.starts_with("#x") || e.starts_with("#X") => u32::from_str_radix(&e[2..], 16).ok().and_then(char::from_u32),
            e if e.starts_with('#') => e[1..].parse::<u32>().ok().and_then(char::from_u32),
            _ => None,
        });
        match (c, fin) {
            (Some(c), Some(j)) => {
                out.push(c);
                resto = &tras[j + 1..];
            }
            _ => {
                out.push('&');
                resto = &tras[1..];
            }
        }
    }
    out.push_str(resto);
    out
}

/// Texto de una presentación (.pptx) o un documento de Word (.docx), en Markdown sencillo.
pub fn texto_office(bytes: &[u8], max: usize) -> Result<Value> {
    if !iaqsb_core::zip::is_zip(bytes) {
        return malo("no es un archivo .pptx ni .docx");
    }
    let entradas = iaqsb_core::zip::read(bytes).map_err(ApiError::BadInput)?;
    let mut out = String::new();
    let mut diapositivas: Vec<(u32, String, Option<String>)> = vec![];
    let mut notas: std::collections::HashMap<u32, String> = Default::default();
    let mut tipo = "";
    for (n, d) in &entradas {
        let num = |pre: &str| n.strip_prefix(pre).and_then(|r| r.strip_suffix(".xml")).and_then(|x| x.parse::<u32>().ok());
        if let Some(k) = num("ppt/slides/slide") {
            tipo = "pptx";
            diapositivas.push((k, texto_xml(&String::from_utf8_lossy(d), "a:t", "a:p"), None));
        } else if let Some(k) = num("ppt/notesSlides/notesSlide") {
            notas.insert(k, texto_xml(&String::from_utf8_lossy(d), "a:t", "a:p"));
        } else if n == "word/document.xml" {
            tipo = "docx";
            out = texto_xml(&String::from_utf8_lossy(d), "w:t", "w:p");
        }
    }
    if tipo == "pptx" {
        diapositivas.sort_by_key(|x| x.0);
        for (k, t, _) in &mut diapositivas {
            out.push_str(&format!("## Diapositiva {k}\n\n{}\n", t.trim()));
            if let Some(nt) = notas.get(k).map(|s| s.trim()).filter(|s| !s.is_empty() && !s.chars().all(|c| c.is_ascii_digit())) {
                out.push_str(&format!("\nNotas: {nt}\n"));
            }
            out.push('\n');
        }
    } else if tipo.is_empty() {
        return malo("no se encontró texto: ¿es un .pptx o un .docx?");
    }
    let total = out.chars().count();
    let recortado = total > max;
    if recortado {
        out = out.chars().take(max).collect::<String>() + "\n\n[… texto recortado]";
    }
    Ok(json!({"text": out, "tipo": tipo, "diapositivas": diapositivas.len(), "recortado": recortado}))
}

// ------------------------------------------------------------------ temas nuevos con plantilla

const MEZCLA_MODELO: &str = include_str!("../../../../cursos/ejemplo-nativo/temas/mezcla-de-productos/modelo.iaqsb");
const MEZCLA_PANTALLA: &str = include_str!("../../../../cursos/ejemplo-nativo/temas/mezcla-de-productos/pantalla.json");
const MEZCLA_EJEMPLO: &str = include_str!("../../../../cursos/ejemplo-nativo/temas/mezcla-de-productos/ejemplos/muebles.json");

const CALC_R: &str = r#"# `modelo` trae los datos de la pantalla (listas de R); las funciones vienen de nativo.R.
factor <- as.numeric(if (is.null(modelo$factor)) 1 else modelo$factor)
nombres <- vapply(modelo$valores, function(v) as.character(v$nombre), character(1))
x <- vapply(modelo$valores, function(v) as.numeric(v$valor), numeric(1)) * factor
total <- sum(x)

inf <- informe(sprintf("Total = %g", total))
inf <- metricas(inf, list(list("Total", total), list("Valores", length(x))))
inf <- tabla(inf, c("Nombre", "Valor × factor"), lapply(seq_along(x), function(i) list(nombres[i], x[i])), titulo = "Valores")
inf <- paso(inf, "Suma", sprintf("%s = %g", paste(x, collapse = " + "), total))
inf <- datos(inf, total = total)
publicar(inf)
"#;

/// Agrega un tema con una plantilla (leccion, python, r o algebra) y lo lista en curso.json.
pub fn nuevo_tema(id: &str, titulo: &str, tipo: &str) -> Result<Value> {
    let d = dir(id)?;
    let mut def = leer_json(&d.join("curso.json")).map_err(ApiError::BadInput)?;
    let titulo = if titulo.trim().is_empty() { "Tema nuevo" } else { titulo.trim() };
    let base = id_de(titulo);
    let mut tid = base.clone();
    let mut n = 2;
    while d.join("temas").join(&tid).exists() {
        tid = format!("{base}-{n}");
        n += 1;
    }
    let t = d.join("temas").join(&tid);
    let j = |v: Value| serde_json::to_string_pretty(&v).unwrap() + "\n";
    let w = |rel: &str, txt: &str| -> Result<()> {
        let p = t.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).map_err(io)?;
        std::fs::write(&p, txt).map_err(io)
    };
    let leccion = format!(
        "# {titulo}\n\n## La idea\n\nQué es, qué decisión ayuda a tomar y por qué importa en una empresa.\n\n## Cómo funciona\n\nEl método paso a paso y sus fórmulas (escritas con Unicode: x̄, σ², √, Σ, ≤).\n\n## Un ejemplo\n\nUn caso pequeño con números reales, resuelto paso a paso.\n\n## Errores comunes\n\n- …\n"
    );
    w("leccion.md", &leccion)?;
    let lecciones = json!([{"titulo": titulo, "archivo": "leccion.md"}]);
    let resumen = "De qué trata el tema y qué calcula.";
    let calculadora_simple = |archivo: &str, leng: &str| -> Result<Value> {
        w("pantalla.json", &j(json!({
            "problem_types": [{
                "id": "basico", "label": "Básico", "description": "Una lista de valores", "dims": [],
                "layout": [
                    {"type": "params", "title": "Parámetros", "fields": [{"key": "factor", "label": "Factor", "kind": {"type": "number"}, "default": 1}]},
                    {"type": "records", "title": "Valores", "path": "valores", "columns": [{"key": "nombre", "label": "Nombre", "kind": {"type": "text"}}, {"key": "valor", "label": "Valor", "kind": {"type": "number"}}], "add_label": "Añadir valor"}
                ]
            }],
            "options": []
        })))?;
        w("ejemplos/ejemplo.json", &j(json!({"name": "Ejemplo", "problem_type": "basico", "factor": 2, "valores": [{"nombre": "A", "valor": 3}, {"nombre": "B", "valor": 5}]})))?;
        Ok(json!({"id": tid, "titulo": titulo, "resumen": resumen, "tipo": "calculadora", "lecciones": lecciones,
            "calculo": {"lenguaje": leng, "archivo": archivo, "pantalla": "pantalla.json"},
            "ejemplos": [{"nombre": "Ejemplo", "archivo": "ejemplos/ejemplo.json"}]}))
    };
    let tema = match tipo {
        "leccion" => json!({"id": tid, "titulo": titulo, "resumen": "De qué trata el tema y por qué importa.", "tipo": "leccion", "lecciones": lecciones}),
        "python" => {
            let py = super::FORMATO_PY;
            w("calcular.py", py)?;
            calculadora_simple("calcular.py", "python")?
        }
        "r" => {
            w("calcular.R", CALC_R)?;
            calculadora_simple("calcular.R", "r")?
        }
        "algebra" => {
            w("modelo.iaqsb", MEZCLA_MODELO)?;
            w("pantalla.json", MEZCLA_PANTALLA)?;
            w("ejemplos/ejemplo.json", MEZCLA_EJEMPLO)?;
            json!({"id": tid, "titulo": titulo, "resumen": "Un modelo de optimización escrito en el lenguaje algebraico y resuelto con HiGHS.", "tipo": "modelo", "lecciones": lecciones,
                "calculo": {"lenguaje": "algebra", "archivo": "modelo.iaqsb", "pantalla": "pantalla.json"},
                "ejemplos": [{"nombre": "Ejemplo", "archivo": "ejemplos/ejemplo.json"}]})
        }
        o => {
            let _ = std::fs::remove_dir_all(&t);
            return malo(format!("tipo de tema desconocido: «{o}» (leccion, python, r o algebra)"));
        }
    };
    w("tema.json", &j(tema))?;
    let mut temas = def["temas"].as_array().cloned().unwrap_or_default();
    temas.push(json!(tid));
    def["temas"] = Value::Array(temas);
    std::fs::write(d.join("curso.json"), j(def)).map_err(io)?;
    Ok(json!({"id": tid, "carpeta": format!("temas/{tid}")}))
}

// ------------------------------------------------------------------ biblioteca de temas

/// Lo que un curso puede reutilizar: los módulos de Nativo (con su motor, su pantalla y su guía)
/// y los temas de los cursos instalados (se copian al curso para modificarlos).
pub fn biblioteca() -> Value {
    let modulos: Vec<Value> = crate::list_modules()["modules"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|m| m["curso"].is_null() && m["id"] != "lp" && m["available"] == true)
        .map(|m| {
            let id = m["id"].as_str().unwrap_or("");
            json!({"desde": format!("modulo:{id}"), "id": id, "titulo": m["name"], "resumen": m["solves"], "guia": iaqsb_core::guias::cruda(id).is_some()})
        })
        .collect();
    let temas: Vec<Value> = super::instalados()
        .into_iter()
        .flat_map(|c| {
            let ct = texto(&c.def, "titulo");
            let version = texto(&c.def, "version");
            c.temas.into_iter().map(move |t| {
                json!({"desde": t.modulo(), "curso": t.curso, "curso_titulo": ct, "version": version, "tema": t.id, "titulo": texto(&t.def, "titulo"),
                       "resumen": texto(&t.def, "resumen"), "tipo": texto(&t.def, "tipo"), "base": t.base(), "lenguaje": t.def["calculo"]["lenguaje"]})
            })
        })
        .collect();
    json!({"modulos": modulos, "temas": temas})
}

/// Agrega al borrador un tema de la biblioteca:
/// - `modulo:<id>`: un tema basado en ese módulo de Nativo (motor, pantalla y guía del módulo; el
///   curso le pone sus lecciones, ejemplos, guía propia y, si hace falta, un ajuste en Python o R).
/// - `c.<curso>.<tema>`: una copia del tema de un curso instalado, con su origen, para modificarla.
pub fn importar_tema(id: &str, desde: &str, nuevo_id: Option<&str>, titulo: Option<&str>) -> Result<Value> {
    let d = dir(id)?;
    let mut def = leer_json(&d.join("curso.json")).map_err(ApiError::BadInput)?;
    let j = |v: &Value| serde_json::to_string_pretty(v).unwrap() + "\n";
    let libre = |base: &str| {
        let base = id_de(base);
        let mut tid = base.clone();
        let mut n = 2;
        while d.join("temas").join(&tid).exists() {
            tid = format!("{base}-{n}");
            n += 1;
        }
        tid
    };
    let (tid, aviso) = if let Some(m) = desde.strip_prefix("modulo:").or_else(|| (!desde.starts_with("c.")).then_some(desde)) {
        if m == "lp" {
            return malo("«lp» tiene su propio editor: para un modelo de optimización usa un tema de tipo «modelo» (lenguaje algebraico)");
        }
        crate::spec(m).map_err(|_| ApiError::BadInput(format!("no hay un módulo «{m}» en Nativo (mira la biblioteca con topic_library)")))?;
        let info = crate::list_modules()["modules"].as_array().and_then(|a| a.iter().find(|x| x["id"] == m).cloned()).unwrap_or(json!({}));
        let tid = libre(nuevo_id.unwrap_or(m));
        let t = d.join("temas").join(&tid);
        std::fs::create_dir_all(&t).map_err(io)?;
        let tema = json!({
            "id": tid, "titulo": titulo.map(String::from).unwrap_or_else(|| texto(&info, "name")), "resumen": texto(&info, "solves"),
            "tipo": "modulo", "base": m, "lecciones": [], "ejemplos": [], "origen": {"modulo": m}
        });
        std::fs::write(t.join("tema.json"), j(&tema)).map_err(io)?;
        (tid, format!("Tema basado en el módulo «{m}»: usa su pantalla, su motor y su guía. Agrégale lecciones, ejemplos propios (modelos del módulo, en ejemplos/), una guia.json con los campos que quieras reemplazar y, si el curso usa otra fórmula, un «ajuste» en Python o R que cambie el informe."))
    } else {
        let t = super::tema(desde).ok_or_else(|| ApiError::BadInput(format!("no hay un tema instalado «{desde}» (mira la biblioteca con topic_library)")))?;
        let curso = leer(&carpeta().join(&t.curso)).map_err(ApiError::BadInput)?;
        let tid = libre(nuevo_id.unwrap_or(&t.id));
        let destino = d.join("temas").join(&tid);
        copiar(&t.dir, &destino)?;
        let mut tema = leer_json(&destino.join("tema.json")).map_err(ApiError::BadInput)?;
        tema["id"] = json!(tid);
        if let Some(x) = titulo {
            tema["titulo"] = json!(x);
        }
        tema["origen"] = json!({"curso": t.curso, "curso_titulo": t.curso_titulo, "tema": t.id, "version": texto(&curso.def, "version"), "autores": curso.def["autores"], "licencia": curso.def["licencia"]});
        std::fs::write(destino.join("tema.json"), j(&tema)).map_err(io)?;
        // las referencias propias que cita su guía pasan a fuentes.json del borrador
        let citas: Vec<String> = leer_json(&destino.join("guia.json"))
            .map(|g| {
                let mut v: Vec<String> = g["referencias"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect();
                for h in g["historia"].as_array().into_iter().flatten() {
                    v.extend(h["refs"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)));
                }
                v
            })
            .unwrap_or_default();
        let origen: Vec<Value> = leer_json(&carpeta().join(&t.curso).join("fuentes.json")).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
        let mut propias: Vec<Value> = leer_json(&d.join("fuentes.json")).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
        for r in origen.into_iter().filter(|r| citas.contains(&texto(r, "id"))) {
            if !propias.iter().any(|x| texto(x, "id") == texto(&r, "id")) {
                propias.push(r);
            }
        }
        std::fs::write(d.join("fuentes.json"), j(&Value::Array(propias))).map_err(io)?;
        let lic = texto(&curso.def, "licencia");
        (
            tid,
            format!(
                "Copia del tema «{}» del curso «{}» (versión {}{}). Queda en temas/ del borrador para modificarla; su origen va en tema.json → origen: conserva el crédito a sus autores.",
                t.id,
                t.curso_titulo,
                texto(&curso.def, "version"),
                if lic.is_empty() { String::new() } else { format!(", licencia {lic}") }
            ),
        )
    };
    let mut temas = def["temas"].as_array().cloned().unwrap_or_default();
    temas.push(json!(tid));
    def["temas"] = Value::Array(temas);
    std::fs::write(d.join("curso.json"), j(&def)).map_err(io)?;
    Ok(json!({"id": tid, "carpeta": format!("temas/{tid}"), "nota": aviso}))
}
