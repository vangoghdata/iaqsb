//! Informes de resultados. Cada módulo devuelve un `Report` formado por bloques
//! genéricos (tablas, gráficos, redes, diagramas de Gantt, árboles y pasos). La
//! aplicación los dibuja con los mismos componentes para los 19 módulos, y una IA
//! puede leer `data`, que trae los resultados en forma estructurada.

use schemars::JsonSchema;
use serde::{Serialize, Serializer};
use serde_json::Value;

/// Valor de una celda: número, texto o vacío.
#[derive(Debug, Clone, PartialEq, JsonSchema)]
pub enum Cell {
    Num(f64),
    Text(String),
    Empty,
}

impl Serialize for Cell {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Cell::Num(x) if x.is_finite() => s.serialize_f64(clean(*x)),
            Cell::Num(x) if x.is_nan() => s.serialize_none(),
            Cell::Num(x) => s.serialize_str(if *x > 0.0 { "∞" } else { "−∞" }),
            Cell::Text(t) => s.serialize_str(t),
            Cell::Empty => s.serialize_none(),
        }
    }
}

impl From<f64> for Cell {
    fn from(x: f64) -> Self {
        Cell::Num(x)
    }
}
impl From<i64> for Cell {
    fn from(x: i64) -> Self {
        Cell::Num(x as f64)
    }
}
impl From<usize> for Cell {
    fn from(x: usize) -> Self {
        Cell::Num(x as f64)
    }
}
impl From<i32> for Cell {
    fn from(x: i32) -> Self {
        Cell::Num(x as f64)
    }
}
impl From<&str> for Cell {
    fn from(x: &str) -> Self {
        Cell::Text(x.to_string())
    }
}
impl From<String> for Cell {
    fn from(x: String) -> Self {
        Cell::Text(x)
    }
}
impl From<&String> for Cell {
    fn from(x: &String) -> Self {
        Cell::Text(x.clone())
    }
}
impl<T: Into<Cell>> From<Option<T>> for Cell {
    fn from(x: Option<T>) -> Self {
        x.map(Into::into).unwrap_or(Cell::Empty)
    }
}

/// Construye una fila de celdas: `cells![a, b, c]`.
#[macro_export]
macro_rules! cells {
    ($($x:expr),* $(,)?) => { vec![$($crate::report::Cell::from($x)),*] };
}

/// Redondea ruido numérico para mostrar (1e-12 → 0, 2.0000000001 → 2).
pub fn clean(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    if x.abs() < 1e-10 {
        return 0.0;
    }
    let r = (x * 1e9).round() / 1e9;
    if (r - x).abs() <= 1e-9 * (1.0 + x.abs()) {
        r
    } else {
        x
    }
}

/// Quita el ruido de los optimizadores (tolerancia ~1e-7): lo que está a esa
/// distancia de un número sencillo (hasta 3 decimales) se redondea
/// (6.00000002 → 6, 1e-7 → 0). Los valores como 8/3 no se tocan.
pub fn tidy(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let r = (x * 1e3).round() / 1e3;
    if (r - x).abs() <= 2e-7 + 1e-9 * x.abs() {
        clean(r)
    } else {
        x
    }
}

/// Tono visual de una fila, celda o marca. La interfaz decide los colores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    #[default]
    Normal,
    /// Lo principal (solución, variable que entra).
    Accent,
    /// Lo que limita o lo crítico (ruta crítica, restricción activa, pivote).
    Highlight,
    Good,
    Bad,
    Muted,
    /// Fila de totales.
    Total,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Metric {
    pub label: String,
    pub value: Cell,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub emphasis: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}
fn is_normal(t: &Tone) -> bool {
    *t == Tone::Normal
}

pub fn metric(label: impl Into<String>, value: impl Into<Cell>) -> Metric {
    Metric { label: label.into(), value: value.into(), unit: None, emphasis: false }
}
pub fn metric_main(label: impl Into<String>, value: impl Into<Cell>) -> Metric {
    Metric { label: label.into(), value: value.into(), unit: None, emphasis: true }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Column {
    pub label: String,
    /// "left" para texto; por defecto los números van a la derecha.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Row {
    pub cells: Vec<Cell>,
    #[serde(skip_serializing_if = "is_normal")]
    pub tone: Tone,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Mark {
    pub row: usize,
    pub col: usize,
    pub tone: Tone,
}

#[derive(Debug, Clone, Serialize, JsonSchema, Default)]
pub struct Table {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
    /// Celdas destacadas (pivote, celda que entra).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<Mark>,
    /// Columnas destacadas: (índice, tono).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub col_marks: Vec<(usize, Tone)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Table {
    /// Tabla con columnas; la primera se alinea a la izquierda.
    pub fn new<S: AsRef<str>>(title: Option<&str>, cols: &[S]) -> Table {
        Table {
            title: title.map(str::to_string),
            columns: cols
                .iter()
                .enumerate()
                .map(|(i, c)| Column { label: c.as_ref().to_string(), align: if i == 0 { Some("left".into()) } else { None } })
                .collect(),
            ..Default::default()
        }
    }
    pub fn left(mut self, col: usize) -> Self {
        if let Some(c) = self.columns.get_mut(col) {
            c.align = Some("left".into());
        }
        self
    }
    pub fn row(&mut self, cells: Vec<Cell>) -> &mut Self {
        self.rows.push(Row { cells, tone: Tone::Normal });
        self
    }
    pub fn row_tone(&mut self, cells: Vec<Cell>, tone: Tone) -> &mut Self {
        self.rows.push(Row { cells, tone });
        self
    }
    pub fn mark(&mut self, row: usize, col: usize, tone: Tone) -> &mut Self {
        self.marks.push(Mark { row, col, tone });
        self
    }
    pub fn note(mut self, n: impl Into<String>) -> Self {
        self.note = Some(n.into());
        self
    }
    pub fn block(self) -> Block {
        Block::Table(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SeriesKind {
    Line,
    Bar,
    Scatter,
    Step,
    Area,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Series {
    pub name: String,
    pub kind: SeriesKind,
    #[serde(skip_serializing_if = "is_normal")]
    pub tone: Tone,
    #[serde(skip_serializing_if = "is_false")]
    pub dashed: bool,
    /// Pares (x, y). Con categorías, x es el índice de la categoría.
    pub points: Vec<[f64; 2]>,
}

impl Series {
    pub fn new(name: impl Into<String>, kind: SeriesKind, points: Vec<[f64; 2]>) -> Series {
        Series { name: name.into(), kind, tone: Tone::Normal, dashed: false, points }
    }
    pub fn tone(mut self, t: Tone) -> Self {
        self.tone = t;
        self
    }
    pub fn dashed(mut self) -> Self {
        self.dashed = true;
        self
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct RefLine {
    pub value: f64,
    pub label: String,
    #[serde(skip_serializing_if = "is_normal")]
    pub tone: Tone,
}

#[derive(Debug, Clone, Serialize, JsonSchema, Default)]
pub struct Chart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub x_label: String,
    pub y_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,
    pub series: Vec<Series>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hlines: Vec<RefLine>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub vlines: Vec<RefLine>,
    /// Empieza el eje Y en cero.
    #[serde(skip_serializing_if = "is_false")]
    pub y_zero: bool,
    /// Las series de barras se apilan en lugar de ir una al lado de otra.
    #[serde(skip_serializing_if = "is_false")]
    pub stacked: bool,
}

impl Chart {
    pub fn stacked(mut self) -> Self {
        self.stacked = true;
        self.y_zero = true;
        self
    }
    pub fn new(title: Option<&str>, x: &str, y: &str) -> Chart {
        Chart { title: title.map(str::to_string), x_label: x.into(), y_label: y.into(), ..Default::default() }
    }
    pub fn series(mut self, s: Series) -> Self {
        self.series.push(s);
        self
    }
    pub fn hline(mut self, value: f64, label: impl Into<String>, tone: Tone) -> Self {
        self.hlines.push(RefLine { value, label: label.into(), tone });
        self
    }
    pub fn vline(mut self, value: f64, label: impl Into<String>, tone: Tone) -> Self {
        self.vlines.push(RefLine { value, label: label.into(), tone });
        self
    }
    pub fn block(self) -> Block {
        Block::Chart(self)
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct NetNode {
    pub id: String,
    pub label: String,
    /// Posición en [0, 1] × [0, 1]; la calcula el módulo.
    pub x: f64,
    pub y: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "is_normal")]
    pub tone: Tone,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct NetEdge {
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "is_normal")]
    pub tone: Tone,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Network {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub directed: bool,
    pub nodes: Vec<NetNode>,
    pub edges: Vec<NetEdge>,
}

/// Un rectángulo del plano (coordenadas en la unidad del plano).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct PlanRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "is_normal")]
    pub tone: Tone,
    /// Rectángulo de fondo (pasillo, bloque) sin borde marcado.
    #[serde(skip_serializing_if = "is_false")]
    pub soft: bool,
}

/// Una cota del plano: línea de (x1, y1) a (x2, y2) con su medida.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct PlanDim {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct FloorPlan {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub width: f64,
    pub height: f64,
    pub unit: String,
    pub rects: Vec<PlanRect>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dims: Vec<PlanDim>,
}

impl FloorPlan {
    pub fn new(title: &str, width: f64, height: f64, unit: &str) -> FloorPlan {
        FloorPlan { title: Some(title.into()), width, height, unit: unit.into(), rects: vec![], dims: vec![] }
    }
    pub fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, label: Option<String>, tone: Tone, soft: bool) {
        self.rects.push(PlanRect { x, y, w, h, label, tone, soft });
    }
    pub fn dim(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, label: impl Into<String>) {
        self.dims.push(PlanDim { x1, y1, x2, y2, label: label.into() });
    }
    pub fn block(self) -> Block {
        Block::Layout(self)
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct GanttBar {
    pub start: f64,
    pub end: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "is_normal")]
    pub tone: Tone,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct GanttRow {
    pub label: String,
    pub bars: Vec<GanttBar>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Gantt {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub unit: String,
    pub rows: Vec<GanttRow>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<RefLine>,
    /// Leyenda de los tonos de las barras.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub legend: Vec<(Tone, String)>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct TreeNode {
    pub id: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<usize>,
    /// Texto de la rama que llega al nodo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge: Option<String>,
    pub label: String,
    /// "decision", "chance", "end" u otro tipo propio del módulo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "is_normal")]
    pub tone: Tone,
}

/// Punto del plano (depósito, cliente, instalación).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct MapPoint {
    pub id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    /// "depot" (depósito) o "stop" (cliente).
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Ruta a la que pertenece (índice en `routes`), para colorearlo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<usize>,
}

/// Recorrido por una secuencia de puntos.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct MapRoute {
    pub name: String,
    pub stops: Vec<String>,
    /// Resumen para la leyenda (carga, distancia).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub dashed: bool,
}

/// Rayo que sale de un punto con un ángulo (grados, antihorario desde el eje X), como en el método del barrido.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct MapRay {
    pub from: String,
    pub angle: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema, Default)]
pub struct RouteMap {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub points: Vec<MapPoint>,
    pub routes: Vec<MapRoute>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rays: Vec<MapRay>,
    /// Flechas en el sentido del recorrido.
    #[serde(skip_serializing_if = "is_false")]
    pub arrows: bool,
    /// Coordenadas geográficas: x = longitud, y = latitud (en grados). El dibujo
    /// usa una proyección equirectangular (la longitud se escala por cos(latitud)).
    #[serde(skip_serializing_if = "is_false")]
    pub geo: bool,
}

impl RouteMap {
    pub fn block(self) -> Block {
        Block::Map(self)
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Tree {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub nodes: Vec<TreeNode>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Step {
    pub title: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub blocks: Vec<Block>,
}

pub fn step(title: impl Into<String>, description: impl Into<String>, blocks: Vec<Block>) -> Step {
    Step { title: title.into(), description: description.into(), blocks }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    Metrics {
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        items: Vec<Metric>,
    },
    Table(Table),
    Text {
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        body: String,
        #[serde(skip_serializing_if = "is_normal")]
        tone: Tone,
    },
    Chart(Chart),
    /// Fuentes del informe: origen de los datos y referencias (APA 7).
    Sources {
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<crate::biblio::DataSource>,
        refs: Vec<crate::biblio::Citation>,
    },
    Network(Network),
    /// Plano a escala: rectángulos (estibas, bloques, pasillos) con cotas.
    Layout(FloorPlan),
    Gantt(Gantt),
    Tree(Tree),
    /// Plano con puntos en sus coordenadas y recorridos (rutas de vehículos).
    Map(RouteMap),
    Steps {
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        steps: Vec<Step>,
    },
}

pub fn text(body: impl Into<String>) -> Block {
    Block::Text { title: None, body: body.into(), tone: Tone::Normal }
}
pub fn text_tone(body: impl Into<String>, tone: Tone) -> Block {
    Block::Text { title: None, body: body.into(), tone }
}
pub fn metrics(items: Vec<Metric>) -> Block {
    Block::Metrics { title: None, items }
}
pub fn steps(steps: Vec<Step>) -> Block {
    Block::Steps { title: None, steps }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Óptimo encontrado.
    Optimal,
    /// Solución factible (heurística o con límite alcanzado).
    Feasible,
    Infeasible,
    Unbounded,
    /// Análisis completado (módulos sin optimización: colas, pronósticos...).
    Solved,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Section {
    pub id: String,
    pub title: String,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Report {
    pub module: String,
    pub problem: String,
    pub status: Status,
    /// Una frase con el resultado principal.
    pub headline: String,
    pub sections: Vec<Section>,
    /// Resultados estructurados para scripts e IA.
    pub data: Value,
    /// Fuentes de los métodos, fórmulas y pasos usados (APA 7).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<crate::biblio::Citation>,
    /// De dónde salen los datos, si el modelo es uno de los ejemplos.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_source: Option<crate::biblio::DataSource>,
}

impl Report {
    pub fn new(module: &str, problem: impl Into<String>, status: Status, headline: impl Into<String>) -> Report {
        Report {
            module: module.into(),
            problem: problem.into(),
            status,
            headline: headline.into(),
            sections: vec![],
            data: Value::Null,
            references: vec![],
            data_source: None,
        }
    }
    /// Cita una fuente de la bibliografía (`fuentes/bibliografia.json`) y dice
    /// para qué se usa en este informe. Citar dos veces la misma suma los usos.
    pub fn cite(mut self, id: &str, used_for: &str) -> Self {
        self.add_cite(id, used_for);
        self
    }
    /// Cita solo si se cumple la condición.
    pub fn cite_if(self, cond: bool, id: &str, used_for: &str) -> Self {
        if cond {
            self.cite(id, used_for)
        } else {
            self
        }
    }
    pub fn add_cite(&mut self, id: &str, used_for: &str) {
        if let Some(c) = self.references.iter_mut().find(|c| c.id == id) {
            if !c.used_for.iter().any(|u| u == used_for) {
                c.used_for.push(used_for.into());
            }
        } else {
            self.references.push(crate::biblio::Citation::new(id, used_for));
        }
    }
    /// Agrega (o rehace) la sección «Fuentes» con el origen de los datos y las referencias.
    pub fn finish_sources(mut self) -> Self {
        self.sections.retain(|s| s.id != "fuentes");
        if !self.references.is_empty() || self.data_source.is_some() {
            let b = Block::Sources { data: self.data_source.clone(), refs: self.references.clone() };
            self.sections.push(Section { id: "fuentes".into(), title: "Fuentes".into(), blocks: vec![b] });
        }
        self
    }
    pub fn section(mut self, id: &str, title: &str, blocks: Vec<Block>) -> Self {
        if !blocks.is_empty() {
            self.sections.push(Section { id: id.into(), title: title.into(), blocks });
        }
        self
    }
    pub fn data(mut self, v: Value) -> Self {
        self.data = v;
        self
    }
}

/// Formatea un número para textos (hasta 4 decimales, sin ceros sobrantes).
pub fn fmt(x: f64) -> String {
    if x.is_nan() {
        return "–".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "∞".into() } else { "−∞".into() };
    }
    let x = clean(x);
    if x == x.trunc() && x.abs() < 1e15 {
        return format!("{}", x as i64);
    }
    let s = format!("{:.4}", x);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".into()
    } else {
        s.to_string()
    }
}
