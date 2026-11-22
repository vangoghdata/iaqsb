//! Peticiones HTTP del asistente de IA hechas desde Rust.
//!
//! La ventana no puede hablar directamente con cualquier proveedor de modelos: la
//! política de seguridad de la página (CSP) solo deja salir a dominios conocidos y
//! muchos proveedores (o un servidor local como Ollama) no aceptan peticiones desde
//! una página web (CORS). Por eso la interfaz le pide la petición a Rust y recibe la
//! respuesta por partes (streaming) en un canal: primero la cabecera, después el
//! texto a medida que llega y al final «fin» o «error».
//!
//! La clave de la API viaja en las cabeceras de la petición y no se guarda ni se
//! escribe en ningún registro.

use futures_util::StreamExt;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::ipc::Channel;
use tauri::State;

/// Lo que llega por el canal.
#[derive(Clone, Serialize, Debug, PartialEq)]
#[serde(tag = "t", rename_all = "lowercase")]
pub enum Trozo {
    Cabecera { status: u16, headers: Vec<(String, String)> },
    Datos { b: String },
    Fin,
    Error { m: String },
}

/// Peticiones en curso (para poder detenerlas).
#[derive(Default)]
pub struct Pedidos(Mutex<HashMap<u32, tauri::async_runtime::JoinHandle<()>>>);

fn cliente() -> &'static reqwest::Client {
    static C: OnceLock<reqwest::Client> = OnceLock::new();
    C.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(20))
            // sin tiempo total: una respuesta larga llega por partes durante minutos
            .read_timeout(Duration::from_secs(300))
            .user_agent(concat!("IAQSB/", env!("CARGO_PKG_VERSION")))
            // sin redirecciones: una a otro dominio podría llevarse la clave (x-api-key) y
            // un POST redirigido llega como GET; el error dice cuál es la dirección final
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("cliente HTTP")
    })
}

/// Solo http y https.
pub fn url_valida(url: &str) -> Result<reqwest::Url, String> {
    let u = reqwest::Url::parse(url.trim()).map_err(|e| format!("Dirección no válida ({url}): {e}"))?;
    match u.scheme() {
        "http" | "https" => Ok(u),
        s => Err(format!("Solo se admiten direcciones http o https (no {s}).")),
    }
}

/// Hace la petición y va mandando los trozos a `enviar`. Devuelve cuando termina o
/// cuando `enviar` dice que ya nadie escucha (devuelve false: la ventana se cerró o recargó).
pub async fn traer(url: &str, metodo: &str, cabeceras: &[(String, String)], cuerpo: Option<String>, enviar: impl Fn(Trozo) -> bool) {
    let u = match url_valida(url) {
        Ok(u) => u,
        Err(m) => {
            enviar(Trozo::Error { m });
            return;
        }
    };
    let metodo = reqwest::Method::from_bytes(metodo.to_ascii_uppercase().as_bytes()).unwrap_or(reqwest::Method::POST);
    let mut req = cliente().request(metodo, u);
    for (k, v) in cabeceras {
        req = req.header(k.as_str(), v.as_str());
    }
    if let Some(c) = cuerpo {
        req = req.body(c);
    }
    let res = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            let m = if e.is_connect() {
                format!("No se pudo conectar con el servidor: {e}")
            } else if e.is_timeout() {
                "El servidor no respondió a tiempo.".to_string()
            } else {
                e.to_string()
            };
            enviar(Trozo::Error { m });
            return;
        }
    };
    let headers = res.headers().iter().map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string())).collect();
    if !enviar(Trozo::Cabecera { status: res.status().as_u16(), headers }) {
        return;
    }
    // UTF-8 por partes: un carácter puede quedar partido entre dos trozos
    let mut resto: Vec<u8> = vec![];
    let mut flujo = res.bytes_stream();
    while let Some(parte) = flujo.next().await {
        match parte {
            Ok(b) => {
                resto.extend_from_slice(&b);
                let hasta = match std::str::from_utf8(&resto) {
                    Ok(_) => resto.len(),
                    Err(e) if e.error_len().is_none() => e.valid_up_to(),
                    // bytes inválidos: se reemplazan y se sigue
                    Err(_) => resto.len(),
                };
                if hasta > 0 {
                    let txt = String::from_utf8_lossy(&resto[..hasta]).into_owned();
                    resto.drain(..hasta);
                    if !enviar(Trozo::Datos { b: txt }) {
                        return;
                    }
                }
            }
            Err(e) => {
                enviar(Trozo::Error { m: format!("Se cortó la conexión: {e}") });
                return;
            }
        }
    }
    if !resto.is_empty() {
        enviar(Trozo::Datos { b: String::from_utf8_lossy(&resto).into_owned() });
    }
    enviar(Trozo::Fin);
}

/// Empieza una petición; la respuesta llega por `canal`. `id` sirve para detenerla.
#[tauri::command]
pub fn ia_fetch(id: u32, url: String, metodo: Option<String>, cabeceras: Vec<(String, String)>, cuerpo: Option<String>, canal: Channel<Trozo>, pedidos: State<'_, Pedidos>) -> Result<(), String> {
    url_valida(&url)?;
    let tarea = tauri::async_runtime::spawn(async move {
        traer(&url, metodo.as_deref().unwrap_or("POST"), &cabeceras, cuerpo, |t| canal.send(t).is_ok())
        .await;
    });
    let mut m = pedidos.0.lock().map_err(|e| e.to_string())?;
    // las que ya terminaron no hace falta guardarlas
    m.retain(|_, h| !h.inner().is_finished());
    m.insert(id, tarea);
    Ok(())
}

/// Detiene una petición en curso (botón «Detener» del asistente).
#[tauri::command]
pub fn ia_cancel(id: u32, pedidos: State<'_, Pedidos>) {
    if let Some(h) = pedidos.0.lock().ok().and_then(|mut m| m.remove(&id)) {
        h.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;

    /// Servidor HTTP mínimo que responde con eventos SSE en varios trozos (con un
    /// carácter de varios bytes partido entre dos) y devuelve lo que recibió.
    fn servidor() -> (String, std::thread::JoinHandle<String>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let dir = format!("http://{}/v1/chat/completions", l.local_addr().unwrap());
        let h = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut buf = vec![0u8; 65536];
            let mut pedido = vec![];
            loop {
                let n = s.read(&mut buf).unwrap();
                pedido.extend_from_slice(&buf[..n]);
                let txt = String::from_utf8_lossy(&pedido).to_string();
                if let Some(p) = txt.find("\r\n\r\n") {
                    let largo: usize = txt.lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|x| x.trim().parse().unwrap())).unwrap_or(0);
                    if pedido.len() >= p + 4 + largo {
                        break;
                    }
                }
            }
            s.write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\r\n").unwrap();
            let eventos: Vec<&[u8]> = vec![b"data: {\"x\":\"opti\xc3", b"\xb3n\"}\n\n", b"data: [DONE]\n\n"];
            for e in eventos {
                s.write_all(format!("{:x}\r\n", e.len()).as_bytes()).unwrap();
                s.write_all(e).unwrap();
                s.write_all(b"\r\n").unwrap();
                s.flush().unwrap();
                std::thread::sleep(Duration::from_millis(30));
            }
            s.write_all(b"0\r\n\r\n").unwrap();
            String::from_utf8_lossy(&pedido).to_string()
        });
        (dir, h)
    }

    #[test]
    fn trae_por_partes_y_manda_las_cabeceras() {
        let (dir, h) = servidor();
        let trozos = Arc::new(Mutex::new(vec![]));
        let t2 = trozos.clone();
        let rt = tokio_rt();
        rt.block_on(traer(&dir, "post", &[("Authorization".into(), "Bearer prueba".into()), ("content-type".into(), "application/json".into())], Some("{\"a\":1}".into()), move |t| {
            t2.lock().unwrap().push(t);
            true
        }));
        let pedido = h.join().unwrap();
        assert!(pedido.starts_with("POST /v1/chat/completions"), "{pedido}");
        assert!(pedido.to_ascii_lowercase().contains("authorization: bearer prueba"));
        assert!(pedido.ends_with("{\"a\":1}"));
        let v = trozos.lock().unwrap().clone();
        assert!(matches!(&v[0], Trozo::Cabecera { status: 200, .. }));
        assert_eq!(v.last(), Some(&Trozo::Fin));
        let texto: String = v.iter().filter_map(|t| if let Trozo::Datos { b } = t { Some(b.as_str()) } else { None }).collect();
        assert_eq!(texto, "data: {\"x\":\"optión\"}\n\ndata: [DONE]\n\n");
        // ningún trozo trae el carácter partido
        assert!(v.iter().all(|t| !matches!(t, Trozo::Datos { b } if b.contains('\u{FFFD}'))));
    }

    #[test]
    fn rechaza_esquemas_que_no_son_http() {
        assert!(url_valida("file:///etc/passwd").is_err());
        assert!(url_valida("javascript:alert(1)").is_err());
        assert!(url_valida("https://openrouter.ai/api/v1/chat/completions").is_ok());
        assert!(url_valida("http://localhost:11434/v1/models").is_ok());
    }

    #[test]
    fn sin_servidor_da_error_y_no_cabecera() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let dir = format!("http://{}/", l.local_addr().unwrap());
        drop(l);
        let trozos = Arc::new(Mutex::new(vec![]));
        let t2 = trozos.clone();
        tokio_rt().block_on(traer(&dir, "GET", &[], None, move |t| {
            t2.lock().unwrap().push(t);
            true
        }));
        let v = trozos.lock().unwrap().clone();
        assert_eq!(v.len(), 1);
        assert!(matches!(&v[0], Trozo::Error { .. }));
    }

    #[test]
    fn no_sigue_redirecciones_ni_sigue_si_nadie_escucha() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let dir = format!("http://{}/v1/messages", l.local_addr().unwrap());
        let h = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut buf = [0u8; 4096];
            let _ = s.read(&mut buf).unwrap();
            s.write_all(b"HTTP/1.1 302 Found\r\nlocation: https://otro.example/robar\r\ncontent-length: 0\r\n\r\n").unwrap();
        });
        let trozos = Arc::new(Mutex::new(vec![]));
        let t2 = trozos.clone();
        tokio_rt().block_on(traer(&dir, "POST", &[("x-api-key".into(), "secreta".into())], Some("{}".into()), move |t| {
            t2.lock().unwrap().push(t);
            // después de la cabecera ya nadie escucha: no se manda nada más
            false
        }));
        h.join().unwrap();
        let v = trozos.lock().unwrap().clone();
        assert_eq!(v.len(), 1, "{v:?}");
        match &v[0] {
            Trozo::Cabecera { status, headers } => {
                assert_eq!(*status, 302);
                assert!(headers.iter().any(|(k, v)| k == "location" && v.contains("otro.example")));
            }
            otro => panic!("{otro:?}"),
        }
    }

    fn tokio_rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()
    }
}
