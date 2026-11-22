//! IAQSB · aplicación de escritorio (Tauri 2).
//!
//! La interfaz web llama a estos comandos; todos delegan en iaqsb-api, el mismo
//! núcleo que usan la línea de comandos y el servidor MCP.

mod red;

use iaqsb_api as api;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Corre un trabajo del núcleo fuera del hilo de la ventana (así la interfaz no
/// se congela mientras se resuelve, se traduce o se lee un archivo).
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn api_call(op: String, args: Value) -> Result<Value, String> {
    blocking(move || api::call(&op, args).map_err(|e| e.to_string())).await
}

fn is_json(path: &Path) -> bool {
    path.extension().map(|e| e.eq_ignore_ascii_case("json")).unwrap_or(false)
}

fn read_sync(path: &str) -> Result<Value, String> {
    let p = PathBuf::from(path);
    let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    // modelo en el lenguaje algebraico de IAQSB
    if matches!(ext.as_str(), "iaqsb" | "ml") {
        let text = std::fs::read_to_string(&p).map_err(|e| format!("No se pudo leer {path}: {e}"))?;
        let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        return Ok(json!({"module": "lp", "layout": "matrix", "algebra": true, "model": {"format": "iaqsb", "text": text, "name": name}}));
    }
    let bytes = std::fs::read(&p).map_err(|e| format!("No se pudo leer {path}: {e}"))?;
    if is_json(&p) {
        let v: Value = serde_json::from_slice(&bytes).map_err(|e| format!("JSON no válido: {e}"))?;
        let model = v.get("model").cloned().unwrap_or_else(|| v.clone());
        let module = v.get("module").cloned().unwrap_or(json!("lp"));
        return Ok(json!({"module": module, "layout": "matrix", "model": model}));
    }
    api::import_bytes(&bytes).map_err(|e| e.to_string())
}

/// Abre un archivo de WinQSB (.LPP…), un modelo JSON de IAQSB, un MPS/LP o un libro de
/// Excel. Un libro que no hizo IAQSB se lee como del módulo (y tipo de problema) dados.
#[tauri::command]
async fn read_file(path: String, module: Option<String>, problem_type: Option<String>) -> Result<Value, String> {
    blocking(move || {
        let is_xlsx = path.to_lowercase().ends_with(".xlsx");
        match read_sync(&path) {
            Err(e) if is_xlsx && module.is_some() => {
                let bytes = std::fs::read(&path).map_err(|e| format!("No se pudo leer {path}: {e}"))?;
                api::xlsx_to_model(&bytes, module.as_deref(), problem_type.as_deref()).map_err(|x| if x.to_string().is_empty() { e } else { x.to_string() })
            }
            r => r,
        }
    })
    .await
}

/// Guarda en formato WinQSB, o en JSON si la ruta termina en .json, o en Excel si termina en .xlsx.
#[tauri::command]
async fn write_file(path: String, module: String, model: Value, layout: Option<String>, lang: Option<String>) -> Result<(), String> {
    blocking(move || {
        let p = PathBuf::from(&path);
        let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let bytes = if is_json(&p) {
            serde_json::to_vec_pretty(&json!({"module": module, "model": model})).unwrap()
        } else if ext == "xlsx" {
            api::model_to_xlsx(&module, model, lang.as_deref()).map_err(|e| e.to_string())?
        } else {
            api::export_bytes(&module, model, layout.as_deref()).map_err(|e| e.to_string())?
        };
        std::fs::write(&p, bytes).map_err(|e| format!("No se pudo guardar {path}: {e}"))
    })
    .await
}

/// Ejecuta una operación de la API que produce un archivo ({"base64"} o {"content"}) y lo guarda.
#[tauri::command]
async fn save_export(path: String, op: String, args: Value) -> Result<(), String> {
    blocking(move || {
        let v = api::call(&op, args).map_err(|e| e.to_string())?;
        let bytes = if let Some(b) = v.get("base64").and_then(|b| b.as_str()) {
            api::base64_decode(b).map_err(|e| e.to_string())?
        } else if let Some(c) = v.get("content").and_then(|c| c.as_str()) {
            c.as_bytes().to_vec()
        } else {
            return Err("la operación no devolvió un archivo".into());
        };
        std::fs::write(&path, bytes).map_err(|e| format!("No se pudo guardar {path}: {e}"))
    })
    .await
}

/// ¿Existe todavía el archivo? (lista de archivos recientes)
#[tauri::command]
fn file_exists(path: String) -> bool {
    Path::new(&path).is_file()
}

/// Guarda una gráfica del informe como PNG o SVG (según la extensión).
#[tauri::command]
async fn save_render(path: String, block: Value, lang: Option<String>) -> Result<(), String> {
    blocking(move || {
        let svg = api::render::svg(&block).ok_or("el bloque no es una gráfica")?;
        // los textos propios de la gráfica (ejes, leyendas) en el idioma de la app
        let svg = api::core::i18n::translate_svg(&svg, api::core::i18n::Lang::parse(lang.as_deref()));
        let bytes = if path.to_lowercase().ends_with(".svg") { svg.into_bytes() } else { api::render::png(&svg, 2.0)? };
        std::fs::write(&path, bytes).map_err(|e| format!("No se pudo guardar {path}: {e}"))
    })
    .await
}

/// Rutas del servidor MCP y de la CLI instalados junto a la aplicación.
#[tauri::command]
fn mcp_info() -> Value {
    let mut dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)).unwrap_or_default();
    let ext = if cfg!(windows) { ".exe" } else { "" };
    // AppImage (Linux): la app corre desde un montaje temporal que cambia en cada
    // arranque; los binarios se copian a una carpeta fija para que Claude los encuentre
    if cfg!(target_os = "linux") && std::env::var_os("APPIMAGE").is_some() {
        if let Some(fijo) = copiar_binarios_fuera_de_appimage(&dir) {
            dir = fijo;
        }
    }
    let server = dir.join(format!("iaqsb-mcp{ext}")).to_string_lossy().to_string();
    let cli = dir.join(format!("iaqsb{ext}")).to_string_lossy().to_string();
    let config = serde_json::to_string_pretty(&json!({"mcpServers": {"iaqsb": {"command": server}}})).unwrap();
    json!({"server_path": server, "cli_path": cli, "config": config})
}

/// Copia `iaqsb` e `iaqsb-mcp` a `~/.local/share/iaqsb/bin` (o `$XDG_DATA_HOME/iaqsb/bin`)
/// y devuelve esa carpeta. Se vuelven a copiar en cada llamada, así quedan al día con la versión.
fn copiar_binarios_fuera_de_appimage(origen: &Path) -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share")))?;
    let destino = base.join("iaqsb").join("bin");
    std::fs::create_dir_all(&destino).ok()?;
    for nombre in ["iaqsb-mcp", "iaqsb"] {
        let de = origen.join(nombre);
        let a = destino.join(nombre);
        // se copia a un archivo aparte y se renombra: así se puede reemplazar aunque
        // Claude tenga abierto el servidor (escribir encima daría «text file busy»)
        let temporal = destino.join(format!("{nombre}.nuevo"));
        std::fs::copy(&de, &temporal).ok()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&temporal, std::fs::Permissions::from_mode(0o755)).ok()?;
        }
        std::fs::rename(&temporal, &a).ok()?;
    }
    Some(destino)
}

/// Archivo con el que se abrió la aplicación (doble clic en un archivo de WinQSB).
#[tauri::command]
fn initial_file() -> Option<Value> {
    let arg = std::env::args().skip(1).find(|a| !a.starts_with('-'))?;
    let p = PathBuf::from(&arg);
    if !p.is_file() {
        return None;
    }
    let mut v = read_sync(&arg).ok()?;
    v["path"] = json!(arg);
    Some(v)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // los catálogos de traducción y los ejemplos traducidos se preparan en segundo
    // plano: cambiar de idioma después es inmediato
    std::thread::spawn(api::warm_up);
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(red::Pedidos::default())
        .invoke_handler(tauri::generate_handler![red::ia_fetch, red::ia_cancel, api_call, read_file, write_file, save_export, file_exists, save_render, mcp_info, initial_file])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar IAQSB");
}
