//! Utilidades para los archivos de WinQSB 2.0: texto ANSI, campos separados por
//! tabuladores y líneas terminadas en CRLF. Cada módulo guarda una línea de
//! cabecera y después el contenido de su hoja de datos, fila por fila.

use crate::module::ModError;

/// Decodifica texto ANSI (Latin-1, que cubre los acentos del español).
pub fn decode(bytes: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.trim_start_matches('\u{feff}').to_string();
    }
    bytes.iter().map(|&b| b as char).collect()
}

/// Codifica en Latin-1; lo que no cabe se sustituye por '?'.
pub fn encode(s: &str) -> Vec<u8> {
    s.chars().map(|c| if (c as u32) < 256 { c as u8 } else { b'?' }).collect()
}

/// Una línea separada en campos, con lectura secuencial como hace WinQSB.
#[derive(Debug, Clone)]
pub struct Line {
    fields: Vec<String>,
    pos: usize,
}

impl Line {
    pub fn parse(line: &str) -> Line {
        let fields = line
            .trim_end_matches(['\r', '\n'])
            .split('\t')
            .map(|f| {
                let f = f.strip_prefix('"').unwrap_or(f);
                f.strip_suffix('"').unwrap_or(f).to_string()
            })
            .collect();
        Line { fields, pos: 0 }
    }
    /// Siguiente campo (vacío si no quedan).
    pub fn next(&mut self) -> String {
        let f = self.fields.get(self.pos).cloned().unwrap_or_default();
        self.pos += 1;
        f
    }
    pub fn get(&self, i: usize) -> &str {
        self.fields.get(i).map(|s| s.as_str()).unwrap_or("")
    }
    pub fn len(&self) -> usize {
        self.fields.len()
    }
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
    pub fn all(&self) -> &[String] {
        &self.fields
    }
}

/// Lector de un archivo de WinQSB línea a línea.
pub struct Reader {
    lines: Vec<String>,
    pos: usize,
}

impl Reader {
    pub fn new(bytes: &[u8]) -> Reader {
        let text = decode(bytes);
        let mut lines: Vec<String> = text.split('\n').map(|l| l.trim_end_matches('\r').to_string()).collect();
        while lines.last().map(|l| l.is_empty()).unwrap_or(false) {
            lines.pop();
        }
        Reader { lines, pos: 0 }
    }
    /// Siguiente línea (las líneas totalmente vacías se saltan: WinQSB a veces las deja entre secciones).
    pub fn line(&mut self, what: &str) -> Result<Line, ModError> {
        while self.lines.get(self.pos).map(|l| l.trim().is_empty()).unwrap_or(false) {
            self.pos += 1;
        }
        let l = self.lines.get(self.pos).ok_or_else(|| ModError::Format(format!("archivo incompleto: falta {what}")))?;
        self.pos += 1;
        Ok(Line::parse(l))
    }
    /// Línea opcional (None al final del archivo).
    pub fn try_line(&mut self) -> Option<Line> {
        while self.lines.get(self.pos).map(|l| l.trim().is_empty()).unwrap_or(false) {
            self.pos += 1;
        }
        let l = self.lines.get(self.pos)?;
        self.pos += 1;
        Some(Line::parse(l))
    }
    pub fn raw(&mut self) -> Option<String> {
        let l = self.lines.get(self.pos)?.clone();
        self.pos += 1;
        Some(l)
    }
    pub fn remaining(&self) -> usize {
        self.lines.len().saturating_sub(self.pos)
    }
}

/// Escritor: campos con tabuladores y líneas con CRLF.
#[derive(Default)]
pub struct Writer {
    out: String,
}

impl Writer {
    pub fn new() -> Writer {
        Writer::default()
    }
    pub fn row<S: AsRef<str>>(&mut self, cells: &[S]) -> &mut Self {
        let parts: Vec<&str> = cells.iter().map(|c| c.as_ref()).collect();
        self.out += &parts.join("\t");
        self.out += "\r\n";
        self
    }
    pub fn bytes(&self) -> Vec<u8> {
        encode(&self.out)
    }
}

/// Número con el formato de los archivos (sin ceros sobrantes; M para infinito).
pub fn num(x: f64) -> String {
    if x == f64::INFINITY {
        return "M".into();
    }
    if x == f64::NEG_INFINITY {
        return "-M".into();
    }
    if x == x.trunc() && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else {
        let s = format!("{:.10}", x);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// Número opcional: vacío si es None.
pub fn opt(x: Option<f64>) -> String {
    x.map(num).unwrap_or_default()
}

/// Lee un número como WinQSB: admite separador de miles, "$", "M" y "-M".
pub fn parse_num(s: &str) -> Option<f64> {
    let t: String = s.chars().filter(|c| *c != ',' && *c != '$' && !c.is_whitespace()).collect();
    match t.to_ascii_uppercase().as_str() {
        "" => None,
        "M" | "+M" | "INF" | "INFINITY" => Some(f64::INFINITY),
        "-M" | "-INF" | "-INFINITY" => Some(f64::NEG_INFINITY),
        _ => t.parse().ok(),
    }
}

/// Número obligatorio (vacío = 0), con mensaje de error claro.
pub fn req(s: &str, at: &str) -> Result<f64, ModError> {
    if s.trim().is_empty() {
        return Ok(0.0);
    }
    parse_num(s).ok_or_else(|| ModError::Format(format!("{at}: «{s}» no es un número")))
}

/// Número opcional: None si está vacío.
pub fn optional(s: &str, at: &str) -> Result<Option<f64>, ModError> {
    if s.trim().is_empty() {
        return Ok(None);
    }
    parse_num(s).map(Some).ok_or_else(|| ModError::Format(format!("{at}: «{s}» no es un número")))
}

/// Comprueba la etiqueta de la cabecera.
pub fn check_tag(first: &Line, tag: &str, module: &str) -> Result<(), ModError> {
    if !first.get(0).trim().to_ascii_uppercase().starts_with(tag) {
        return Err(ModError::Format(format!("no es un archivo de {module}")));
    }
    Ok(())
}
