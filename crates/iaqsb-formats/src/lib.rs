//! Formatos de archivo de WinQSB 2.0.
//!
//! Los archivos de WinQSB son texto ANSI con campos separados por tabuladores y
//! líneas terminadas en CRLF. La primera línea identifica el módulo, el formato de
//! entrada, el nombre del problema y sus dimensiones. El resto es el contenido de
//! la hoja de datos del programa, fila por fila.

pub mod lpp;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("no es un archivo de {0}")]
    WrongModule(&'static str),
    #[error("archivo incompleto: falta {0}")]
    Truncated(String),
    #[error("dato no válido en {at}: {msg}")]
    Bad { at: String, msg: String },
}

/// Decodifica texto ANSI (Windows-1252 aproximado por Latin-1, que cubre los
/// acentos del español).
pub fn decode_ansi(bytes: &[u8]) -> String {
    // UTF-8 válido con BOM o sin él: se respeta (archivos editados a mano)
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.trim_start_matches('\u{feff}').to_string();
    }
    bytes.iter().map(|&b| b as char).collect()
}

/// Codifica en Latin-1; los caracteres fuera de rango se sustituyen por '?'.
pub fn encode_ansi(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| if (c as u32) < 256 { c as u8 } else { b'?' })
        .collect()
}

/// Separa una línea en campos como la función de WinQSB: por tabuladores y
/// quitando las comillas que rodean un campo.
pub fn fields(line: &str) -> Vec<String> {
    line.trim_end_matches(['\r', '\n'])
        .split('\t')
        .map(|f| {
            let f = f.strip_prefix('"').unwrap_or(f);
            let f = f.strip_suffix('"').unwrap_or(f);
            f.to_string()
        })
        .collect()
}
