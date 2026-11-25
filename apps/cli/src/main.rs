//! iaqsb: IAQSB en la línea de comandos.
//!
//! Pensado tanto para personas como para agentes de IA sin MCP: todas las órdenes
//! aceptan y devuelven JSON (`--json`), y `iaqsb llamar` expone la API completa.

use clap::{Parser, Subcommand};
use iaqsb_api as api;
use iaqsb_render as render;
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::OnceLock;

use api::core::i18n::{self, Lang};

static LANG: OnceLock<Lang> = OnceLock::new();

/// Idioma de la salida: --lang, o la variable IAQSB_LANG; por defecto español.
fn lang() -> Lang {
    *LANG.get().unwrap_or(&Lang::Es)
}

fn tr(s: &str) -> String {
    i18n::translate(s, lang())
}

/// println! en el idioma elegido.
macro_rules! say {
    () => { println!() };
    ($($t:tt)*) => { println!("{}", tr(&format!($($t)*))) };
}

#[derive(Parser)]
#[command(
    name = "iaqsb",
    version,
    about = "IAQSB · investigación de operaciones nativa para IA. Hecho por IA Nativa."
)]
struct Cli {
    /// Salida en JSON (para scripts y agentes de IA).
    #[arg(long, global = true)]
    json: bool,
    /// Salida en Markdown breve (pensada para agentes de IA): resultado, tablas y gráficas descritas.
    #[arg(long, global = true)]
    md: bool,
    /// Idioma de la salida: es (español), en (inglés) o pt (portugués). También: variable IAQSB_LANG.
    #[arg(long, global = true)]
    lang: Option<String>,
    /// Ejecuta el código de Python o de R de los temas de cursos con el Python o el R del equipo
    /// (solo si confías en el autor del curso: ese código puede leer y escribir archivos).
    #[arg(long, global = true)]
    confiar: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum CursoCmd {
    /// Crea la carpeta de un curso nuevo con un tema de lección y uno de calculadora en Python.
    Nuevo {
        /// Carpeta donde se crea.
        carpeta: PathBuf,
        /// Identificador (letras minúsculas, números y guiones), p. ej. mrp-ii.
        #[arg(long)]
        id: String,
        /// Título del curso.
        #[arg(long)]
        titulo: String,
    },
    /// Revisa una carpeta de curso: errores y avisos.
    Validar { carpeta: PathBuf },
    /// Empaqueta una carpeta de curso en un archivo .ncourse para compartirlo.
    Empaquetar {
        carpeta: PathBuf,
        /// Archivo de salida (por defecto <id>.ncourse).
        #[arg(long)]
        salida: Option<PathBuf>,
    },
    /// Instala un .ncourse (o una carpeta de curso) para usarlo en la app, la CLI y el MCP.
    Instalar { ruta: PathBuf },
    /// Quita un curso instalado.
    Quitar { id: String },
    /// Lista los cursos instalados y sus temas.
    Lista,
    /// Resuelve los ejemplos de cada tema que calcula, para probar el curso antes de compartirlo.
    Probar { carpeta: PathBuf },
}

#[derive(Subcommand)]
enum Cmd {
    /// Lista los módulos y cuáles están disponibles.
    Modulos,
    /// Muestra el esquema JSON y un ejemplo de un módulo.
    Describir { modulo: String },
    /// Resuelve un problema (.json de IAQSB o cualquier archivo de WinQSB).
    Resolver {
        archivo: PathBuf,
        /// Módulo (por defecto se deduce del archivo).
        #[arg(long)]
        modulo: Option<String>,
        /// Muestra el paso a paso (tablas del símplex, iteraciones del método...).
        #[arg(long)]
        pasos: bool,
        /// Muestra los nodos de branch and bound.
        #[arg(long)]
        nodos: bool,
        /// Opciones en JSON, p. ej. '{"integer_tolerance":0.001}'.
        #[arg(long)]
        opciones: Option<String>,
        /// Guarda las gráficas del informe como PNG en esta carpeta (con --pasos, también las del paso a paso).
        #[arg(long)]
        graficas: Option<PathBuf>,
        /// Con --graficas, guarda también cada gráfica en SVG.
        #[arg(long)]
        svg: bool,
        #[command(flatten)]
        motor: MotorArgs,
    },
    /// Motores de optimización disponibles en este equipo (HiGHS, Gurobi, CPLEX, SCIP, CBC, GLPK).
    Motores,
    /// Convierte un modelo entre JSON, WinQSB, Excel (.xlsx) y, en LP-ILP, MPS y LP (según las extensiones).
    Convertir {
        entrada: PathBuf,
        salida: PathBuf,
        /// Módulo, para leer un libro de Excel que no hizo IAQSB.
        #[arg(long)]
        modulo: Option<String>,
    },
    /// Informe para entregar en Word (.docx), PDF, HTML, Excel (.xlsx) o Markdown (según la extensión de --salida).
    Informe {
        archivo: PathBuf,
        /// Archivo de salida: informe.docx, informe.pdf, informe.html, informe.xlsx o informe.md.
        #[arg(long, short)]
        salida: PathBuf,
        /// Incluye el paso a paso (tablas del símplex, iteraciones…).
        #[arg(long)]
        pasos: bool,
        #[arg(long)]
        autor: Option<String>,
        #[arg(long)]
        curso: Option<String>,
        #[arg(long)]
        institucion: Option<String>,
        /// Escribe los números con coma decimal.
        #[arg(long)]
        coma: bool,
        #[arg(long)]
        modulo: Option<String>,
        /// Opciones del módulo en JSON.
        #[arg(long)]
        opciones: Option<String>,
        #[command(flatten)]
        motor: MotorArgs,
    },
    /// Revisa un modelo del lenguaje algebraico (.iaqsb) sin resolverlo: errores o tamaño.
    Revisar { archivo: PathBuf },
    /// ¿Qué pasa si…? Resuelve con varios valores de un dato (p. ej. --dato constraints.2.rhs --desde 10 --hasta 30 --pasos 10).
    Barrer {
        archivo: PathBuf,
        #[arg(long)]
        dato: String,
        #[arg(long)]
        desde: f64,
        #[arg(long)]
        hasta: f64,
        #[arg(long, default_value_t = 10)]
        pasos: u32,
        #[arg(long)]
        modulo: Option<String>,
    },
    /// Análisis paramétrico exacto de LP-ILP: el objetivo según un costo (--costo X1) o un lado derecho (--lado C3).
    Parametrico {
        archivo: PathBuf,
        #[arg(long)]
        costo: Option<String>,
        #[arg(long)]
        lado: Option<String>,
        /// Desde qué valor (por defecto 0).
        #[arg(long)]
        desde: Option<f64>,
        /// Hasta qué valor (por defecto, el doble del valor actual).
        #[arg(long)]
        hasta: Option<f64>,
    },
    /// Ajuste automático de los parámetros del motor para un modelo de LP-ILP.
    Ajustar {
        archivo: PathBuf,
        /// Segundos en total.
        #[arg(long, default_value_t = 20.0)]
        tiempo: f64,
    },
    /// Explica por qué un modelo de LP-ILP no es factible (IIS y relajación de factibilidad).
    Diagnosticar { archivo: PathBuf },
    /// Convierte un archivo de WinQSB a JSON de IAQSB.
    Importar { archivo: PathBuf },
    /// Guarda un modelo JSON en formato de WinQSB.
    Exportar {
        modelo: PathBuf,
        /// Archivo de salida (.LPP, .QAA, .QCC...).
        #[arg(short, long)]
        salida: PathBuf,
        /// Solo LP-ILP: "matrix" o "normal".
        #[arg(long, default_value = "matrix")]
        formato: String,
        /// Módulo (por defecto se deduce del modelo).
        #[arg(long)]
        modulo: Option<String>,
    },
    /// Datos del método gráfico (LP con dos variables).
    Grafico { archivo: PathBuf },
    /// Lista los ejemplos incluidos (con su fuente) o muestra uno.
    Ejemplos { nombre: Option<String> },
    /// Resuelve el mismo problema con cada método y los compara.
    Comparar {
        archivo: PathBuf,
        /// Opción que cambia (por defecto, la de método del tipo de problema).
        #[arg(long)]
        campo: Option<String>,
        /// Valores a probar, separados por comas.
        #[arg(long)]
        valores: Option<String>,
        /// Métrica para decidir la mejor (p. ej. MAD).
        #[arg(long)]
        metrica: Option<String>,
        /// Opciones comunes en JSON.
        #[arg(long)]
        opciones: Option<String>,
        /// Muestra la solución de cada variante.
        #[arg(long)]
        pasos: bool,
        /// Guarda las gráficas del informe como PNG en esta carpeta (con --pasos, también las del paso a paso).
        #[arg(long)]
        graficas: Option<PathBuf>,
        /// Con --graficas, guarda también cada gráfica en SVG.
        #[arg(long)]
        svg: bool,
    },
    /// Cursos (.ncourse): crear, validar, empaquetar, instalar y probar temas que hacen profesores y estudiantes.
    Curso {
        #[command(subcommand)]
        cmd: CursoCmd,
    },
    /// Guía de un módulo para aprender o enseñar el tema: qué es, para qué sirve, historia con fuentes, cómo usarlo, industria y más.
    Guia {
        /// Módulo (mrp, lp, qa…); sin módulo, la lista de guías.
        modulo: Option<String>,
    },
    /// Bibliografía de IAQSB en APA 7 (o en BibTeX con --bibtex).
    Fuentes {
        /// Escribe la bibliografía en BibTeX (para LaTeX o Zotero).
        #[arg(long)]
        bibtex: bool,
        /// Solo las referencias que faltan por verificar.
        #[arg(long)]
        pendientes: bool,
    },
    /// Llama a cualquier operación de la API con argumentos JSON (por stdin si se omite).
    Llamar {
        operacion: String,
        argumentos: Option<String>,
    },
}

/// Parámetros del motor de optimización.
#[derive(clap::Args, Debug, Default)]
struct MotorArgs {
    /// Motor: highs (incluido), gurobi, cplex, scip, cbc o glpk (si están instalados).
    #[arg(long)]
    motor: Option<String>,
    /// Segundos como máximo; al llegar, se devuelve la mejor solución encontrada.
    #[arg(long)]
    tiempo: Option<f64>,
    /// Brecha MIP relativa aceptada (0.01 = 1 %).
    #[arg(long)]
    brecha: Option<f64>,
    /// Hilos de cálculo.
    #[arg(long)]
    hilos: Option<u32>,
    /// Método: auto, dual_simplex, primal_simplex, barrier o pdlp.
    #[arg(long)]
    metodo: Option<String>,
    /// Muestra el registro del motor.
    #[arg(long)]
    registro: bool,
}

impl MotorArgs {
    fn json(&self) -> Value {
        let mut o = serde_json::Map::new();
        if let Some(v) = &self.motor {
            o.insert("solver".into(), json!(v));
        }
        if let Some(v) = self.tiempo {
            o.insert("time_limit".into(), json!(v));
        }
        if let Some(v) = self.brecha {
            o.insert("mip_gap".into(), json!(v));
        }
        if let Some(v) = self.hilos {
            o.insert("threads".into(), json!(v));
        }
        if let Some(v) = &self.metodo {
            o.insert("method".into(), json!(v));
        }
        if self.registro {
            o.insert("log".into(), json!(true));
        }
        Value::Object(o)
    }
}

/// Como [`load_model`], pero un libro de Excel que no hizo IAQSB se lee como del módulo dado.
fn load_model_as(path: &Path, module: Option<&str>) -> Result<(String, Value), String> {
    let is_xlsx = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("xlsx"));
    if let (true, Some(m)) = (is_xlsx, module) {
        let bytes = std::fs::read(path).map_err(|e| format!("no se pudo leer {}: {e}", path.display()))?;
        let id = api::module_id(m).unwrap_or_else(|| m.to_string());
        let v = api::xlsx_to_model(&bytes, Some(&id), None).map_err(|e| e.in_lang(lang()).to_string())?;
        return Ok((id, v["model"].clone()));
    }
    load_model(path)
}

fn load_model(path: &Path) -> Result<(String, Value), String> {
    // modelo escrito en el lenguaje algebraico de IAQSB
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if matches!(ext.as_str(), "iaqsb" | "ml") {
        let text = std::fs::read_to_string(path).map_err(|e| format!("no se pudo leer {}: {e}", path.display()))?;
        let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        return Ok(("lp".into(), json!({"format": "iaqsb", "text": text, "name": name})));
    }
    let bytes =
        std::fs::read(path).map_err(|e| format!("no se pudo leer {}: {e}", path.display()))?;
    let is_json = path
        .extension()
        .map(|e| e.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
        || bytes.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'{');
    if is_json {
        let v: Value =
            serde_json::from_slice(&bytes).map_err(|e| format!("JSON no válido: {e}"))?;
        // admite tanto el modelo directo como {module, model}
        if let (Some(m), Some(model)) = (v.get("module").and_then(|x| x.as_str()), v.get("model")) {
            return Ok((m.to_string(), model.clone()));
        }
        let m = api::guess_module(&v).unwrap_or_else(|| "lp".into());
        return Ok((m, v));
    }
    let mut imp = api::import_bytes(&bytes).map_err(|e| e.to_string())?;
    // un MPS/LP sin nombre toma el del archivo
    if imp["model"]["name"].as_str().map_or(true, |s| s.trim().is_empty()) {
        if let Some(stem) = path.file_stem() {
            imp["model"]["name"] = json!(stem.to_string_lossy());
        }
    }
    Ok((
        imp["module"].as_str().unwrap().to_string(),
        imp["model"].clone(),
    ))
}

fn n(x: &Value) -> String {
    match x {
        Value::Number(v) => {
            let f = v.as_f64().unwrap();
            if f == f.trunc() && f.abs() < 1e12 {
                format!("{}", f as i64)
            } else {
                format!("{:.4}", f)
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_string()
            }
        }
        Value::String(s) => s.clone(),
        Value::Null => "-".into(),
        o => o.to_string(),
    }
}

fn table(headers: &[&str], rows: Vec<Vec<String>>) -> String {
    let headers: Vec<String> = headers.iter().map(|h| tr(h)).collect();
    let headers: Vec<&str> = headers.iter().map(|h| h.as_str()).collect();
    let headers = &headers[..];
    let mut w: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    for r in &rows {
        for (i, c) in r.iter().enumerate() {
            w[i] = w[i].max(c.chars().count());
        }
    }
    let line = |cells: Vec<String>| -> String {
        cells
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{:>width$}", c, width = w[i]))
            .collect::<Vec<_>>()
            .join("  ")
    };
    let mut out = line(headers.iter().map(|s| s.to_string()).collect()) + "\n";
    out += &w
        .iter()
        .map(|x| "-".repeat(*x))
        .collect::<Vec<_>>()
        .join("  ");
    out += "\n";
    for r in rows {
        out += &line(r);
        out += "\n";
    }
    out
}

fn print_block(b: &Value, show_steps: bool) {
    match b["type"].as_str().unwrap_or("") {
        "metrics" => {
            for it in b["items"].as_array().unwrap() {
                say!("{}: {}", it["label"].as_str().unwrap_or(""), n(&it["value"]));
            }
            say!();
        }
        "text" => say!("{}\n", b["body"].as_str().unwrap_or("")),
        "table" => {
            if let Some(t) = b["title"].as_str() {
                say!("{t}");
            }
            let heads: Vec<String> = b["columns"].as_array().unwrap().iter().map(|c| c["label"].as_str().unwrap_or("").to_string()).collect();
            let rows: Vec<Vec<String>> = b["rows"].as_array().unwrap().iter().map(|r| r["cells"].as_array().unwrap().iter().map(n).collect()).collect();
            let h: Vec<&str> = heads.iter().map(|s| s.as_str()).collect();
            println!("{}", table(&h, rows));
        }
        "steps" if show_steps => {
            for s in b["steps"].as_array().unwrap() {
                say!("── {} ──", s["title"].as_str().unwrap_or(""));
                if let Some(d) = s["description"].as_str() {
                    say!("{d}\n");
                }
                for inner in s["blocks"].as_array().unwrap() {
                    print_block(inner, show_steps);
                }
            }
        }
        "sources" => {
            let plain = |v: &Value| v.as_str().unwrap_or("").replace('*', "");
            if let Some(d) = b.get("data").filter(|d| !d.is_null()) {
                say!("Origen de los datos: {} — {}", plain(&d["origin_label"]), plain(&d["summary"]));
                if let Some(e) = d["expected"].as_str() {
                    say!("  Resultado en la fuente: {e}");
                }
                if let Some(e) = d["note"].as_str() {
                    say!("  {e}");
                }
                if let Some(e) = d["pending"].as_str() {
                    say!("  Por verificar: {e}");
                }
                say!();
            }
            let refs = b["refs"].as_array().cloned().unwrap_or_default();
            if !refs.is_empty() {
                say!("Métodos, fórmulas y pasos:");
                for c in &refs {
                    for u in c["used_for"].as_array().unwrap() {
                        say!("  • {} ({})", plain(u), plain(&c["cite"]));
                    }
                }
                say!("\nReferencias (APA 7):");
                let mut all: Vec<&Value> = refs.iter().collect();
                if let Some(d) = b.get("data").and_then(|d| d["refs"].as_array()) {
                    for c in d {
                        if !all.iter().any(|x| x["id"] == c["id"]) {
                            all.push(c);
                        }
                    }
                }
                all.sort_by_key(|c| plain(&c["apa"]).to_lowercase());
                for c in all {
                    let pend = if c["verified"].as_bool() == Some(false) { "  [por verificar]" } else { "" };
                    say!("  {}{pend}", plain(&c["apa"]));
                }
                say!();
            }
        }
        "network" | "chart" | "gantt" | "tree" | "map" | "layout" => {
            if let Some(t) = b["title"].as_str() {
                say!("[{t}: se ve en la aplicación de escritorio]\n");
            }
        }
        _ => {}
    }
}

/// Guarda las gráficas del informe en `dir` como PNG (y SVG si se pide).
fn save_figures(r: &Value, dir: &std::path::Path, steps: bool, svg: bool, quiet: bool) -> Result<(), String> {
    let (figs, _) = render::figures(r, render::Pick { steps, max: usize::MAX });
    if figs.is_empty() {
        if !quiet {
            say!("\nEste informe no tiene gráficas.");
        }
        return Ok(());
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("no se pudo crear {}: {e}", dir.display()))?;
    let slug = |t: &str| {
        let mut o = String::new();
        for c in t.to_lowercase().chars() {
            let c = match c {
                'á' => 'a',
                'é' => 'e',
                'í' => 'i',
                'ó' | 'ö' => 'o',
                'ú' | 'ü' => 'u',
                'ñ' => 'n',
                c => c,
            };
            if c.is_ascii_alphanumeric() {
                o.push(c);
            } else if !o.ends_with('-') {
                o.push('-');
            }
        }
        o.trim_matches('-').chars().take(60).collect::<String>()
    };
    if !quiet {
        say!("\nGráficas guardadas en {}:", dir.display());
    }
    for (k, f) in figs.iter().enumerate() {
        let name = format!("{:02}-{}", k + 1, slug(&f.place));
        let f_svg = i18n::translate_svg(&f.svg, lang());
        let png = render::png(&f_svg, 2.0)?;
        std::fs::write(dir.join(format!("{name}.png")), png).map_err(|e| e.to_string())?;
        if svg {
            std::fs::write(dir.join(format!("{name}.svg")), &f_svg).map_err(|e| e.to_string())?;
        }
        if !quiet {
            say!("  {name}.png  ·  {}", f.title);
        }
    }
    Ok(())
}

fn print_report(r: &Value, show_steps: bool) {
    say!("IAQSB · {}  ({})", r["problem"].as_str().unwrap_or(""), api::TAGLINE);
    say!("{}\n", r["headline"].as_str().unwrap_or(""));
    let e = &r["engine"];
    if e.is_object() {
        let mut parts = vec![
            tr(&format!("Motor: {}", e["solver"].as_str().unwrap_or(""))),
            tr(&format!("{} resoluciones", e["runs"])),
            format!("{:.3} s", e["time"].as_f64().unwrap_or(0.0)),
        ];
        if let Some(g) = e["gap"].as_f64() {
            parts.push(tr(&format!("brecha {:.4} %", g * 100.0)));
        }
        if e["limited"].as_bool() == Some(true) {
            parts.push(tr("se alcanzó un límite (la solución puede no ser óptima)"));
        }
        println!("{}\n", parts.join(" · "));
    }
    for s in r["sections"].as_array().unwrap() {
        if s["id"] == "pasos" && !show_steps {
            continue;
        }
        say!("═══ {} ═══", s["title"].as_str().unwrap_or(""));
        for b in s["blocks"].as_array().unwrap() {
            print_block(b, show_steps);
        }
    }
}

/// Motor usado, diagnóstico de infactibilidad, avisos y registro (si los hay).
fn print_engine(r: &Value) {
    let e = &r["engine"];
    if e.is_object() {
        let mut parts = vec![format!("Motor: {}", e["solver"].as_str().unwrap_or(""))];
        if let Some(t) = e["time"].as_f64() {
            parts.push(format!("{t:.3} s"));
        }
        if let (Some(c), Some(f)) = (e["cols"].as_u64(), e["rows"].as_u64()) {
            parts.push(format!("{c} variables × {f} restricciones"));
        }
        if e["is_mip"].as_bool() == Some(true) {
            parts.push(format!("{} nodos", e["nodes"]));
            if let Some(g) = e["gap"].as_f64() {
                parts.push(format!("brecha {:.4} %", g * 100.0));
            }
            if let Some(b) = e["bound"].as_f64() {
                parts.push(format!("cota {}", n(&json!(b))));
            }
        } else if let Some(i) = e["iterations"].as_u64() {
            parts.push(format!("{i} iteraciones"));
        }
        let parts: Vec<String> = parts.iter().map(|p| tr(p)).collect();
        println!("{}\n", parts.join(" · "));
    }
    if let Some(d) = r.get("diagnosis").filter(|d| d.is_object()) {
        say!("═══ Por qué no es factible ═══");
        say!("{}", d["summary"].as_str().unwrap_or(""));
        for c in d["conflict"].as_array().into_iter().flatten() {
            say!("  ✗ {}", c["text"].as_str().unwrap_or(""));
        }
        println!();
    }
    for (name, v) in r["objective_values"].as_array().into_iter().flatten().filter_map(|p| Some((p.get(0)?.as_str()?, p.get(1)?))) {
        say!("Objetivo «{}» = {}", name, n(v));
    }
    for note in r["notes"].as_array().into_iter().flatten() {
        say!("Nota: {}", note.as_str().unwrap_or(""));
    }
    if let Some(log) = r["log"].as_str() {
        say!("═══ Registro del motor ═══");
        println!("{log}");
    }
}

fn print_lp_report(r: &Value) {
    say!(
        "IAQSB · {}  ({})",
        r["problem"].as_str().unwrap_or(""),
        api::TAGLINE
    );
    say!("{}\n", r["message"].as_str().unwrap_or(""));
    if let Some(trace) = r["simplex_trace"].as_array() {
        for t in trace {
            say!(
                "Iteración {}  entra: {}  sale: {}",
                t["iteration"],
                n(&t["entering"]),
                n(&t["leaving"])
            );
            let cols: Vec<String> = t["columns"].as_array().unwrap().iter().map(n).collect();
            let mut headers = vec!["Base"];
            headers.extend(cols.iter().map(|s| s.as_str()));
            headers.push("b");
            let mut rows = Vec::new();
            for (i, row) in t["rows"].as_array().unwrap().iter().enumerate() {
                let mut cells = vec![n(&t["basis"][i])];
                cells.extend(row.as_array().unwrap().iter().map(n));
                cells.push(n(&t["rhs"][i]));
                rows.push(cells);
            }
            let mv = |v: &Value| {
                let (m, c) = (v["m"].as_f64().unwrap(), v["c"].as_f64().unwrap());
                if m == 0.0 {
                    n(&json!(c))
                } else if c == 0.0 {
                    format!("{}M", n(&json!(m)))
                } else {
                    format!("{}M{:+}", n(&json!(m)), c)
                }
            };
            let mut cz = vec!["C(j)-Z(j)".to_string()];
            cz.extend(t["cj_minus_zj"].as_array().unwrap().iter().map(mv));
            cz.push(mv(&t["objective"]));
            rows.push(cz);
            println!("{}", table(&headers, rows));
        }
    }
    print_engine(r);
    if matches!(r["status"].as_str(), Some("infeasible" | "unbounded" | "error"))
        || r["variables"].as_array().map_or(true, |v| v.is_empty())
    {
        return;
    }
    let sens = r["sensitivity"].as_bool().unwrap_or(false);
    let rows = r["variables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            let mut c = vec![
                n(&v["name"]),
                n(&v["value"]),
                n(&v["cost"]),
                n(&v["contribution"]),
                n(&v["reduced_cost"]),
                n(&v["basis_status"]).replace('_', " "),
            ];
            if sens {
                c.push(n(&v["cost_min"]));
                c.push(n(&v["cost_max"]));
            }
            c
        })
        .collect();
    let mut h = vec![
        "Variable",
        "Valor",
        "c(j)",
        "Contribución",
        "Coste reducido",
        "Estado",
    ];
    if sens {
        h.extend(["Min c(j)", "Max c(j)"]);
    }
    println!("{}", table(&h, rows));
    say!(
        "Objetivo ({}) = {}\n",
        r["objective"].as_str().unwrap_or(""),
        n(&r["objective_value"])
    );
    let rows = r["constraints"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            let mut x = vec![
                n(&c["name"]),
                n(&c["lhs"]),
                n(&c["direction"]),
                n(&c["rhs"]),
                n(&c["slack_or_surplus"]),
            ];
            if sens {
                x.extend([n(&c["shadow_price"]), n(&c["rhs_min"]), n(&c["rhs_max"])]);
            }
            x
        })
        .collect();
    let mut h = vec![
        "Restricción",
        "Lado izq.",
        "Sentido",
        "Lado der.",
        "Holgura/Exceso",
    ];
    if sens {
        h.extend(["Precio sombra", "Min LD", "Max LD"]);
    }
    println!("{}", table(&h, rows));
    // modelo algebraico: la solución por índices
    for t in r["tables"].as_array().into_iter().flatten() {
        print_block(t, false);
    }
    if let Some(b) = r.get("branch_and_bound").filter(|b| !b.is_null()) {
        say!(
            "Branch and bound: {} nodos, óptimo relajado {}",
            b["nodes_explored"],
            n(&b["relaxed_objective"])
        );
        if let Some(tr) = b["trace"].as_array() {
            for t in tr {
                let br: Vec<String> = t["branches"].as_array().unwrap().iter().map(n).collect();
                say!(
                    "  nodo {:>3}  [{}]  z = {}  {}",
                    t["node"],
                    br.join(", "),
                    n(&t["objective_value"]),
                    n(&t["action"])
                );
            }
        }
    }
    // fuentes: métodos citados y origen de los datos
    for s in r["sections"].as_array().into_iter().flatten() {
        say!("═══ {} ═══", s["title"].as_str().unwrap_or(""));
        for b in s["blocks"].as_array().into_iter().flatten() {
            print_block(b, false);
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    let out = |v: &Value| {
        let mut v = v.clone();
        i18n::translate_value(&mut v, lang());
        println!("{}", serde_json::to_string_pretty(&v).unwrap())
    };
    let out_raw = |v: &Value| println!("{}", serde_json::to_string_pretty(v).unwrap());
    match cli.cmd {
        Cmd::Modulos => {
            let mut v = api::list_modules();
            i18n::translate_value(&mut v, lang());
            if cli.json {
                out(&v);
            } else {
                say!("IAQSB {} · {}\n", api::VERSION, api::TAGLINE);
                for m in v["modules"].as_array().unwrap() {
                    let mark = if m["available"].as_bool().unwrap() {
                        "disponible"
                    } else {
                        "en desarrollo"
                    };
                    say!(
                        "  {:<5} {:<9} {:<46} {}",
                        m["id"].as_str().unwrap(),
                        m["winqsb"].as_str().unwrap(),
                        m["name"].as_str().unwrap(),
                        mark
                    );
                }
            }
        }
        Cmd::Describir { modulo } => {
            out(&api::describe_module(&modulo).map_err(|e| e.to_string())?)
        }
        Cmd::Resolver {
            archivo,
            modulo,
            pasos,
            nodos,
            opciones,
            graficas,
            svg,
            motor,
        } => {
            let (m, model) = load_model(&archivo)?;
            let mut opts: Value = match opciones {
                Some(s) => {
                    serde_json::from_str(&s).map_err(|e| format!("opciones no válidas: {e}"))?
                }
                None => json!({}),
            };
            // parámetros del motor: en LP-ILP van en las opciones; en los demás, en options.engine
            let engine = motor.json();
            if engine.as_object().is_some_and(|o| !o.is_empty()) {
                let target = modulo.as_deref().unwrap_or(&m);
                if target == "lp" {
                    for (k, v) in engine.as_object().unwrap() {
                        opts[k] = v.clone();
                    }
                } else {
                    opts["engine"] = engine;
                }
            }
            if pasos {
                opts["trace"] = json!(true);
                opts["steps"] = json!(true);
            }
            if nodos {
                opts["bnb_trace"] = json!(true);
            }
            let mut r = api::solve(modulo.as_deref().unwrap_or(&m), model, Some(opts))
                .map_err(|e| e.in_lang(lang()).to_string())?;
            i18n::translate_value(&mut r, lang());
            if cli.md {
                println!("{}", i18n::translate_markdown(&render::digest::markdown(&r, &render::digest::DigestOpts { detail: if pasos { render::digest::Detail::Steps } else { render::digest::Detail::Summary }, ..Default::default() }), lang()));
            } else if cli.json {
                out(&r)
            } else if r.get("variables").is_some() && r.get("objective_value").is_some() {
                print_lp_report(&r)
            } else {
                print_report(&r, pasos)
            }
            if let Some(dir) = graficas {
                save_figures(&r, &dir, pasos, svg, cli.json)?;
            }
        }
        Cmd::Motores => {
            let v = api::solver_info();
            if cli.json {
                out(&v);
            } else {
                for x in v["solvers"].as_array().into_iter().flatten() {
                    say!("  {:<7} {:<16} {:<28} {}", x["id"].as_str().unwrap_or(""), x["name"].as_str().unwrap_or(""), x["problems"].as_str().unwrap_or(""), x["via"].as_str().unwrap_or(""));
                }
                say!("\nÚsalo con --motor gurobi (por ejemplo) al resolver.");
            }
        }
        Cmd::Convertir { entrada, salida, modulo } => {
            let (m, model) = load_model_as(&entrada, modulo.as_deref())?;
            let ext = salida.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            if ext == "xlsx" {
                let bytes = api::model_to_xlsx(&m, model, Some(lang().code())).map_err(|e| e.in_lang(lang()).to_string())?;
                std::fs::write(&salida, bytes).map_err(|e| e.to_string())?;
            } else if ext == "json" && m != "lp" {
                std::fs::write(&salida, serde_json::to_vec_pretty(&json!({"module": m, "model": model})).unwrap()).map_err(|e| e.to_string())?;
            } else if m != "lp" {
                let bytes = api::export_bytes(&m, model, None).map_err(|e| e.to_string())?;
                std::fs::write(&salida, bytes).map_err(|e| e.to_string())?;
            } else {
                let to = match ext.as_str() {
                    "lpp" => "lpp",
                    "json" => "json",
                    "lp" => "lp",
                    "iaqsb" | "ml" => "iaqsb",
                    _ => "mps",
                };
                let v = api::convert_lp(model, to).map_err(|e| e.in_lang(lang()).to_string())?;
                let content = match v.get("content").and_then(|c| c.as_str()) {
                    Some(c) if to == "lpp" => api::formats::encode_ansi(c),
                    Some(c) => c.as_bytes().to_vec(),
                    None => serde_json::to_vec_pretty(&v["model"]).unwrap(),
                };
                std::fs::write(&salida, content).map_err(|e| e.to_string())?;
            }
            if !cli.json {
                say!("Guardado en {}", salida.display());
            }
        }
        Cmd::Informe { archivo, salida, pasos, autor, curso, institucion, coma, modulo, opciones, motor } => {
            let (m, model) = load_model_as(&archivo, modulo.as_deref())?;
            let mut opts: Value = match opciones {
                Some(s) => serde_json::from_str(&s).map_err(|e| format!("opciones no válidas: {e}"))?,
                None => json!({}),
            };
            let engine = motor.json();
            if engine.as_object().is_some_and(|o| !o.is_empty()) {
                if m == "lp" {
                    for (k, v) in engine.as_object().unwrap() {
                        opts[k] = v.clone();
                    }
                } else {
                    opts["engine"] = engine;
                }
            }
            let format = salida.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_else(|| "docx".into());
            let v = api::call(
                "export_report",
                json!({"module": m, "model": model, "options": opts, "format": format, "steps": pasos, "author": autor, "course": curso,
                       "institution": institucion, "decimal_comma": coma, "lang": lang().code()}),
            )
            .map_err(|e| e.to_string())?;
            let bytes = match v.get("base64").and_then(|b| b.as_str()) {
                Some(b) => api::base64_decode(b).map_err(|e| e.to_string())?,
                None => v["content"].as_str().unwrap_or("").as_bytes().to_vec(),
            };
            std::fs::write(&salida, &bytes).map_err(|e| e.to_string())?;
            if cli.json {
                out_raw(&json!({"saved": salida, "bytes": bytes.len()}));
            } else {
                say!("Informe guardado en {} ({} KB)", salida.display(), bytes.len().div_ceil(1024));
            }
        }
        Cmd::Revisar { archivo } => {
            let text = std::fs::read_to_string(&archivo).map_err(|e| format!("no se pudo leer {}: {e}", archivo.display()))?;
            let v = api::call("check_model", json!({"text": text, "lang": lang().code()})).map_err(|e| e.to_string())?;
            if cli.json {
                out_raw(&v);
            } else if v["ok"].as_bool() == Some(true) {
                let st = &v["stats"];
                say!("Modelo correcto: {} variables ({} enteras), {} restricciones, {} coeficientes distintos de cero.", st["variables"], st["integer"], st["constraints"], st["nonzeros"]);
                for r in st["reformulations"].as_array().into_iter().flatten() {
                    say!("  · {}", r.as_str().unwrap_or(""));
                }
            } else {
                for e in v["errors"].as_array().into_iter().flatten() {
                    say!("{}:{}:{}: {}", archivo.display(), e["line"], e["col"], e["message"].as_str().unwrap_or(""));
                }
                return Err(tr("el modelo tiene errores"));
            }
        }
        Cmd::Barrer { archivo, dato, desde, hasta, pasos, modulo } => {
            let (m, model) = load_model_as(&archivo, modulo.as_deref())?;
            let v = api::call("sweep", json!({"module": m, "model": model, "path": dato, "values": {"from": desde, "to": hasta, "steps": pasos}, "lang": lang().code()})).map_err(|e| e.to_string())?;
            if cli.json {
                out_raw(&v);
            } else {
                let rows: Vec<Vec<String>> = v["points"].as_array().into_iter().flatten().map(|p| vec![n(&p["value"]), p["status"].as_str().unwrap_or("").into(), n(&p["metric"]), p["headline"].as_str().unwrap_or("").into()]).collect();
                println!("{}", table(&[&tr("Valor"), &tr("Estado"), &tr("Métrica"), &tr("Resultado")], rows));
            }
        }
        Cmd::Parametrico { archivo, costo, lado, desde, hasta } => {
            let (_, model) = load_model(&archivo)?;
            let (kind, index) = match (costo, lado) {
                (Some(c), _) => ("cost", c),
                (None, Some(l)) => ("rhs", l),
                _ => return Err(tr("indica --costo VARIABLE o --lado RESTRICCIÓN")),
            };
            let idx = index.parse::<u64>().map(|k| json!(k)).unwrap_or(json!(index));
            // valor actual del dato (para el rango por defecto: de 0 al doble)
            let (lista, campo) = if kind == "cost" { ("variables", "cost") } else { ("constraints", "rhs") };
            let actual = model[lista]
                .as_array()
                .and_then(|a| match &idx {
                    Value::Number(k) => a.get(k.as_u64().unwrap_or(0) as usize),
                    other => a.iter().find(|x| x["name"] == *other),
                })
                .and_then(|x| x[campo].as_f64())
                .unwrap_or(0.0);
            let desde = desde.unwrap_or(0.0_f64.min(actual));
            let hasta = hasta.unwrap_or(if actual > 0.0 { 2.0 * actual } else { 10.0 });
            let v = api::call("parametric", json!({"model": model, "kind": kind, "index": idx, "from": desde, "to": hasta, "lang": lang().code()})).map_err(|e| e.to_string())?;
            if cli.json {
                out_raw(&v);
            } else {
                let rows: Vec<Vec<String>> = v["segments"].as_array().into_iter().flatten().map(|s| vec![n(&s["from"]), n(&s["to"]), s["status"].as_str().unwrap_or("").into(), n(&s["slope"]), n(&s["objective_from"]), n(&s["objective_to"])]).collect();
                println!("{}", table(&[&tr("Desde"), &tr("Hasta"), &tr("Estado"), &tr("Pendiente"), &tr("Z al inicio"), &tr("Z al final")], rows));
            }
        }
        Cmd::Ajustar { archivo, tiempo } => {
            let (_, model) = load_model(&archivo)?;
            let v = api::call("tune", json!({"model": model, "budget": tiempo, "lang": lang().code()})).map_err(|e| e.to_string())?;
            if cli.json {
                out_raw(&v);
            } else {
                say!("{}\n", v["summary"].as_str().unwrap_or(""));
                let rows: Vec<Vec<String>> = v["trials"].as_array().into_iter().flatten().map(|t| vec![tr(t["name"].as_str().unwrap_or("")), format!("{:.4}", t["time"].as_f64().unwrap_or(0.0)), if t["same_result"].as_bool() == Some(true) { tr("sí") } else { tr("no") }, format!("×{:.2}", t["speedup"].as_f64().unwrap_or(1.0))]).collect();
                println!("{}", table(&[&tr("Configuración"), &tr("Tiempo (s)"), &tr("Mismo resultado"), &tr("Rapidez")], rows));
                say!("\nParámetros recomendados: {}", v["best"]);
            }
        }
        Cmd::Diagnosticar { archivo } => {
            let (_, model) = load_model(&archivo)?;
            let v = api::diagnose_lp(model).map_err(|e| e.in_lang(lang()).to_string())?;
            if cli.json {
                out(&v);
            } else if v["feasible"].as_bool() == Some(true) {
                say!("El modelo es factible.");
            } else if let Some(d) = v.get("diagnosis") {
                say!("{}", d["summary"].as_str().unwrap_or(""));
                for c in d["conflict"].as_array().into_iter().flatten() {
                    say!("  ✗ {}", c["text"].as_str().unwrap_or(""));
                }
            } else {
                say!("{}", v["message"].as_str().unwrap_or(""));
            }
        }
        Cmd::Importar { archivo } => {
            let bytes = std::fs::read(&archivo).map_err(|e| e.to_string())?;
            out(&api::import_bytes(&bytes).map_err(|e| e.to_string())?);
        }
        Cmd::Exportar {
            modelo,
            salida,
            formato,
            modulo,
        } => {
            let (guess, model) = load_model(&modelo)?;
            let modulo = modulo.unwrap_or(guess);
            let bytes =
                api::export_bytes(&modulo, model, Some(&formato)).map_err(|e| e.to_string())?;
            std::fs::write(&salida, bytes).map_err(|e| e.to_string())?;
            if !cli.json {
                say!("Guardado en {}", salida.display());
            }
        }
        Cmd::Grafico { archivo } => {
            let (_, model) = load_model(&archivo)?;
            // un modelo algebraico o MPS/LP se pasa primero a la hoja (el método gráfico es de 2 variables)
            let model = if model.get("variables").is_some() { model } else { api::convert_lp(model, "json").map_err(|e| e.in_lang(lang()).to_string())?["model"].clone() };
            out(&api::graphic(model).map_err(|e| e.to_string())?);
        }
        Cmd::Ejemplos { nombre } => {
            let ex = api::call("examples", json!({"lang": lang().code()})).unwrap();
            match nombre {
                None if !cli.json => {
                    for e in ex.as_array().unwrap() {
                        let src = e["source"]
                            .as_object()
                            .map(|s| {
                                format!(
                                    "  ← {}: {}{}",
                                    s["origin_label"].as_str().unwrap_or(""),
                                    s["summary"].as_str().unwrap_or(""),
                                    if s["verified"].as_bool() == Some(false) { " (por verificar)" } else { "" }
                                )
                            })
                            .unwrap_or_default();
                        say!(
                            "  {:<4} {}{src}",
                            e["module"].as_str().unwrap(),
                            e["name"].as_str().unwrap()
                        );
                    }
                }
                None => out(&ex),
                Some(nm) => {
                    let e = ex
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|e| e["name"] == nm.as_str())
                        .ok_or("ejemplo no encontrado")?;
                    out(&e["model"]);
                }
            }
        }
        Cmd::Comparar { archivo, campo, valores, metrica, opciones, pasos, graficas, svg } => {
            let (module, model) = load_model(&archivo)?;
            let opts: Option<Value> = match opciones {
                Some(o) => Some(serde_json::from_str(&o).map_err(|e| format!("opciones no válidas: {e}"))?),
                None => None,
            };
            let vals = valores.map(|v| v.split(',').map(|x| json!(x.trim())).collect::<Vec<_>>());
            let mut r = api::compare(&module, model, opts, campo, vals, None, metrica, false).map_err(|e| e.in_lang(lang()).to_string())?;
            i18n::translate_value(&mut r, lang());
            if cli.md {
                println!("{}", i18n::translate_markdown(&render::digest::markdown(&r, &render::digest::DigestOpts { detail: if pasos { render::digest::Detail::Steps } else { render::digest::Detail::Summary }, ..Default::default() }), lang()));
            } else if cli.json {
                out(&r);
            } else {
                print_report(&r, pasos);
            }
            if let Some(dir) = graficas {
                save_figures(&r, &dir, pasos, svg, cli.json)?;
            }
        }
        Cmd::Curso { cmd } => {
            let ver = |v: Value| -> std::result::Result<(), String> {
                if cli.json {
                    out(&v);
                } else {
                    println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                }
                Ok(())
            };
            match cmd {
                CursoCmd::Nuevo { carpeta, id, titulo } => {
                    let v = api::cursos::nuevo(&carpeta, &id, &titulo).map_err(|e| e.to_string())?;
                    if !cli.json {
                        println!("Curso creado en {}. Edítalo, revísalo con `iaqsb curso validar` y pruébalo con `iaqsb curso probar`.", carpeta.display());
                        return Ok(());
                    }
                    ver(v)?;
                }
                CursoCmd::Validar { carpeta } => {
                    let (err, avi) = api::cursos::validar(&carpeta);
                    if cli.json {
                        out(&json!({"ok": err.is_empty(), "errores": err, "avisos": avi}));
                    } else {
                        for e in &err {
                            println!("error: {e}");
                        }
                        for a in &avi {
                            println!("aviso: {a}");
                        }
                        println!("{}", if err.is_empty() { "El curso está bien." } else { "El curso tiene errores." });
                    }
                    if !err.is_empty() {
                        return Err(format!("{} errores", err.len()));
                    }
                }
                CursoCmd::Empaquetar { carpeta, salida } => {
                    let bytes = api::cursos::empaquetar(&carpeta).map_err(|e| e.to_string())?;
                    let id = api::cursos::leer(&carpeta).map(|c| c.def["id"].as_str().unwrap_or("curso").to_string()).unwrap_or_else(|_| "curso".into());
                    let f = salida.unwrap_or_else(|| PathBuf::from(format!("{id}.ncourse")));
                    std::fs::write(&f, &bytes).map_err(|e| format!("{}: {e}", f.display()))?;
                    if cli.json {
                        out(&json!({"archivo": f.to_string_lossy(), "bytes": bytes.len()}));
                    } else {
                        println!("{} ({} KB)", f.display(), bytes.len() / 1024 + 1);
                    }
                }
                CursoCmd::Instalar { ruta } => {
                    let bytes = if ruta.is_dir() { api::cursos::empaquetar(&ruta).map_err(|e| e.to_string())? } else { std::fs::read(&ruta).map_err(|e| format!("{}: {e}", ruta.display()))? };
                    ver(api::cursos::instalar(&bytes).map_err(|e| e.to_string())?)?;
                }
                CursoCmd::Quitar { id } => ver(api::cursos::quitar(&id).map_err(|e| e.to_string())?)?,
                CursoCmd::Lista => ver(api::cursos::lista())?,
                CursoCmd::Probar { carpeta } => {
                    let (err, _) = api::cursos::validar(&carpeta);
                    if !err.is_empty() {
                        return Err(format!("el curso tiene errores:\n- {}", err.join("\n- ")));
                    }
                    let r = api::cursos::probar(&carpeta).map_err(|e| e.to_string())?;
                    if cli.json {
                        out(&r);
                    } else {
                        for t in r.as_array().into_iter().flatten() {
                            println!("{} {} · {}: {}", if t["ok"].as_bool() == Some(true) { "bien " } else { "FALLA" }, t["tema"].as_str().unwrap_or(""), t["ejemplo"].as_str().unwrap_or(""), t["resultado"].as_str().unwrap_or(""));
                        }
                    }
                    if r.as_array().is_some_and(|a| a.iter().any(|t| t["ok"].as_bool() != Some(true))) {
                        return Err("algún ejemplo falló".into());
                    }
                }
            }
        }
        Cmd::Guia { modulo } => match modulo {
            None => {
                for m in api::core::guias::modulos() {
                    println!("{m}");
                }
            }
            Some(m) => {
                let lc = match lang() {
                    Lang::En => "en",
                    Lang::Pt => "pt",
                    _ => "es",
                };
                let g = api::call("guide", json!({"module": m, "lang": lc})).map_err(|e| e.to_string())?;
                if cli.json {
                    out(&g["guide"]);
                } else {
                    println!("{}", g["markdown"].as_str().unwrap_or(""));
                }
            }
        },
        Cmd::Fuentes { bibtex, pendientes } => {
            let mut all = api::bibliography();
            i18n::translate_value(&mut all, lang());
            let list: Vec<&Value> = all
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| !pendientes || r["verified"].as_bool() == Some(false))
                .collect();
            if cli.json {
                out(&json!(list));
            } else if bibtex {
                for r in list {
                    println!("{}", r["bibtex"].as_str().unwrap_or(""));
                }
            } else {
                for r in list {
                    let pend = if r["verified"].as_bool() == Some(false) {
                        format!("\n      Por verificar: {}", r["pending"].as_str().unwrap_or(""))
                    } else {
                        String::new()
                    };
                    say!("[{}] {}{pend}\n", r["id"].as_str().unwrap_or(""), r["apa"].as_str().unwrap_or("").replace('*', ""));
                }
            }
        }
        Cmd::Llamar {
            operacion,
            argumentos,
        } => {
            let txt = match argumentos {
                Some(a) => a,
                None => {
                    let mut s = String::new();
                    std::io::stdin()
                        .read_to_string(&mut s)
                        .map_err(|e| e.to_string())?;
                    s
                }
            };
            let mut args: Value = if txt.trim().is_empty() {
                json!({})
            } else {
                serde_json::from_str(&txt).map_err(|e| e.to_string())?
            };
            // el idioma de la CLI, si la llamada no trae el suyo
            if args.get("lang").is_none() && args.get("language").is_none() && lang() != Lang::Es {
                if let Some(o) = args.as_object_mut() {
                    o.insert("lang".into(), json!(lang().code()));
                }
            }
            match api::call(&operacion, args) {
                Ok(v) => out_raw(&json!({"ok": true, "result": v})),
                Err(e) => {
                    out_raw(
                        &json!({"ok": false, "error": {"code": e.code(), "message": e.to_string()}}),
                    );
                    return Err(String::new());
                }
            }
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    // al encadenar con `head` u otro programa que cierra la tubería, terminar sin error
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = info.payload().downcast_ref::<String>().map(String::as_str).or_else(|| info.payload().downcast_ref::<&str>().copied()).unwrap_or("");
        if msg.contains("failed printing to stdout") {
            std::process::exit(0);
        }
        default(info);
    }));
    let cli = Cli::parse();
    if cli.confiar {
        // SAFETY: al principio, antes de crear hilos.
        unsafe { std::env::set_var("IAQSB_CONFIAR_CODIGO", "1") };
    }
    let _ = LANG.set(Lang::parse(cli.lang.clone().or_else(|| std::env::var("IAQSB_LANG").ok()).as_deref()));
    let json = cli.json;
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if e.is_empty() => ExitCode::FAILURE,
        Err(e) => {
            if json {
                println!("{}", json!({"ok": false, "error": {"message": e}}));
            } else {
                eprintln!("Error: {}", tr(&e));
            }
            ExitCode::FAILURE
        }
    }
}
