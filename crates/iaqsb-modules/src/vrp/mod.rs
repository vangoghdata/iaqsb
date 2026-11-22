//! VRP · Ruteo de vehículos (no existe en WinQSB; es el tema de ruteo urbano).
//!
//! Un depósito, clientes con demanda y vehículos con capacidad. Se resuelve con
//! los métodos que se ven en clase y se comparan entre sí:
//!
//! - Barrido (Gillett y Miller): se ordenan los clientes por el ángulo que
//!   forman con el depósito y se llena cada vehículo en ese orden; después
//!   se busca el mejor recorrido de cada zona.
//! - Ahorros de Clarke y Wright: se empieza con una ruta por cliente y se unen
//!   rutas por los extremos con el mayor ahorro s(i,j) = d(i,0) + d(0,j) − d(i,j).
//! - Vecino más cercano con capacidad.
//! - Óptimo con programación entera (HiGHS, formulación de Miller-Tucker-Zemlin
//!   con cargas), para pocos clientes.
//!
//! Cada ruta se mejora con intercambios 2-opt y las zonas del barrido se
//! recorren de forma óptima (programación dinámica de Held y Karp) cuando
//! tienen hasta 12 clientes. Las distancias salen de las coordenadas
//! (euclidiana, rectilínea o geográfica: latitud y longitud con la distancia
//! del gran círculo, fórmula del haversine, en km) o de una tabla de distancias
//! de la red de calles; en la tabla, los pares sin calle directa se completan
//! con la ruta más corta (Floyd-Warshall).

mod algo;

use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum VrpType {
    /// Clientes ubicados por coordenadas.
    #[default]
    Coordinates,
    /// Red de calles: tabla de distancias entre puntos.
    Network,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    #[default]
    Euclidean,
    Rectilinear,
    /// Coordenadas geográficas: `y` = latitud y `x` = longitud, en grados;
    /// distancia del gran círculo (haversine) en kilómetros.
    Haversine,
}

/// Radio medio de la Tierra en km (IUGG).
pub const EARTH_RADIUS_KM: f64 = 6371.0088;

/// Distancia del gran círculo entre dos puntos (latitud, longitud en grados), en km.
pub fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = p2 - p1;
    let dl = (lon2 - lon1).to_radians();
    let h = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_KM * h.sqrt().min(1.0).asin()
}

/// Punto de la red: el primero es el depósito.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct Stop {
    pub name: String,
    /// Coordenada X (con la distancia geográfica, la longitud en grados).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    /// Coordenada Y (con la distancia geográfica, la latitud en grados).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
    #[serde(default)]
    pub demand: f64,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VrpModel {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub problem_type: VrpType,
    /// El primer punto es el depósito; los demás, los clientes.
    pub points: Vec<Stop>,
    /// Capacidad de cada vehículo (en las mismas unidades que la demanda).
    pub capacity: f64,
    /// Vehículos disponibles; vacío = los que hagan falta.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vehicles: Option<usize>,
    /// Distancia máxima de cada ruta; vacío = sin límite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_distance: Option<f64>,
    #[serde(default)]
    pub metric: Metric,
    /// Red de calles: distancia directa de i a j (vacío = no hay calle directa).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub distances: Vec<Vec<Option<f64>>>,
    /// Red de calles de doble sentido: d(i,j) = d(j,i).
    #[serde(default = "yes")]
    pub symmetric: bool,
    /// Costo por unidad de distancia (opcional, para expresar el resultado en dinero).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_per_distance: Option<f64>,
    /// Costo fijo por vehículo usado (opcional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_per_vehicle: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// Todos los métodos, comparados.
    #[default]
    Compare,
    Sweep,
    Savings,
    NearestNeighbor,
    Exact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Contrario a las manecillas del reloj.
    #[default]
    Counterclockwise,
    Clockwise,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct VrpOptions {
    pub steps: bool,
    pub method: Method,
    /// Barrido: ángulo donde empieza (grados, 0 = hacia la derecha del depósito).
    pub start_angle: f64,
    pub direction: Direction,
    /// Barrido: probar cada cliente como inicio y quedarse con el mejor.
    pub best_start: bool,
    /// Mejorar cada ruta con intercambios 2-opt.
    pub improve: bool,
    /// Segundos como máximo para el óptimo.
    pub time_limit: f64,
}

impl Default for VrpOptions {
    fn default() -> Self {
        VrpOptions { steps: true, method: Method::Compare, start_angle: 0.0, direction: Direction::Counterclockwise, best_start: false, improve: true, time_limit: 20.0 }
    }
}

impl VrpModel {
    pub fn validate(&self) -> Result<(), ModError> {
        check_names("puntos", &self.points.iter().map(|p| &p.name).collect::<Vec<_>>())?;
        if self.points.len() < 2 {
            return invalid("hace falta el depósito y al menos un cliente");
        }
        if !(self.capacity > 0.0) || !self.capacity.is_finite() {
            return invalid("la capacidad del vehículo debe ser un número mayor que cero");
        }
        for p in &self.points[1..] {
            if p.demand < 0.0 || !p.demand.is_finite() {
                return invalid(format!("la demanda de {} debe ser un número no negativo", p.name));
            }
            if p.demand > self.capacity + 1e-9 {
                return invalid(format!(
                    "la demanda de {} ({}) supera la capacidad del vehículo ({}): ningún vehículo la puede llevar",
                    p.name,
                    fmt(p.demand),
                    fmt(self.capacity)
                ));
            }
        }
        if let Some(l) = self.max_distance {
            if l < 0.0 {
                return invalid("la distancia máxima por ruta no puede ser negativa");
            }
        }
        match self.problem_type {
            VrpType::Coordinates => {
                for p in &self.points {
                    if p.x.is_none() || p.y.is_none() {
                        return invalid(format!("faltan las coordenadas de {}", p.name));
                    }
                    if self.geographic() {
                        let (lat, lon) = (p.y.unwrap(), p.x.unwrap());
                        if !(-90.0..=90.0).contains(&lat) {
                            return invalid(format!("la latitud de {} ({}) debe estar entre −90 y 90 grados", p.name, fmt(lat)));
                        }
                        if !(-180.0..=180.0).contains(&lon) {
                            return invalid(format!("la longitud de {} ({}) debe estar entre −180 y 180 grados", p.name, fmt(lon)));
                        }
                    }
                }
            }
            VrpType::Network => {
                let n = self.points.len();
                if self.distances.len() != n || self.distances.iter().any(|r| r.len() != n) {
                    return invalid("la tabla de distancias debe tener una fila y una columna por punto");
                }
                if self.distances.iter().flatten().flatten().any(|d| *d < 0.0 || !d.is_finite()) {
                    return invalid("las distancias deben ser números no negativos");
                }
            }
        }
        Ok(())
    }

    /// Vehículos disponibles (0 o vacío = los que hagan falta).
    pub fn fleet(&self) -> Option<usize> {
        self.vehicles.filter(|v| *v > 0)
    }

    pub fn has_coords(&self) -> bool {
        self.points.iter().all(|p| p.x.is_some() && p.y.is_some())
    }

    /// ¿Puntos en latitud y longitud (distancia del gran círculo, en km)?
    pub fn geographic(&self) -> bool {
        self.problem_type == VrpType::Coordinates && self.metric == Metric::Haversine
    }
}

pub struct Vrp;

impl TypedModule for Vrp {
    type Model = VrpModel;
    type Options = VrpOptions;

    fn file_tag(&self) -> &'static str {
        "IAQSB-VRP"
    }

    fn spec(&self) -> ModuleSpec {
        let params = |net: bool| {
            let mut f = vec![
                Field::number("capacity", "Capacidad de cada vehículo").default(1000.0),
                Field::integer("vehicles", "Vehículos disponibles", 0).optional().help("Vacío = los que hagan falta"),
                Field::number("max_distance", "Distancia máxima por ruta").default(0.0).optional().help("Vacío = sin límite"),
                Field::number("cost_per_distance", "Costo por unidad de distancia").default(0.0).optional(),
                Field::number("cost_per_vehicle", "Costo fijo por vehículo").default(0.0).optional(),
            ];
            if net {
                f.push(Field::boolean("symmetric", "Calles de doble sentido (la distancia vale igual en los dos sentidos)", true));
            } else {
                f.push(metric_field());
            }
            Widget::params(Some("Flota"), f)
        };
        let points = |net: bool| {
            let cols = if net {
                vec![
                    Field::text("name", "Punto"),
                    Field::number("x", "X (para el mapa y el barrido)").default(0.0).optional(),
                    Field::number("y", "Y (para el mapa y el barrido)").default(0.0).optional(),
                    Field::number("demand", "Demanda"),
                ]
            } else {
                // con la distancia geográfica, Y es la latitud y X la longitud (en ese orden, como en los mapas)
                let plane = &["euclidean", "rectilinear"];
                vec![
                    Field::text("name", "Punto"),
                    Field::number("x", "X").default(0.0).show_if("metric", plane),
                    Field::number("y", "Y").default(0.0).show_if("metric", plane),
                    Field::number("y", "Latitud").default(0.0).show_if("metric", &["haversine"]).help("Grados: positiva al norte del ecuador, negativa al sur"),
                    Field::number("x", "Longitud").default(0.0).show_if("metric", &["haversine"]).help("Grados: negativa al oeste de Greenwich (toda América)"),
                    Field::number("demand", "Demanda"),
                ]
            };
            Widget::records(Some("Depósito (primera fila) y clientes"), "points", cols, Some("Añadir cliente"))
        };
        ModuleSpec {
            id: "vrp".into(),
            winqsb: "Nuevo".into(),
            name: "Ruteo de vehículos".into(),
            description: "Rutas de reparto desde un depósito con vehículos de capacidad limitada: barrido, ahorros de Clarke y Wright, vecino más cercano y óptimo, con mapa de rutas".into(),
            file_ext: "VRP".into(),
            problem_types: vec![
                ProblemType {
                    id: "coordinates".into(),
                    label: "Clientes con coordenadas".into(),
                    description: "Cada punto tiene coordenadas X, Y o latitud y longitud; las distancias se calculan en línea recta, por cuadras o sobre la Tierra (km, con mapa real)".into(),
                    dims: vec![Field::integer("customers", "Número de clientes", 8), metric_field()],
                    layout: vec![params(false), points(false)],
                },
                ProblemType {
                    id: "network".into(),
                    label: "Red de calles (tabla de distancias)".into(),
                    description: "Distancias entre los puntos conectados; los demás pares se completan con la ruta más corta".into(),
                    dims: vec![Field::integer("customers", "Número de clientes", 6)],
                    layout: vec![
                        params(true),
                        points(true),
                        Widget::matrix(
                            Some("Distancias directas (vacío = no hay calle directa)"),
                            "distances",
                            Axis::new("points", "Desde"),
                            Axis::new("points", "Hasta"),
                            Field::number("d", "Distancia").optional(),
                        ),
                    ],
                },
            ],
            options: vec![
                Field::boolean("steps", "Mostrar paso a paso", true),
                Field::choice(
                    "method",
                    "Método",
                    &[
                        ("compare", "Comparar todos"),
                        ("sweep", "Barrido"),
                        ("savings", "Ahorros (Clarke y Wright)"),
                        ("nearest_neighbor", "Vecino más cercano"),
                        ("exact", "Óptimo (programación entera)"),
                    ],
                ).comparable(),
                Field::boolean("improve", "Mejorar las rutas con 2-opt", true),
                Field::number("start_angle", "Barrido: ángulo inicial (grados)").default(0.0).help("0 = hacia la derecha del depósito, 90 = hacia arriba"),
                Field::choice("direction", "Barrido: sentido", &[("counterclockwise", "Contrario al reloj"), ("clockwise", "Como el reloj")]),
                Field::boolean("best_start", "Barrido: probar todos los inicios", false),
                Field::number("time_limit", "Tiempo máximo del óptimo (segundos)").default(20.0),
            ],
        }
    }

    fn template(&self, problem_type: &str, dims: &Value) -> Result<VrpModel, ModError> {
        let t: VrpType = serde_json::from_value(Value::from(problem_type)).map_err(|_| ModError::Invalid(format!("tipo de problema desconocido: {problem_type}")))?;
        let n = dim(dims, "customers", 8, 1, 200)?;
        let metric: Metric = serde_json::from_value(Value::from(dim_str(dims, "metric", "euclidean"))).unwrap_or_default();
        let geo = t == VrpType::Coordinates && metric == Metric::Haversine;
        // con la distancia geográfica, el depósito queda en el centro de Bogotá y
        // los clientes en un círculo de unos 3 km (se mueven después en el mapa)
        let (lat0, lon0) = (4.6097, -74.0817);
        let round6 = |v: f64| (v * 1e6).round() / 1e6;
        let depot = if geo { Stop { name: "Depósito".into(), x: Some(lon0), y: Some(lat0), demand: 0.0 } } else { Stop { name: "Depósito".into(), x: Some(0.0), y: Some(0.0), demand: 0.0 } };
        let mut points = vec![depot];
        for k in 1..=n {
            let a = std::f64::consts::TAU * (k as f64) / (n as f64);
            let (x, y) = match t {
                VrpType::Coordinates if geo => (Some(round6(lon0 + 0.027 * a.cos() / lat0.to_radians().cos())), Some(round6(lat0 + 0.027 * a.sin()))),
                VrpType::Coordinates => (Some((10.0 * a.cos()).round()), Some((10.0 * a.sin()).round())),
                VrpType::Network => (None, None),
            };
            points.push(Stop { name: format!("Cliente{k}"), x, y, demand: 0.0 });
        }
        Ok(VrpModel {
            name: "Ruteo nuevo".into(),
            problem_type: t,
            distances: if t == VrpType::Network { vec![vec![None; n + 1]; n + 1] } else { vec![] },
            points,
            capacity: 1000.0,
            vehicles: None,
            max_distance: None,
            metric: if t == VrpType::Coordinates { metric } else { Metric::Euclidean },
            symmetric: true,
            cost_per_distance: None,
            cost_per_vehicle: None,
        })
    }

    fn solve(&self, m: &VrpModel, o: &VrpOptions) -> Result<Report, ModError> {
        m.validate()?;
        Ok(cite(algo::solve(m, o)?, m, o))
    }

    fn read(&self, bytes: &[u8]) -> Result<VrpModel, ModError> {
        let s = String::from_utf8_lossy(bytes);
        let s = s.trim_start_matches('\u{feff}');
        let (head, rest) = s.split_once('\n').unwrap_or((s, ""));
        if !head.trim().to_ascii_uppercase().starts_with("IAQSB-VRP") {
            return Err(ModError::Format("no es un archivo de ruteo de IAQSB".into()));
        }
        serde_json::from_str(rest).map_err(|e| ModError::Format(e.to_string()))
    }

    fn write(&self, m: &VrpModel) -> Result<Vec<u8>, ModError> {
        Ok(format!("IAQSB-VRP 1\n{}\n", serde_json::to_string_pretty(m).unwrap()).into_bytes())
    }

    fn examples(&self) -> Vec<(&'static str, VrpModel)> {
        let ex = |s: &str| serde_json::from_str::<VrpModel>(s).unwrap();
        vec![
            ("barrido_clase", ex(include_str!("ejemplos/barrido.json"))),
            ("reparto_ciudad", ex(include_str!("ejemplos/coordenadas.json"))),
            ("reparto_bogota", ex(include_str!("ejemplos/bogota.json"))),
        ]
    }
}

/// Opción de distancia entre coordenadas (en los datos y al crear un problema).
fn metric_field() -> Field {
    Field::choice(
        "metric",
        "Distancia",
        &[
            ("euclidean", "Euclidiana (en línea recta)"),
            ("rectilinear", "Rectilínea (por cuadras)"),
            ("haversine", "Geográfica: latitud y longitud (km)"),
        ],
    )
}

/// Fuentes de los métodos de ruteo usados en el informe.
fn cite(mut r: Report, m: &VrpModel, o: &VrpOptions) -> Report {
    let used: Vec<String> = r
        .data
        .get("comparison")
        .and_then(|c| c.as_array())
        .map(|a| a.iter().filter_map(|x| x.get("method").and_then(|v| v.as_str()).map(|s| s.to_lowercase())).collect())
        .unwrap_or_default();
    let ran = |k: &str, me: Method| o.method == me || used.iter().any(|u| u.contains(k));
    r.add_cite("dantzigramser1959", "Problema de ruteo de vehículos con capacidad (VRP)");
    r.add_cite("tothvigo2014", "Variantes, formulaciones y métodos del VRP");
    if ran("barrido", Method::Sweep) {
        r.add_cite("gillettmiller1974", "Método del barrido: clientes ordenados por ángulo polar desde el depósito");
        r.add_cite("wrenholliday1972", "Agrupación de clientes por ángulo (antecedente del barrido)");
        r.add_cite("sarache2026ruteo", "Método del barrido como se ve en clase (diapositiva 5)");
        r.add_cite("heldkarp1962", "Orden óptimo de cada zona del barrido (programación dinámica, hasta 12 clientes)");
    }
    if ran("ahorros", Method::Savings) {
        r.add_cite("clarkewright1964", "Método de los ahorros s(i,j) = d(0,i) + d(0,j) − d(i,j)");
    }
    if ran("vecino", Method::NearestNeighbor) {
        r.add_cite("rosenkrantz1977", "Vecino más cercano (aquí con la capacidad del vehículo)");
    }
    if ran("óptimo", Method::Exact) {
        r.add_cite("miller1960", "Óptimo: formulación de dos índices con variables de carga del tipo Miller-Tucker-Zemlin");
        r.add_cite("huangfuhall2018", "Solución del programa entero con HiGHS");
    }
    if o.improve {
        r.add_cite("croes1958", "Mejora de cada ruta con intercambios 2-opt");
    }
    if m.geographic() {
        r.add_cite("sinnott1984", "Distancia del gran círculo entre latitudes y longitudes (fórmula del haversine), en km");
    }
    if m.problem_type == VrpType::Network {
        r.add_cite("floyd1962", "Distancias por la red de calles: rutas más cortas entre todos los pares (Floyd-Warshall)");
        r.add_cite("warshall1962", "Distancias por la red de calles: rutas más cortas entre todos los pares (Floyd-Warshall)");
    }
    r
}
