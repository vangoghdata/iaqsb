//! Descripción de las entradas de cada módulo. La aplicación construye con esto
//! el editor de datos (parámetros, hojas de registros y matrices), igual que las
//! hojas de cálculo de WinQSB, y una IA lo usa para saber qué datos pedir.

use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FieldKind {
    Number,
    Integer,
    Text,
    Bool,
    /// Lista cerrada de opciones: (valor, etiqueta).
    Choice { options: Vec<(String, String)> },
    /// Número o "M" / "-M" (infinito), como en WinQSB.
    Bound,
    /// Lista de nombres separados por comas (p. ej. predecesores).
    List,
    /// Expresión matemática (p. ej. "x1^2 + 3*x1*x2").
    Expr,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Field {
    pub key: String,
    pub label: String,
    pub kind: FieldKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(skip_serializing_if = "Value::is_null")]
    pub default: Value,
    /// Solo se muestra si se cumplen todas estas condiciones: el campo indicado
    /// (de la fila, del modelo o de las opciones) tiene alguno de los valores.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub show_if: Vec<(String, Vec<String>)>,
    /// Puede quedar vacío (null).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    /// Opción de método: se puede comparar resolviendo con cada uno de sus valores.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub comparable: bool,
}

impl Field {
    fn base(key: &str, label: &str, kind: FieldKind, default: Value) -> Field {
        Field { key: key.into(), label: label.into(), kind, help: None, default, show_if: vec![], optional: false, comparable: false }
    }
    pub fn number(key: &str, label: &str) -> Field {
        Self::base(key, label, FieldKind::Number, Value::from(0.0))
    }
    pub fn integer(key: &str, label: &str, default: i64) -> Field {
        Self::base(key, label, FieldKind::Integer, Value::from(default))
    }
    pub fn text(key: &str, label: &str) -> Field {
        Self::base(key, label, FieldKind::Text, Value::from(""))
    }
    pub fn boolean(key: &str, label: &str, default: bool) -> Field {
        Self::base(key, label, FieldKind::Bool, Value::from(default))
    }
    pub fn choice(key: &str, label: &str, options: &[(&str, &str)]) -> Field {
        let opts: Vec<(String, String)> = options.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        let def = Value::from(opts.first().map(|o| o.0.clone()).unwrap_or_default());
        Self::base(key, label, FieldKind::Choice { options: opts }, def)
    }
    pub fn bound(key: &str, label: &str, default: Value) -> Field {
        Self::base(key, label, FieldKind::Bound, default)
    }
    pub fn list(key: &str, label: &str) -> Field {
        Self::base(key, label, FieldKind::List, Value::Array(vec![]))
    }
    pub fn expr(key: &str, label: &str) -> Field {
        Self::base(key, label, FieldKind::Expr, Value::from(""))
    }
    pub fn default(mut self, v: impl Into<Value>) -> Self {
        self.default = v.into();
        self
    }
    pub fn help(mut self, h: &str) -> Self {
        self.help = Some(h.into());
        self
    }
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self.default = Value::Null;
        self
    }
    /// Marca una opción de método para la comparación de métodos.
    pub fn comparable(mut self) -> Self {
        self.comparable = true;
        self
    }
    pub fn show_if(mut self, key: &str, values: &[&str]) -> Self {
        self.show_if.push((key.into(), values.iter().map(|s| s.to_string()).collect()));
        self
    }
}

/// Eje de una matriz: los nombres de filas o columnas salen de una lista del modelo.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Axis {
    /// Ruta de la lista en el modelo (p. ej. "sources").
    pub path: String,
    /// Clave del nombre dentro de cada elemento.
    pub label_key: String,
    /// Título del eje (p. ej. "Origen").
    pub label: String,
    /// Campos adicionales de cada elemento: columnas extra (filas) o filas extra (columnas).
    pub extra: Vec<Field>,
    /// Texto del botón para añadir un elemento; None = tamaño fijo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_label: Option<String>,
}

impl Axis {
    pub fn new(path: &str, label: &str) -> Axis {
        Axis { path: path.into(), label_key: "name".into(), label: label.into(), extra: vec![], add_label: None }
    }
    pub fn extra(mut self, f: Field) -> Self {
        self.extra.push(f);
        self
    }
    pub fn add(mut self, label: &str) -> Self {
        self.add_label = Some(label.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Widget {
    /// Campos sueltos en la raíz del modelo (o en `path`, si se indica).
    Params {
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        fields: Vec<Field>,
    },
    /// Lista de registros: una fila por elemento, una columna por campo.
    Records {
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        path: String,
        columns: Vec<Field>,
        #[serde(skip_serializing_if = "Option::is_none")]
        add_label: Option<String>,
    },
    /// Matriz `path[i][j]` con filas y columnas tomadas de dos listas (o de la misma).
    Matrix {
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        path: String,
        rows: Axis,
        cols: Axis,
        cell: Field,
        /// En matrices cuadradas, permitir la diagonal (por defecto se bloquea: no hay arco i→i).
        #[serde(skip_serializing_if = "std::ops::Not::not", default)]
        allow_diagonal: bool,
    },
}

impl Widget {
    pub fn params(title: Option<&str>, fields: Vec<Field>) -> Widget {
        Widget::Params { title: title.map(str::to_string), path: None, fields }
    }
    pub fn records(title: Option<&str>, path: &str, columns: Vec<Field>, add_label: Option<&str>) -> Widget {
        Widget::Records { title: title.map(str::to_string), path: path.into(), columns, add_label: add_label.map(str::to_string) }
    }
    pub fn matrix(title: Option<&str>, path: &str, rows: Axis, cols: Axis, cell: Field) -> Widget {
        Widget::Matrix { title: title.map(str::to_string), path: path.into(), rows, cols, cell, allow_diagonal: false }
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProblemType {
    pub id: String,
    pub label: String,
    pub description: String,
    /// Datos que se piden al crear un problema nuevo (dimensiones, criterio...).
    pub dims: Vec<Field>,
    pub layout: Vec<Widget>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ModuleSpec {
    pub id: String,
    /// Nombre del módulo en WinQSB.
    pub winqsb: String,
    pub name: String,
    pub description: String,
    /// Extensión de sus archivos en WinQSB.
    pub file_ext: String,
    pub problem_types: Vec<ProblemType>,
    /// Opciones de resolución comunes a todos los tipos.
    pub options: Vec<Field>,
}
