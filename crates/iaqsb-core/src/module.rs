//! Contrato común de los módulos de IAQSB.

use crate::report::Report;
use crate::spec::ModuleSpec;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, thiserror::Error)]
pub enum ModError {
    /// El modelo no es válido (dato que falta, incoherente...).
    #[error("{0}")]
    Invalid(String),
    /// El archivo no se pudo leer.
    #[error("archivo no válido: {0}")]
    Format(String),
}

pub fn invalid<T>(msg: impl Into<String>) -> Result<T, ModError> {
    Err(ModError::Invalid(msg.into()))
}

/// Interfaz dinámica (JSON) que usan la API, la CLI, el MCP y la aplicación.
pub trait Module: Sync + Send {
    fn spec(&self) -> ModuleSpec;
    fn model_schema(&self) -> Value;
    fn options_schema(&self) -> Value;
    /// Problema en blanco del tipo indicado, con las dimensiones pedidas.
    fn template(&self, problem_type: &str, dims: &Value) -> Result<Value, ModError>;
    fn solve(&self, model: &Value, options: &Value) -> Result<Report, ModError>;
    /// Lee un archivo de WinQSB y devuelve el modelo.
    fn read_file(&self, bytes: &[u8]) -> Result<Value, ModError>;
    /// Escribe el modelo en el formato de WinQSB.
    fn write_file(&self, model: &Value) -> Result<Vec<u8>, ModError>;
    fn examples(&self) -> Vec<(String, Value)>;
    /// ¿Es este archivo de este módulo? (por la cabecera)
    fn recognizes(&self, head: &str) -> bool;
}

/// Forma cómoda de escribir un módulo con tipos de Rust. Se convierte
/// automáticamente en `Module`.
pub trait TypedModule: Sync + Send {
    type Model: DeserializeOwned + Serialize + JsonSchema;
    type Options: DeserializeOwned + Default + JsonSchema;
    fn spec(&self) -> ModuleSpec;
    fn template(&self, problem_type: &str, dims: &Value) -> Result<Self::Model, ModError>;
    fn solve(&self, model: &Self::Model, options: &Self::Options) -> Result<Report, ModError>;
    fn read(&self, bytes: &[u8]) -> Result<Self::Model, ModError>;
    fn write(&self, model: &Self::Model) -> Result<Vec<u8>, ModError>;
    fn examples(&self) -> Vec<(&'static str, Self::Model)>;
    /// Prefijo de la primera línea de sus archivos (p. ej. "NET").
    fn file_tag(&self) -> &'static str;
    /// Todos los prefijos posibles de la primera línea (por defecto, solo `file_tag`).
    fn file_tags(&self) -> Vec<&'static str> {
        vec![self.file_tag()]
    }
}

fn parse<T: DeserializeOwned>(v: &Value, what: &str) -> Result<T, ModError> {
    serde_json::from_value(v.clone()).map_err(|e| ModError::Invalid(format!("{what}: {e}")))
}

impl<T: TypedModule> Module for T {
    fn spec(&self) -> ModuleSpec {
        TypedModule::spec(self)
    }
    fn model_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(T::Model)).unwrap()
    }
    fn options_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(T::Options)).unwrap()
    }
    fn template(&self, problem_type: &str, dims: &Value) -> Result<Value, ModError> {
        Ok(serde_json::to_value(TypedModule::template(self, problem_type, dims)?).unwrap())
    }
    fn solve(&self, model: &Value, options: &Value) -> Result<Report, ModError> {
        let m: T::Model = parse(model, "modelo")?;
        // una opción en null (campo opcional vacío en la aplicación) toma su valor por defecto
        let options = match options {
            Value::Object(m) => Value::Object(m.iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| (k.clone(), v.clone())).collect()),
            o => o.clone(),
        };
        let o: T::Options = if options.is_null() { T::Options::default() } else { parse(&options, "opciones")? };
        TypedModule::solve(self, &m, &o)
    }
    fn read_file(&self, bytes: &[u8]) -> Result<Value, ModError> {
        Ok(serde_json::to_value(self.read(bytes)?).unwrap())
    }
    fn write_file(&self, model: &Value) -> Result<Vec<u8>, ModError> {
        let m: T::Model = parse(model, "modelo")?;
        self.write(&m)
    }
    fn examples(&self) -> Vec<(String, Value)> {
        TypedModule::examples(self).into_iter().map(|(n, m)| (n.to_string(), serde_json::to_value(m).unwrap())).collect()
    }
    fn recognizes(&self, head: &str) -> bool {
        let h = head.trim_start().trim_start_matches('"').to_ascii_uppercase();
        self.file_tags().iter().any(|t| h.starts_with(t))
    }
}

/// Lee un número de dimensión (entero ≥ min) de los datos de un problema nuevo.
pub fn dim(dims: &Value, key: &str, default: usize, min: usize, max: usize) -> Result<usize, ModError> {
    let v = match dims.get(key) {
        None | Some(Value::Null) => default,
        Some(x) => x
            .as_f64()
            .or_else(|| x.as_str().and_then(|s| s.trim().parse().ok()))
            .ok_or_else(|| ModError::Invalid(format!("{key} debe ser un número")))? as usize,
    };
    if v < min || v > max {
        return invalid(format!("{key} debe estar entre {min} y {max}"));
    }
    Ok(v)
}

pub fn dim_str(dims: &Value, key: &str, default: &str) -> String {
    dims.get(key).and_then(|v| v.as_str()).unwrap_or(default).to_string()
}

/// Comprueba que los nombres de una lista no estén vacíos ni repetidos.
pub fn check_names<S: AsRef<str>>(what: &str, names: &[S]) -> Result<(), ModError> {
    let mut seen = std::collections::HashSet::new();
    for n in names {
        let n = n.as_ref().trim();
        if n.is_empty() {
            return invalid(format!("hay un elemento de «{what}» sin nombre"));
        }
        if !seen.insert(n.to_uppercase()) {
            return invalid(format!("el nombre «{n}» está repetido en «{what}»"));
        }
    }
    Ok(())
}

/// Criterio del objetivo, común a muchos módulos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum Objective {
    #[default]
    Min,
    Max,
}

impl Objective {
    pub fn is_max(self) -> bool {
        self == Objective::Max
    }
    pub fn word(self) -> &'static str {
        if self.is_max() {
            "Maximization"
        } else {
            "Minimization"
        }
    }
    /// Lee "MAX..." / "MIN..." de un archivo.
    pub fn parse(s: &str) -> Objective {
        if s.trim().to_ascii_uppercase().starts_with("MAX") {
            Objective::Max
        } else {
            Objective::Min
        }
    }
}

/// Celda opcional de una matriz (vacía = sin conexión).
pub type Cellv = Option<f64>;
