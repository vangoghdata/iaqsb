//! Cursos (`.ncourse`): paquetes de temas que crean profesores, estudiantes o una IA, y que
//! cualquiera instala, usa y comparte. El formato está en `docs/CURSOS.md`.
//!
//! Un curso es una carpeta (o un zip con extensión `.ncourse`):
//!
//! ```text
//! curso.json                 id, título, versión, autores, licencia, área, temas…
//! fuentes.json               referencias propias del curso (opcional)
//! temas/<tema>/tema.json     título, resumen, tipo y cómo calcula
//! temas/<tema>/guia.json     guía con el formato de Guíame (opcional)
//! temas/<tema>/*.md          lecciones (opcional)
//! temas/<tema>/pantalla.json la pantalla de datos (como la de los módulos)
//! temas/<tema>/calcular.py   el cálculo: Python, R (.R) o el lenguaje algebraico (.iaqsb)
//! temas/<tema>/ejemplos/*.json
//! ```
//!
//! Cada tema se ve como un módulo más, con el id `c.<curso>.<tema>`: la app lo abre con la misma
//! pantalla de datos, el mismo informe y el mismo Guíame que los módulos incluidos.

pub mod borradores;

use crate::{ApiError, Result};
use iaqsb_core::ejecutor::{self, Programa};
use iaqsb_core::i18n::Lang;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const AYUDA_PY: &str = include_str!("nativo.py");
const AYUDA_R: &str = include_str!("nativo.R");
const MARCA_INFORME: &str = "@@NATIVO_INFORME@@";
/// Tamaño máximo de un curso (descomprimido).
pub(crate) const MAX_BYTES: usize = 60 * 1024 * 1024;
pub(crate) const EXTENSIONES: &[&str] = &["json", "md", "txt", "py", "r", "iaqsb", "csv", "tsv", "xlsx", "png", "jpg", "jpeg", "webp", "svg", "gif", "pdf"];

pub(crate) fn malo<T>(m: impl Into<String>) -> Result<T> {
    Err(ApiError::BadInput(m.into()))
}

// ------------------------------------------------------------------ carpetas

/// Carpeta donde se instalan los cursos: la misma para la app, la CLI y el servidor MCP.
pub fn carpeta() -> PathBuf {
    if let Ok(p) = std::env::var("IAQSB_CURSOS") {
        return PathBuf::from(p);
    }
    let id = "com.ianativa.iaqsb";
    let base = if cfg!(windows) {
        std::env::var("APPDATA").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir())
    } else if cfg!(target_os = "macos") {
        std::env::var("HOME").map(|h| PathBuf::from(h).join("Library").join("Application Support")).unwrap_or_else(|_| std::env::temp_dir())
    } else {
        std::env::var("XDG_DATA_HOME").map(PathBuf::from).or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".local").join("share"))).unwrap_or_else(|_| std::env::temp_dir())
    };
    base.join(id).join("cursos")
}

pub(crate) fn id_valido(s: &str) -> bool {
    (2..=48).contains(&s.len()) && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') && !s.starts_with('-')
}

pub(crate) fn leer_json(p: &Path) -> std::result::Result<Value, String> {
    let t = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
    serde_json::from_str(t.trim_start_matches('\u{feff}')).map_err(|e| format!("{}: JSON no válido: {e}", p.display()))
}

pub(crate) fn texto(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or("").trim().to_string()
}

// ------------------------------------------------------------------ cursos y temas

#[derive(Debug, Clone)]
pub struct Curso {
    pub dir: PathBuf,
    pub def: Value,
    pub temas: Vec<Tema>,
}

#[derive(Debug, Clone)]
pub struct Tema {
    pub curso: String,
    pub curso_titulo: String,
    pub id: String,
    pub dir: PathBuf,
    pub def: Value,
}

impl Tema {
    pub fn modulo(&self) -> String {
        format!("c.{}.{}", self.curso, self.id)
    }
    pub fn calcula(&self) -> bool {
        self.def["calculo"].is_object() || self.base().is_some()
    }
    /// Módulo de Nativo en que se basa el tema (tipo «modulo»): usa su pantalla, su motor y su
    /// guía, con las lecciones, los ejemplos y el ajuste del curso encima.
    pub fn base(&self) -> Option<String> {
        let b = texto(&self.def, "base");
        (texto(&self.def, "tipo") == "modulo" && !b.is_empty()).then_some(b)
    }
}

/// Lee un curso de una carpeta (sin validar a fondo).
pub fn leer(dir: &Path) -> std::result::Result<Curso, String> {
    let def = leer_json(&dir.join("curso.json"))?;
    let id = texto(&def, "id");
    let titulo = texto(&def, "titulo");
    let mut temas = vec![];
    for t in def["temas"].as_array().into_iter().flatten() {
        let tid = t.as_str().unwrap_or("").to_string();
        let tdir = dir.join("temas").join(&tid);
        let tdef = leer_json(&tdir.join("tema.json"))?;
        temas.push(Tema { curso: id.clone(), curso_titulo: titulo.clone(), id: tid, dir: tdir, def: tdef });
    }
    Ok(Curso { dir: dir.to_path_buf(), def, temas })
}

/// Cursos instalados.
pub fn instalados() -> Vec<Curso> {
    let mut v: Vec<Curso> = std::fs::read_dir(carpeta()).into_iter().flatten().flatten().filter(|e| e.path().join("curso.json").is_file()).filter_map(|e| leer(&e.path()).ok()).collect();
    v.sort_by(|a, b| texto(&a.def, "titulo").to_lowercase().cmp(&texto(&b.def, "titulo").to_lowercase()));
    v
}

/// El tema de un id de módulo `c.<curso>.<tema>`.
pub fn tema(modulo: &str) -> Option<Tema> {
    let resto = modulo.strip_prefix("c.")?;
    let (c, t) = resto.split_once('.')?;
    let dir = carpeta().join(c);
    let curso = leer(&dir).ok()?;
    curso.temas.into_iter().find(|x| x.id == t)
}

pub fn es_tema(modulo: &str) -> bool {
    modulo.starts_with("c.")
}

fn buscar(modulo: &str) -> Result<Tema> {
    match tema(modulo) {
        Some(t) => Ok(t),
        None => malo(format!("no hay un tema instalado con el id «{modulo}» (los temas de los cursos son c.<curso>.<tema>; mira los instalados con list_courses)")),
    }
}

/// Los temas como módulos (para la lista de módulos de la app y de la IA).
pub fn modulos() -> Vec<Value> {
    instalados()
        .into_iter()
        .flat_map(|c| {
            let ct = texto(&c.def, "titulo");
            let cid = texto(&c.def, "id");
            c.temas.into_iter().map(move |t| {
                let base = t.base();
                let ext = base.as_deref().and_then(|b| crate::spec(b).ok()).and_then(|s| s["file_ext"].as_str().map(String::from)).unwrap_or_else(|| "NCDATA".into());
                json!({
                    "id": t.modulo(), "winqsb": "Curso", "name": texto(&t.def, "titulo"), "solves": texto(&t.def, "resumen"),
                    "file_ext": ext, "available": true, "curso": {"id": cid, "titulo": ct}, "leccion": !t.calcula(), "tipo": texto(&t.def, "tipo"),
                    "base": base
                })
            })
        })
        .collect()
}

/// Resumen de los cursos instalados.
pub fn lista() -> Value {
    Value::Array(
        instalados()
            .into_iter()
            .map(|c| {
                let mut d = c.def.clone();
                d["carpeta"] = json!(c.dir.to_string_lossy());
                d["temas"] = Value::Array(c.temas.iter().map(|t| json!({"id": t.id, "modulo": t.modulo(), "titulo": texto(&t.def, "titulo"), "resumen": texto(&t.def, "resumen"), "tipo": texto(&t.def, "tipo"), "calcula": t.calcula(), "base": t.base(), "origen": t.def.get("origen").cloned().unwrap_or(Value::Null)})).collect());
                d
            })
            .collect(),
    )
}

// ------------------------------------------------------------------ validar

/// Revisa un curso en una carpeta: (errores, avisos).
pub fn validar(dir: &Path) -> (Vec<String>, Vec<String>) {
    let mut err = vec![];
    let mut avi = vec![];
    let def = match leer_json(&dir.join("curso.json")) {
        Ok(d) => d,
        Err(e) => return (vec![format!("falta curso.json o no es válido: {e}")], avi),
    };
    if def["formato"].as_u64() != Some(1) {
        err.push("curso.json: «formato» debe ser 1".into());
    }
    let id = texto(&def, "id");
    if !id_valido(&id) {
        err.push(format!("curso.json: «id» no válido: «{id}» (de 2 a 48 letras minúsculas, números o guiones, como «produccion-basica»)"));
    }
    for k in ["titulo", "version", "descripcion"] {
        if texto(&def, k).is_empty() {
            err.push(format!("curso.json: falta «{k}»"));
        }
    }
    if def["autores"].as_array().is_none_or(|a| a.is_empty()) {
        avi.push("curso.json: no dice quién lo hizo («autores»)".into());
    }
    if texto(&def, "licencia").is_empty() {
        avi.push("curso.json: no tiene licencia (sugerida: «CC BY 4.0» para el contenido y «MIT» para el código)".into());
    }
    let fuentes: Vec<String> = match dir.join("fuentes.json") {
        p if p.is_file() => match leer_json(&p) {
            Ok(Value::Array(a)) => {
                let mut ids = vec![];
                for (k, r) in a.iter().enumerate() {
                    let rid = texto(r, "id");
                    if rid.is_empty() || texto(r, "apa").is_empty() || texto(r, "cita").is_empty() {
                        err.push(format!("fuentes.json: la referencia {} necesita «id», «cita» (Autor, año) y «apa»", k + 1));
                    }
                    ids.push(rid);
                }
                ids
            }
            Ok(_) => {
                err.push("fuentes.json debe ser una lista de referencias".into());
                vec![]
            }
            Err(e) => {
                err.push(e);
                vec![]
            }
        },
        _ => vec![],
    };
    let temas = def["temas"].as_array().cloned().unwrap_or_default();
    if temas.is_empty() {
        err.push("curso.json: «temas» debe listar al menos un tema".into());
    }
    for t in &temas {
        let tid = t.as_str().unwrap_or("");
        if !id_valido(tid) {
            err.push(format!("curso.json: id de tema no válido: «{tid}»"));
            continue;
        }
        let tdir = dir.join("temas").join(tid);
        let pre = format!("temas/{tid}");
        let td = match leer_json(&tdir.join("tema.json")) {
            Ok(v) => v,
            Err(e) => {
                err.push(format!("{pre}: falta tema.json o no es válido ({e})"));
                continue;
            }
        };
        for k in ["titulo", "resumen"] {
            if texto(&td, k).is_empty() {
                err.push(format!("{pre}/tema.json: falta «{k}»"));
            }
        }
        let tipo = texto(&td, "tipo");
        if !["leccion", "calculadora", "modelo", "modulo"].contains(&tipo.as_str()) {
            err.push(format!("{pre}/tema.json: «tipo» debe ser leccion, calculadora, modelo o modulo"));
        }
        if texto(&td, "id") != tid {
            err.push(format!("{pre}/tema.json: «id» debe ser «{tid}», el nombre de su carpeta"));
        }
        for l in td["lecciones"].as_array().into_iter().flatten() {
            let f = l["archivo"].as_str().unwrap_or("");
            if !tdir.join(f).is_file() {
                err.push(format!("{pre}: falta la lección «{f}»"));
            }
        }
        if tdir.join("guia.json").is_file() {
            match leer_json(&tdir.join("guia.json")) {
                Ok(g) => {
                    revisar_guia(&g, &pre, &mut err);
                    let mut refs: Vec<String> = g["referencias"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect();
                    for h in g["historia"].as_array().into_iter().flatten() {
                        refs.extend(h["refs"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)));
                    }
                    for r in refs {
                        if !fuentes.contains(&r) && iaqsb_core::biblio::get(&r).is_none() {
                            err.push(format!("{pre}/guia.json: cita «{r}», que no está en fuentes.json ni en la bibliografía de Nativo"));
                        }
                    }
                }
                Err(e) => err.push(e),
            }
        } else if td["lecciones"].as_array().is_none_or(|a| a.is_empty()) && tipo != "modulo" {
            avi.push(format!("{pre}: no tiene guía ni lecciones; solo se verá su resumen"));
        }
        let ejemplos_json = |err: &mut Vec<String>| {
            for e in td["ejemplos"].as_array().into_iter().flatten() {
                let f = e["archivo"].as_str().unwrap_or("");
                if let Err(x) = leer_json(&tdir.join(f)) {
                    err.push(format!("{pre}: ejemplo «{f}»: {x}"));
                }
            }
        };
        if tipo == "modulo" {
            // un módulo de Nativo con lo del curso encima
            let base = texto(&td, "base");
            if base == "lp" {
                err.push(format!("{pre}/tema.json: «lp» tiene su propio editor y no puede ser la base de un tema; usa un tema de tipo «modelo» (lenguaje algebraico)"));
            } else if crate::spec(&base).is_err() {
                let ids: Vec<String> = crate::list_modules()["modules"].as_array().into_iter().flatten().filter(|m| m["curso"].is_null()).filter_map(|m| m["id"].as_str().map(String::from)).filter(|m| m != "lp").collect();
                err.push(format!("{pre}/tema.json: «base» debe ser un módulo de Nativo ({}); se recibió «{base}»", ids.join(", ")));
            }
            if let Some(a) = td["ajuste"].as_object() {
                let leng = a.get("lenguaje").and_then(|v| v.as_str()).unwrap_or("");
                if !["python", "r"].contains(&leng) {
                    err.push(format!("{pre}/tema.json: ajuste.lenguaje debe ser python o r"));
                }
                let arch = a.get("archivo").and_then(|v| v.as_str()).unwrap_or("");
                if !tdir.join(arch).is_file() {
                    err.push(format!("{pre}: falta el archivo del ajuste «{arch}»"));
                }
            }
            if td["calculo"].is_object() {
                err.push(format!("{pre}/tema.json: un tema de tipo modulo calcula con su base (y su «ajuste» opcional); quita «calculo»"));
            }
            ejemplos_json(&mut err);
        }
        if let Some(c) = td["calculo"].as_object() {
            let leng = c.get("lenguaje").and_then(|v| v.as_str()).unwrap_or("");
            if !["python", "r", "algebra"].contains(&leng) {
                err.push(format!("{pre}/tema.json: calculo.lenguaje debe ser python, r o algebra"));
            }
            let arch = c.get("archivo").and_then(|v| v.as_str()).unwrap_or("");
            if !tdir.join(arch).is_file() {
                err.push(format!("{pre}: falta el archivo del cálculo «{arch}»"));
            }
            match leer_json(&tdir.join(c.get("pantalla").and_then(|v| v.as_str()).unwrap_or("pantalla.json"))) {
                Ok(p) => {
                    let pts = p["problem_types"].as_array().cloned().unwrap_or_default();
                    if pts.is_empty() {
                        err.push(format!("{pre}/pantalla.json: «problem_types» debe tener al menos un tipo de problema"));
                    }
                    for pt in &pts {
                        if texto(pt, "id").is_empty() || texto(pt, "label").is_empty() {
                            err.push(format!("{pre}/pantalla.json: cada tipo de problema necesita «id» y «label»"));
                        }
                        for w in pt["layout"].as_array().into_iter().flatten() {
                            if !["params", "records", "matrix"].contains(&texto(w, "type").as_str()) {
                                err.push(format!("{pre}/pantalla.json: tipo de bloque desconocido «{}» (usa params, records o matrix)", texto(w, "type")));
                            }
                        }
                    }
                }
                Err(e) => err.push(e),
            }
            if td["ejemplos"].as_array().is_none_or(|a| a.is_empty()) {
                avi.push(format!("{pre}: no tiene ejemplos (la pantalla empezará vacía)"));
            }
            ejemplos_json(&mut err);
        } else if tipo != "leccion" && tipo != "modulo" {
            err.push(format!("{pre}/tema.json: un tema de tipo {} necesita «calculo»", texto(&td, "tipo")));
        }
    }
    // archivos: extensiones conocidas y tamaño
    let mut total = 0usize;
    for f in archivos(dir) {
        total += std::fs::metadata(&f).map(|m| m.len() as usize).unwrap_or(0);
        let ext = f.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if !EXTENSIONES.contains(&ext.as_str()) {
            err.push(format!("archivo no permitido en un curso: {} (se aceptan {})", f.strip_prefix(dir).unwrap_or(&f).display(), EXTENSIONES.join(", ")));
        }
    }
    if total > MAX_BYTES {
        err.push(format!("el curso pesa {} MB; el máximo es {} MB", total / 1_048_576, MAX_BYTES / 1_048_576));
    }
    (err, avi)
}

/// Campos de la guía (formato de Guíame, crates/iaqsb-core/guias/ESQUEMA.md) y lo que contiene cada uno.
const CAMPOS_GUIA: &[(&str, &str)] = &[
    ("modulo", "texto"),
    ("titulo", "texto"),
    ("resumen", "texto"),
    ("que_es", "textos"),
    ("analogia", "texto"),
    ("para_que_sirve", "sector,ejemplo"),
    ("historia", "anio,hecho"),
    ("necesitas", "dato,para_que"),
    ("supuestos", "textos"),
    ("pasos", "titulo,texto"),
    ("como_resuelve", "textos"),
    ("interpretar", "textos"),
    ("industria", "nombre,tipo,uso"),
    ("alternativas", "textos"),
    ("errores_comunes", "textos"),
    ("profesor", "objetivos,actividades,preguntas"),
    ("glosario", "termino,definicion"),
    ("referencias", "textos"),
];

/// Revisa que la guía tenga el formato de Guíame: campos conocidos y del tipo correcto.
fn revisar_guia(g: &Value, pre: &str, err: &mut Vec<String>) {
    let Some(o) = g.as_object() else {
        err.push(format!("{pre}/guia.json: debe ser un objeto"));
        return;
    };
    let nombres: Vec<&str> = CAMPOS_GUIA.iter().map(|(k, _)| *k).collect();
    for (k, v) in o {
        let Some((_, forma)) = CAMPOS_GUIA.iter().find(|(c, _)| c == k) else {
            err.push(format!("{pre}/guia.json: campo desconocido «{k}» (los campos son: {})", nombres.join(", ")));
            continue;
        };
        let malo_ = |que: &str| format!("{pre}/guia.json: «{k}» debe ser {que}");
        match *forma {
            "texto" => {
                if !v.is_string() {
                    err.push(malo_("un texto"));
                }
            }
            "textos" => {
                if !v.as_array().is_some_and(|a| a.iter().all(|x| x.is_string())) {
                    err.push(malo_("una lista de textos"));
                }
            }
            "objetivos,actividades,preguntas" => {
                if !v.as_object().is_some_and(|p| p.iter().all(|(c, x)| ["objetivos", "actividades", "preguntas"].contains(&c.as_str()) && x.as_array().is_some_and(|a| a.iter().all(|y| y.is_string())))) {
                    err.push(malo_("un objeto {objetivos, actividades, preguntas}, cada uno una lista de textos"));
                }
            }
            campos => {
                let req: Vec<&str> = campos.split(',').collect();
                let ok = v.as_array().is_some_and(|a| a.iter().all(|x| x.is_object() && req.iter().all(|c| x[*c].is_string())));
                if !ok {
                    let ej = req.iter().map(|c| format!("\"{c}\": \"…\"")).collect::<Vec<_>>().join(", ");
                    err.push(malo_(&format!("una lista de objetos {{{ej}}}{}", if *k == "historia" { " (y «refs»: lista de ids)" } else { "" })));
                }
            }
        }
    }
}

pub(crate) fn archivos(dir: &Path) -> Vec<PathBuf> {
    let mut v = vec![];
    let mut pila = vec![dir.to_path_buf()];
    while let Some(d) = pila.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            let n = e.file_name().to_string_lossy().to_string();
            if n.starts_with('.') || n == "__pycache__" {
                continue;
            }
            if p.is_dir() {
                pila.push(p);
            } else {
                v.push(p);
            }
        }
    }
    v.sort();
    v
}

// ------------------------------------------------------------------ paquetes

/// Empaqueta una carpeta de curso en un `.ncourse` (zip), después de validarla.
pub fn empaquetar(dir: &Path) -> Result<Vec<u8>> {
    let (err, _) = validar(dir);
    if !err.is_empty() {
        return malo(format!("el curso tiene errores:\n- {}", err.join("\n- ")));
    }
    let mut z = iaqsb_core::zip::ZipWriter::new();
    for f in archivos(dir) {
        let rel = f.strip_prefix(dir).unwrap().to_string_lossy().replace('\\', "/");
        let d = std::fs::read(&f).map_err(|e| ApiError::BadInput(e.to_string()))?;
        z.add(&rel, &d);
    }
    Ok(z.finish())
}

/// Desempaca un `.ncourse` en una carpeta temporal (rutas seguras y tamaño máximo).
pub(crate) fn desempacar(bytes: &[u8]) -> Result<PathBuf> {
    if !iaqsb_core::zip::is_zip(bytes) {
        return malo("no es un paquete .ncourse (debe ser un zip)");
    }
    let entradas = iaqsb_core::zip::read(bytes).map_err(ApiError::BadInput)?;
    let total: usize = entradas.iter().map(|(_, d)| d.len()).sum();
    if total > MAX_BYTES {
        return malo(format!("el curso pesa más de {} MB", MAX_BYTES / 1_048_576));
    }
    let dir = std::env::temp_dir().join(format!("nativo-curso-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)));
    for (n, d) in entradas {
        if n.ends_with('/') {
            continue;
        }
        let n = n.replace('\\', "/");
        if n.starts_with('/') || n.split('/').any(|p| p == ".." || p.contains(':')) {
            let _ = std::fs::remove_dir_all(&dir);
            return malo(format!("ruta no permitida dentro del paquete: «{n}»"));
        }
        let p = dir.join(&n);
        if let Some(padre) = p.parent() {
            std::fs::create_dir_all(padre).map_err(|e| ApiError::BadInput(e.to_string()))?;
        }
        std::fs::write(&p, d).map_err(|e| ApiError::BadInput(e.to_string()))?;
    }
    Ok(dir)
}

/// Instala un curso (bytes de un `.ncourse`): lo valida y lo deja en la carpeta de cursos.
pub fn instalar(bytes: &[u8]) -> Result<Value> {
    let tmp = desempacar(bytes)?;
    let (err, avi) = validar(&tmp);
    if !err.is_empty() {
        let _ = std::fs::remove_dir_all(&tmp);
        return malo(format!("el curso tiene errores y no se instaló:\n- {}", err.join("\n- ")));
    }
    let c = leer(&tmp).map_err(ApiError::BadInput)?;
    let id = texto(&c.def, "id");
    let destino = carpeta().join(&id);
    std::fs::create_dir_all(carpeta()).map_err(|e| ApiError::BadInput(e.to_string()))?;
    let antes = destino.is_dir();
    if antes {
        std::fs::remove_dir_all(&destino).map_err(|e| ApiError::BadInput(format!("no se pudo reemplazar la versión instalada: {e}")))?;
    }
    if std::fs::rename(&tmp, &destino).is_err() {
        // otra unidad: copiar
        for f in archivos(&tmp) {
            let rel = f.strip_prefix(&tmp).unwrap();
            let p = destino.join(rel);
            if let Some(padre) = p.parent() {
                std::fs::create_dir_all(padre).map_err(|e| ApiError::BadInput(e.to_string()))?;
            }
            std::fs::copy(&f, &p).map_err(|e| ApiError::BadInput(e.to_string()))?;
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }
    Ok(json!({"id": id, "titulo": texto(&c.def, "titulo"), "version": texto(&c.def, "version"), "temas": c.temas.len(), "actualizado": antes, "avisos": avi, "carpeta": destino.to_string_lossy()}))
}

pub fn quitar(id: &str) -> Result<Value> {
    if !id_valido(id) {
        return malo(format!("id de curso no válido: «{id}»"));
    }
    let d = carpeta().join(id);
    if !d.join("curso.json").is_file() {
        return malo(format!("no hay un curso instalado con el id «{id}»"));
    }
    std::fs::remove_dir_all(&d).map_err(|e| ApiError::BadInput(e.to_string()))?;
    Ok(json!({"id": id, "quitado": true}))
}

// ------------------------------------------------------------------ plantilla de un curso nuevo

/// Crea la carpeta de un curso nuevo con un tema de lección y uno de calculadora en Python.
pub fn nuevo(dir: &Path, id: &str, titulo: &str) -> Result<Value> {
    if !id_valido(id) {
        return malo(format!("id no válido: «{id}» (de 2 a 48 letras minúsculas, números o guiones)"));
    }
    if dir.join("curso.json").exists() {
        return malo(format!("ya hay un curso en {}", dir.display()));
    }
    let w = |rel: &str, txt: &str| -> Result<()> {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).map_err(|e| ApiError::BadInput(e.to_string()))?;
        std::fs::write(&p, txt).map_err(|e| ApiError::BadInput(e.to_string()))
    };
    let j = |v: Value| serde_json::to_string_pretty(&v).unwrap() + "\n";
    w("curso.json", &j(json!({
        "formato": 1, "id": id, "titulo": titulo, "version": "0.1.0",
        "descripcion": "Qué enseña el curso y para quién es.",
        "autores": [{"nombre": "Tu nombre", "institucion": "Tu universidad o empresa"}],
        "licencia": "CC BY 4.0", "idioma": "es", "area": "Ingeniería industrial",
        "temas": ["introduccion", "calculadora"]
    })))?;
    w("fuentes.json", &j(json!([])))?;
    w("temas/introduccion/tema.json", &j(json!({
        "id": "introduccion", "titulo": "Introducción", "resumen": "De qué trata el tema y por qué importa.", "tipo": "leccion",
        "lecciones": [{"titulo": "Introducción", "archivo": "introduccion.md"}]
    })))?;
    w("temas/introduccion/introduccion.md", "# Introducción\n\nEscribe aquí la lección: la idea, un ejemplo de la vida real y las fórmulas (con Unicode: x̄, σ², √, Σ, ≤).\n")?;
    w("temas/calculadora/tema.json", &j(json!({
        "id": "calculadora", "titulo": "Calculadora de ejemplo", "resumen": "Suma una lista de valores y los grafica.", "tipo": "calculadora",
        "calculo": {"lenguaje": "python", "archivo": "calcular.py", "pantalla": "pantalla.json"},
        "ejemplos": [{"nombre": "Ejemplo", "archivo": "ejemplos/ejemplo.json"}]
    })))?;
    w("temas/calculadora/pantalla.json", &j(json!({
        "problem_types": [{
            "id": "basico", "label": "Básico", "description": "Una lista de valores", "dims": [],
            "layout": [
                {"type": "params", "title": "Parámetros", "fields": [{"key": "factor", "label": "Factor", "kind": {"type": "number"}, "default": 1}]},
                {"type": "records", "title": "Valores", "path": "valores", "columns": [{"key": "nombre", "label": "Nombre", "kind": {"type": "text"}}, {"key": "valor", "label": "Valor", "kind": {"type": "number"}}], "add_label": "Añadir valor"}
            ]
        }],
        "options": []
    })))?;
    w("temas/calculadora/ejemplos/ejemplo.json", &j(json!({"name": "Ejemplo", "problem_type": "basico", "factor": 2, "valores": [{"nombre": "A", "valor": 3}, {"nombre": "B", "valor": 5}]})))?;
    w("temas/calculadora/calcular.py", FORMATO_PY)?;
    Ok(json!({"carpeta": dir.to_string_lossy(), "id": id}))
}

// ------------------------------------------------------------------ el tema como módulo

fn pantalla(t: &Tema) -> Result<Value> {
    let f = t.def["calculo"]["pantalla"].as_str().unwrap_or("pantalla.json");
    leer_json(&t.dir.join(f)).map_err(ApiError::BadInput)
}

pub fn spec(modulo: &str) -> Result<Value> {
    let t = buscar(modulo)?;
    if let Some(base) = t.base() {
        let mut s = crate::spec(&base)?;
        s["id"] = json!(t.modulo());
        s["name"] = json!(texto(&t.def, "titulo"));
        s["description"] = json!(texto(&t.def, "resumen"));
        s["curso"] = json!({"id": t.curso, "titulo": t.curso_titulo});
        s["base"] = json!(base);
        return Ok(s);
    }
    if !t.calcula() {
        return Ok(json!({"id": t.modulo(), "winqsb": "Curso", "name": texto(&t.def, "titulo"), "description": texto(&t.def, "resumen"), "file_ext": "NCDATA", "problem_types": [], "options": [], "leccion": true}));
    }
    let p = pantalla(&t)?;
    Ok(json!({
        "id": t.modulo(), "winqsb": "Curso", "name": texto(&t.def, "titulo"), "description": texto(&t.def, "resumen"), "file_ext": "NCDATA",
        "problem_types": p["problem_types"].clone(), "options": p.get("options").cloned().unwrap_or(json!([])),
        "curso": {"id": t.curso, "titulo": t.curso_titulo}
    }))
}

pub fn ejemplos(modulo: &str) -> Result<Value> {
    let t = buscar(modulo)?;
    let mut v = vec![];
    for e in t.def["ejemplos"].as_array().into_iter().flatten() {
        let f = e["archivo"].as_str().unwrap_or("");
        let m = leer_json(&t.dir.join(f)).map_err(ApiError::BadInput)?;
        v.push(json!({"module": t.modulo(), "name": e["nombre"].as_str().unwrap_or(f), "model": m, "source": null}));
    }
    // los del módulo base van después de los del curso
    if let Some(base) = t.base() {
        if t.def["ejemplos_base"].as_bool() != Some(false) {
            for (_, n, m) in crate::examples().into_iter().filter(|e| e.0 == base) {
                let src = crate::example_source(&base, &n);
                v.push(json!({"module": t.modulo(), "name": n, "model": m, "source": src}));
            }
        }
    }
    Ok(Value::Array(v))
}

pub fn plantilla(modulo: &str, problem_type: &str, dims: &Value) -> Result<Value> {
    let t = buscar(modulo)?;
    if let Some(base) = t.base() {
        return crate::template(&base, problem_type, dims);
    }
    let p = pantalla(&t)?;
    if let Some(m) = p["plantillas"].get(problem_type) {
        return Ok(m.clone());
    }
    if let Some(e) = ejemplos(modulo)?.as_array().and_then(|a| a.iter().find(|e| e["model"]["problem_type"].as_str() == Some(problem_type)).cloned()) {
        return Ok(e["model"].clone());
    }
    // un modelo vacío con los valores por defecto de los parámetros
    let mut m = json!({"name": texto(&t.def, "titulo"), "problem_type": problem_type});
    if let Some(pt) = p["problem_types"].as_array().and_then(|a| a.iter().find(|x| x["id"].as_str() == Some(problem_type))) {
        for w in pt["layout"].as_array().into_iter().flatten() {
            match texto(w, "type").as_str() {
                "params" => {
                    for f in w["fields"].as_array().into_iter().flatten() {
                        m[texto(f, "key")] = f.get("default").cloned().unwrap_or(Value::Null);
                    }
                }
                "records" | "matrix" => m[texto(w, "path")] = json!([]),
                _ => {}
            }
        }
    }
    Ok(m)
}

pub fn describir(modulo: &str) -> Result<Value> {
    let t = buscar(modulo)?;
    if let Some(base) = t.base() {
        let mut d = crate::describe_module(&base)?;
        d["module"] = json!({"id": t.modulo(), "name": texto(&t.def, "titulo"), "solves": texto(&t.def, "resumen"), "curso": {"id": t.curso, "titulo": t.curso_titulo}, "base": base});
        d["examples"] = ejemplos(modulo)?;
        if let Some(n) = d["notes"].as_array_mut() {
            n.push(json!(format!("Tema de un curso basado en el módulo «{base}» de Nativo: el modelo tiene el mismo esquema y se resuelve con su motor{}.", if t.def["ajuste"].is_object() { ", y después el curso ajusta el informe con su propio código" } else { "" })));
        }
        return Ok(d);
    }
    let ej = ejemplos(modulo).ok().and_then(|e| e.as_array().and_then(|a| a.first().cloned()));
    Ok(json!({
        "module": {"id": t.modulo(), "name": texto(&t.def, "titulo"), "solves": texto(&t.def, "resumen"), "curso": {"id": t.curso, "titulo": t.curso_titulo}},
        "spec": spec(modulo)?, "example": ej,
        "notes": ["Tema de un curso instalado: el modelo es un objeto con problem_type y las claves de los campos de spec.problem_types[].layout (params: claves sueltas; records y matrix: listas en su path).", "Pide la guía y las lecciones con get_module_guide."]
    }))
}

/// Cambia {{campo}} y {{tabla.columna}} del modelo algebraico por los datos de la pantalla.
fn rellenar(plantilla: &str, modelo: &Value) -> Result<String> {
    let lit = |v: &Value| -> String {
        match v {
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => (if *b { "1" } else { "0" }).into(),
            Value::String(s) => {
                if !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_') && !s.chars().next().unwrap().is_ascii_digit() {
                    s.clone()
                } else {
                    format!("\"{}\"", s.replace('"', "'"))
                }
            }
            Value::Null => "0".into(),
            _ => v.to_string(),
        }
    };
    let lista = |v: &Value| -> String {
        match v {
            Value::Array(a) => a
                .iter()
                .map(|x| match x {
                    Value::Array(f) => format!("[{}]", f.iter().map(&lit).collect::<Vec<_>>().join(", ")),
                    o => lit(o),
                })
                .collect::<Vec<_>>()
                .join(", "),
            o => lit(o),
        }
    };
    let mut out = String::new();
    let mut resto = plantilla;
    while let Some(i) = resto.find("{{") {
        out.push_str(&resto[..i]);
        let j = resto[i..].find("}}").ok_or_else(|| ApiError::BadInput("el modelo algebraico tiene un «{{» sin cerrar".into()))? + i;
        let clave = resto[i + 2..j].trim();
        let v = match clave.split_once('.') {
            Some((tabla, col)) => {
                let filas = modelo[tabla].as_array().ok_or_else(|| ApiError::BadInput(format!("la tabla «{tabla}» no está en los datos")))?;
                Value::Array(filas.iter().map(|f| f[col].clone()).collect())
            }
            None => modelo.get(clave).cloned().ok_or_else(|| ApiError::BadInput(format!("el dato «{clave}» no está en la pantalla")))?,
        };
        out.push_str(&lista(&v));
        resto = &resto[j + 2..];
    }
    out.push_str(resto);
    Ok(out)
}

/// Resuelve un tema: con el lenguaje algebraico (HiGHS) o con su código de Python o de R.
pub fn resolver(modulo: &str, modelo: Value, opciones: Option<Value>, lang: Lang) -> Result<Value> {
    let t = buscar(modulo)?;
    if !t.calcula() {
        return malo("este tema es una lección: no tiene cálculo");
    }
    if let Some(base) = t.base() {
        let mut r = crate::solve(&base, modelo.clone(), opciones.clone())?;
        if let Some(a) = t.def["ajuste"].as_object() {
            // el curso cambia o completa el informe del módulo con su código (otra fórmula, otra tabla…)
            let leng = a.get("lenguaje").and_then(|v| v.as_str()).unwrap_or("python");
            let arch = a.get("archivo").and_then(|v| v.as_str()).unwrap_or("");
            let codigo = std::fs::read_to_string(t.dir.join(arch)).map_err(|e| ApiError::BadInput(format!("{arch}: {e}")))?;
            let paquetes = paquetes_de(a.get("paquetes"));
            r = correr(&t, leng, &codigo, json!({"model": modelo, "options": opciones.unwrap_or(json!({})), "lang": idioma(lang), "base": r}), paquetes)?;
        }
        if let Some(o) = r.as_object_mut() {
            o.insert("module".into(), json!(t.modulo()));
        }
        return Ok(r);
    }
    let c = &t.def["calculo"];
    let leng = c["lenguaje"].as_str().unwrap_or("");
    let arch = c["archivo"].as_str().unwrap_or("");
    let codigo = std::fs::read_to_string(t.dir.join(arch)).map_err(|e| ApiError::BadInput(format!("{arch}: {e}")))?;
    let mut r = match leng {
        "algebra" => {
            let texto_m = rellenar(&codigo, &modelo)?;
            let nombre = modelo["name"].as_str().unwrap_or("").to_string();
            let mut r = crate::solve("lp", json!({"format": "iaqsb", "text": texto_m, "name": nombre}), opciones)?;
            // la solución con el formato de los informes genéricos: objetivo, tablas por índices y el modelo lleno
            let mut bloques = vec![json!({"type": "metrics", "items": [
                {"label": "Valor óptimo", "value": r["objective_value"], "emphasis": true},
                {"label": "Estado", "value": match r["status"].as_str().unwrap_or("") { "optimal" => "Óptimo", "infeasible" => "No factible", "unbounded" => "No acotado", "feasible" => "Factible", o => o }},
                {"label": "Iteraciones", "value": r["iterations"]}
            ]})];
            if let Some(m) = r["message"].as_str() {
                bloques.push(json!({"type": "text", "body": m}));
            }
            bloques.extend(r["tables"].as_array().cloned().unwrap_or_default());
            let mut secciones = vec![
                json!({"id": "solucion", "title": "Solución", "blocks": bloques}),
                json!({"id": "modelo", "title": "Modelo", "blocks": [{"type": "text", "title": "Modelo en el lenguaje algebraico (con los datos de la pantalla)", "body": texto_m}]}),
            ];
            secciones.extend(r["sections"].as_array().cloned().unwrap_or_default());
            if let Some(o) = r.as_object_mut() {
                o.insert("modelo_algebraico".into(), json!(texto_m));
                o.insert("sections".into(), Value::Array(secciones));
            }
            r
        }
        "python" | "r" => correr(&t, leng, &codigo, json!({"model": modelo, "options": opciones.unwrap_or(json!({})), "lang": idioma(lang)}), paquetes_de(c.get("paquetes")))?,
        l => return malo(format!("lenguaje de cálculo desconocido: «{l}»")),
    };
    if let Some(o) = r.as_object_mut() {
        o.insert("module".into(), json!(t.modulo()));
        if o.get("problem").and_then(|v| v.as_str()).is_none_or(|s| s.is_empty()) {
            o.insert("problem".into(), modelo["name"].clone());
        }
    }
    Ok(r)
}

fn idioma(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "en",
        Lang::Pt => "pt",
        _ => "es",
    }
}

/// Paquetes que el código necesita («paquetes»: ["pulp", "networkx"] o "pulp, networkx").
fn paquetes_de(v: Option<&Value>) -> Vec<String> {
    match v {
        Some(Value::Array(a)) => a.iter().filter_map(|x| x.as_str()).map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect(),
        Some(Value::String(t)) => t.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect(),
        _ => vec![],
    }
}

/// Corre el código de Python o de R de un tema con la entrada (modelo, opciones, idioma y, en
/// los ajustes, el informe del módulo base) y lee el informe que publica.
fn correr(t: &Tema, leng: &str, codigo: &str, entrada: Value, paquetes: Vec<String>) -> Result<Value> {
    // en R los datos llegan también como código R (así no hace falta ningún paquete para leerlos)
    let mut archivos: Vec<(String, Vec<u8>)> = vec![("entrada.json".into(), entrada.to_string().into_bytes()), ("entrada.R".into(), format!(".entrada <- {}\n", a_r(&entrada)).into_bytes())];
    for f in archivos_tema(&t.dir) {
        let rel = f.strip_prefix(&t.dir).unwrap().to_string_lossy().replace('\\', "/");
        if rel.starts_with("ejemplos/") || rel == "tema.json" || rel == "guia.json" || rel.ends_with(".md") {
            continue;
        }
        archivos.push((rel, std::fs::read(&f).unwrap_or_default()));
    }
    let programa = if leng == "python" {
        archivos.push(("nativo.py".into(), AYUDA_PY.as_bytes().to_vec()));
        format!("import os, sys\nif os.path.isdir('/home/pyodide'):\n    os.chdir('/home/pyodide')\nsys.path.insert(0, os.getcwd())\nsys.modules.pop('nativo', None)\n{codigo}")
    } else {
        archivos.push(("nativo.R".into(), AYUDA_R.as_bytes().to_vec()));
        format!("source(\"nativo.R\", encoding = \"UTF-8\")\n{codigo}")
    };
    let salida = ejecutor::ejecutar(&Programa { lenguaje: leng.into(), archivos, codigo: programa, paquetes }).map_err(ApiError::BadInput)?;
    leer_informe(&salida)
}

/// Un valor JSON como código R (listas con nombre para los objetos).
fn a_r(v: &Value) -> String {
    match v {
        Value::Null => "NULL".into(),
        Value::Bool(b) => (if *b { "TRUE" } else { "FALSE" }).into(),
        Value::Number(n) => n.as_f64().filter(|x| x.is_finite()).map(|x| format!("{x:?}")).unwrap_or_else(|| "NA".into()),
        Value::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "").replace('\t', "\\t")),
        Value::Array(a) => format!("list({})", a.iter().map(a_r).collect::<Vec<_>>().join(", ")),
        Value::Object(o) if o.is_empty() => "structure(list(), names = character(0))".into(),
        Value::Object(o) => format!("list({})", o.iter().map(|(k, x)| format!("`{}` = {}", k.replace('`', ""), a_r(x))).collect::<Vec<_>>().join(", ")),
    }
}

fn archivos_tema(dir: &Path) -> Vec<PathBuf> {
    archivos(dir).into_iter().filter(|f| std::fs::metadata(f).map(|m| m.len() < 8 * 1024 * 1024).unwrap_or(false)).collect()
}

/// Saca el informe de lo que escribió el programa; lo demás va como avisos.
fn leer_informe(salida: &str) -> Result<Value> {
    let mut informe = None;
    let mut avisos = vec![];
    for l in salida.lines() {
        if let Some(j) = l.trim_start().strip_prefix(MARCA_INFORME) {
            informe = Some(serde_json::from_str::<Value>(j.trim()).map_err(|e| ApiError::BadInput(format!("el informe del tema no es JSON válido: {e}")))?);
        } else if !l.trim().is_empty() && !l.trim_start().starts_with("Loading ") && !l.trim_start().starts_with("Loaded ") {
            avisos.push(l.to_string());
        }
    }
    let Some(mut r) = informe else {
        let cola: Vec<&str> = salida.lines().rev().take(15).collect::<Vec<_>>().into_iter().rev().collect();
        return malo(format!("el programa del tema terminó sin publicar el informe (falta Informe(...).publicar()). Lo último que escribió:\n{}", cola.join("\n")));
    };
    if !r["sections"].is_array() || r["headline"].as_str().is_none() {
        return malo("el informe del tema debe tener «headline» y «sections» (usa la clase Informe de nativo.py o las funciones de nativo.R)");
    }
    if !avisos.is_empty() {
        if let Some(s) = r["sections"].as_array_mut().and_then(|a| a.first_mut()) {
            if let Some(b) = s["blocks"].as_array_mut() {
                b.push(json!({"type": "text", "title": "Salida del programa", "body": avisos.join("\n"), "tone": "muted"}));
            }
        }
    }
    Ok(r)
}

/// Resuelve los ejemplos de cada tema que calcula, desde la carpeta del curso (sin instalarlo).
pub fn probar(dir: &Path) -> Result<Value> {
    let c = leer(dir).map_err(ApiError::BadInput)?;
    let id = texto(&c.def, "id");
    // se instala en una carpeta temporal de cursos para usar el mismo camino que la app
    let tmp = std::env::temp_dir().join(format!("nativo-probar-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| ApiError::BadInput(e.to_string()))?;
    let destino = tmp.join(&id);
    for f in archivos(dir) {
        let p = destino.join(f.strip_prefix(dir).unwrap());
        std::fs::create_dir_all(p.parent().unwrap()).map_err(|e| ApiError::BadInput(e.to_string()))?;
        std::fs::copy(&f, &p).map_err(|e| ApiError::BadInput(e.to_string()))?;
    }
    let antes = std::env::var("IAQSB_CURSOS").ok();
    // SAFETY: la CLI corre esto en un solo hilo.
    unsafe { std::env::set_var("IAQSB_CURSOS", &tmp) };
    let mut v = vec![];
    for t in &c.temas {
        if !t.calcula() {
            continue;
        }
        let m = format!("c.{id}.{}", t.id);
        for e in ejemplos(&m)?.as_array().cloned().unwrap_or_default() {
            let r = resolver(&m, e["model"].clone(), None, Lang::Es);
            v.push(match r {
                Ok(r) => json!({"tema": t.id, "ejemplo": e["name"], "ok": true, "resultado": r["headline"]}),
                Err(x) => json!({"tema": t.id, "ejemplo": e["name"], "ok": false, "resultado": x.to_string()}),
            });
        }
    }
    unsafe {
        match antes {
            Some(a) => std::env::set_var("IAQSB_CURSOS", a),
            None => std::env::remove_var("IAQSB_CURSOS"),
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    Ok(Value::Array(v))
}

// ------------------------------------------------------------------ guía y lecciones

/// La guía del tema (formato de Guíame) con sus lecciones y las referencias resueltas.
pub fn guia(modulo: &str, lang: Lang) -> Result<Value> {
    let t = buscar(modulo)?;
    let propia = if t.dir.join("guia.json").is_file() { leer_json(&t.dir.join("guia.json")).map_err(ApiError::BadInput)? } else { json!({}) };
    // un tema basado en un módulo parte de la guía del módulo; lo que escribió el curso la reemplaza campo por campo
    let mut g = match t.base().and_then(|b| iaqsb_core::guias::cruda(&b)) {
        Some(mut b) => {
            for (k, v) in propia.as_object().into_iter().flatten() {
                b[k] = v.clone();
            }
            if propia.get("titulo").is_none() {
                b["titulo"] = json!(texto(&t.def, "titulo"));
            }
            if propia.get("resumen").is_none() && !texto(&t.def, "resumen").is_empty() {
                b["resumen"] = json!(texto(&t.def, "resumen"));
            }
            b
        }
        None => propia,
    };
    g["modulo"] = json!(t.modulo());
    if texto(&g, "titulo").is_empty() {
        g["titulo"] = json!(texto(&t.def, "titulo"));
    }
    if texto(&g, "resumen").is_empty() {
        g["resumen"] = json!(texto(&t.def, "resumen"));
    }
    let mut lecciones = vec![];
    for l in t.def["lecciones"].as_array().into_iter().flatten() {
        let f = l["archivo"].as_str().unwrap_or("");
        if let Ok(md) = std::fs::read_to_string(t.dir.join(f)) {
            lecciones.push(json!({"titulo": l["titulo"].as_str().unwrap_or(f), "markdown": md}));
        }
    }
    g["lecciones"] = Value::Array(lecciones);
    g["curso"] = json!({"id": t.curso, "titulo": t.curso_titulo});
    let curso_dir = carpeta().join(&t.curso);
    let fuentes: Vec<Value> = leer_json(&curso_dir.join("fuentes.json")).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
    Ok(iaqsb_core::guias::resolver(g, lang, &fuentes))
}

/// El formato de los cursos (docs/CURSOS.md), para el asistente de creación.
pub const FORMATO: &str = include_str!("../../../../docs/CURSOS.md");
/// El formato de las guías (Guíame).
pub const ESQUEMA_GUIA: &str = include_str!("../../../iaqsb-core/guias/ESQUEMA.md");

/// La calculadora de ejemplo en Python (plantilla de los temas nuevos).
pub const FORMATO_PY: &str = "from nativo import modelo, Informe\n\nfactor = float(modelo.get(\"factor\") or 1)\nfilas = [(v.get(\"nombre\", \"\"), float(v.get(\"valor\") or 0) * factor) for v in modelo.get(\"valores\", [])]\ntotal = sum(x for _, x in filas)\n\ninf = Informe(f\"Total = {total:g}\")\ninf.metricas([(\"Total\", total), (\"Valores\", len(filas))])\ninf.tabla([\"Nombre\", \"Valor × factor\"], [list(f) for f in filas], titulo=\"Valores\")\ninf.grafica({\"Valor\": [(i, x) for i, (_, x) in enumerate(filas)]}, titulo=\"Valores\", x=\"Valor\", y=\"Cantidad\", tipo=\"bar\", categorias=[n for n, _ in filas])\ninf.paso(\"Suma\", \" + \".join(f\"{x:g}\" for _, x in filas) + f\" = {total:g}\")\ninf.publicar()\n";
