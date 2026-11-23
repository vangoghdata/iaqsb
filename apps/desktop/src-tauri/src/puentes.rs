//! Puentes entre el núcleo y la ventana:
//!
//! - **R dentro de la app (webR).** Si el equipo no tiene R con forecast, el módulo de
//!   series de tiempo le pide a la ventana que ejecute el programa de R con webR (el R
//!   oficial compilado a WebAssembly, con los mismos paquetes de CRAN): mismos resultados
//!   que R sin instalar nada. El núcleo corre en un hilo aparte y espera la respuesta.
//! - **Conversaciones del asistente** guardadas en la carpeta de datos de la app, para que
//!   sigan ahí aunque se cierre el programa.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

static SIGUIENTE: AtomicU64 = AtomicU64::new(1);
static PENDIENTES: Mutex<Option<HashMap<u64, Sender<Result<String, String>>>>> = Mutex::new(None);

/// Registra webR como el R del núcleo (cuando el equipo no tiene R con forecast).
pub fn registrar_webr(app: AppHandle) {
    iaqsb_api::modules::st::motores::registrar_r_externo(
        "webR, dentro de IAQSB",
        Box::new(move |programa: &str| {
            let id = SIGUIENTE.fetch_add(1, Ordering::Relaxed);
            let (tx, rx) = channel();
            PENDIENTES.lock().map_err(|e| e.to_string())?.get_or_insert_with(HashMap::new).insert(id, tx);
            if let Err(e) = app.emit("r-ejecutar", json!({ "id": id, "programa": programa })) {
                quitar(id);
                return Err(format!("no se pudo pedir R a la ventana: {e}"));
            }
            // la primera vez descarga R y forecast (unos 30 MB); después tarda segundos
            match rx.recv_timeout(Duration::from_secs(900)) {
                Ok(r) => r,
                Err(_) => {
                    quitar(id);
                    Err("el R de la app no respondió a tiempo".into())
                }
            }
        }),
    );
}

fn quitar(id: u64) -> Option<Sender<Result<String, String>>> {
    PENDIENTES.lock().ok()?.as_mut()?.remove(&id)
}

/// La ventana devuelve lo que escribió R (o el error).
#[tauri::command]
pub fn r_resultado(id: u64, ok: bool, salida: String) {
    if let Some(tx) = quitar(id) {
        let _ = tx.send(if ok { Ok(salida) } else { Err(salida) });
    }
}

// ------------------------------------------------------------------ conversaciones

fn carpeta(app: &AppHandle) -> Result<PathBuf, String> {
    let d = app.path().app_data_dir().map_err(|e| e.to_string())?.join("conversaciones");
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d)
}

/// Solo letras, números, guion y guion bajo (el id lo genera la app).
fn id_valido(id: &str) -> Result<&str, String> {
    if !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        Ok(id)
    } else {
        Err("identificador de conversación no válido".into())
    }
}

/// Guarda una conversación: `datos` es el JSON completo; `resumen`, lo que se lista.
#[tauri::command]
pub async fn conversacion_guardar(app: AppHandle, id: String, datos: String, resumen: Value) -> Result<(), String> {
    let d = carpeta(&app)?;
    let id = id_valido(&id)?.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let tmp = d.join(format!("{id}.json.tmp"));
        std::fs::write(&tmp, datos.as_bytes()).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, d.join(format!("{id}.json"))).map_err(|e| e.to_string())?;
        std::fs::write(d.join(format!("{id}.resumen.json")), resumen.to_string()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn conversacion_leer(app: AppHandle, id: String) -> Result<String, String> {
    let d = carpeta(&app)?;
    std::fs::read_to_string(d.join(format!("{}.json", id_valido(&id)?))).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn conversacion_borrar(app: AppHandle, id: String) -> Result<(), String> {
    let d = carpeta(&app)?;
    let id = id_valido(&id)?;
    let _ = std::fs::remove_file(d.join(format!("{id}.resumen.json")));
    std::fs::remove_file(d.join(format!("{id}.json"))).or_else(|e| if e.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(e.to_string()) })
}

/// Resúmenes de las conversaciones guardadas (las más recientes primero).
#[tauri::command]
pub fn conversaciones_listar(app: AppHandle) -> Result<Vec<Value>, String> {
    let d = carpeta(&app)?;
    let mut v: Vec<Value> = std::fs::read_dir(&d)
        .map_err(|e| e.to_string())?
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".resumen.json"))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .filter_map(|s| serde_json::from_str::<Value>(&s).ok())
        .filter(|r| r.get("id").and_then(|x| x.as_str()).is_some_and(|id| d.join(format!("{id}.json")).is_file()))
        .collect();
    v.sort_by(|a, b| b["fecha"].as_f64().unwrap_or(0.0).partial_cmp(&a["fecha"].as_f64().unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal));
    Ok(v)
}

/// Carpeta donde se guardan (para abrirla desde la app).
#[tauri::command]
pub fn conversaciones_carpeta(app: AppHandle) -> Result<String, String> {
    Ok(carpeta(&app)?.to_string_lossy().to_string())
}
