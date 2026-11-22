//! iaqsb-mcp: servidor MCP (Model Context Protocol) de IAQSB por stdio.
//!
//! Implementa directamente JSON-RPC 2.0 sobre stdin/stdout (un mensaje JSON por
//! línea), sin dependencias más allá de serde_json, para que el binario sea
//! pequeño y estable. Herramientas, recursos (ejemplos) y un prompt de ayuda.

use iaqsb_api as api;
mod tools;
use tools::INSTRUCTIONS;
use serde_json::{json, Value};
use std::io::{BufRead, Write};

const SUPPORTED: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

fn resources() -> Value {
    Value::Array(
        api::examples()
            .into_iter()
            .map(|(m, n, _)| {
                let m = m.as_str();
                let n = n.as_str();
                json!({
                    "uri": format!("iaqsb://ejemplos/{m}/{n}"),
                    "name": n,
                    "title": format!("Ejemplo {m}: {n}"),
                    "description": "Modelo de ejemplo en JSON de IAQSB",
                    "mimeType": "application/json"
                })
            })
            .collect(),
    )
}

fn handle(method: &str, params: &Value) -> Result<Value, (i64, String)> {
    match method {
        "initialize" => {
            let asked = params
                .get("protocolVersion")
                .and_then(|v| v.as_str())
                .unwrap_or(SUPPORTED[0]);
            let version = if SUPPORTED.contains(&asked) {
                asked
            } else {
                SUPPORTED[0]
            };
            Ok(json!({
                "protocolVersion": version,
                "capabilities": {"tools": {"listChanged": false}, "resources": {"listChanged": false}, "prompts": {"listChanged": false}},
                "serverInfo": {"name": "iaqsb", "title": "IAQSB · Hecho por IA Nativa", "version": api::VERSION},
                "instructions": INSTRUCTIONS
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tools::tools()})),
        "tools/call" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            Ok(match tools::call(name, &args) {
                Ok(r) => {
                    let mut out = json!({"content": r.content, "isError": false});
                    if let Some(sc) = r.structured {
                        out["structuredContent"] = sc;
                    }
                    out
                }
                Err(e) => json!({"content": [{"type": "text", "text": e}], "isError": true}),
            })
        }
        "resources/list" => Ok(json!({"resources": resources()})),
        "resources/templates/list" => Ok(json!({"resourceTemplates": []})),
        "resources/read" => {
            let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
            let found = api::examples()
                .into_iter()
                .find(|(m, n, _)| uri == format!("iaqsb://ejemplos/{m}/{n}"));
            match found {
                Some((m, _, model)) => Ok(
                    json!({"contents": [{"uri": uri, "mimeType": "application/json", "text": serde_json::to_string_pretty(&json!({"module": m, "model": model})).unwrap()}]}),
                ),
                None => Err((-32002, format!("recurso no encontrado: {uri}"))),
            }
        }
        "prompts/list" => Ok(json!({"prompts": [{
            "name": "resolver_problema",
            "title": "Resolver un problema de investigación de operaciones",
            "description": "Guía para plantear un problema descrito en palabras y resolverlo con IAQSB",
            "arguments": [{"name": "enunciado", "description": "El problema en palabras", "required": true}]
        }]})),
        "prompts/get" => {
            let e = params
                .pointer("/arguments/enunciado")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            Ok(json!({
                "description": "Plantear y resolver con IAQSB",
                "messages": [{"role": "user", "content": {"type": "text", "text": format!(
                    "Plantea este problema con IAQSB: elige el módulo adecuado (list_modules), pide su esquema con describe_module, \
                    arma el modelo y resuélvelo con solve. Explica la solución con el resumen y los datos del informe y, \
                    si el informe los trae, los recursos que limitan (precios sombra) y qué pasaría si cambian los datos.\n\nProblema:\n{e}")}}]
            }))
        }
        "logging/setLevel" => Ok(json!({})),
        _ => Err((-32601, format!("método no encontrado: {method}"))),
    }
}

fn main() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    eprintln!("iaqsb-mcp {} listo (stdio)", api::VERSION);
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let err = json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": format!("JSON no válido: {e}")}});
                let _ = writeln!(stdout, "{err}");
                let _ = stdout.flush();
                continue;
            }
        };
        let batch = match msg {
            Value::Array(v) => v,
            v => vec![v],
        };
        for m in batch {
            let Some(id) = m.get("id").cloned() else {
                continue;
            }; // notificación
            if m.get("method").is_none() {
                continue; // respuesta a algo que no pedimos
            }
            let method = m["method"].as_str().unwrap_or("");
            let params = m.get("params").cloned().unwrap_or(json!({}));
            let reply = match handle(method, &params) {
                Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
                Err((code, message)) => {
                    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
                }
            };
            let _ = writeln!(stdout, "{reply}");
            let _ = stdout.flush();
        }
    }
}
