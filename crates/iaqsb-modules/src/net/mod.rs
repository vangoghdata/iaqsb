//! NET · Modelos de redes (equivalente al módulo NET de WinQSB).
//!
//! Tipos de problema: flujo en redes con costes (transbordo), transporte,
//! asignación, ruta más corta, flujo máximo, árbol de expansión mínima y agente
//! viajero. El resultado lo calculan algoritmos exactos (HiGHS, Kuhn-Munkres,
//! Dijkstra/Bellman-Ford, Edmonds-Karp, Kruskal, cortes de subciclos) y el modo
//! didáctico muestra el método de clase paso a paso.

mod assign;
mod flow;
mod flowx;
mod fuentes;
mod format;
mod layout;
mod lpmodel;
mod paths;
mod transport;
mod tsp;

use iaqsb_core::linear::Bnd;
use iaqsb_core::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum NetType {
    /// Flujo en redes con costes (transbordo, red capacitada).
    NetworkFlow,
    #[default]
    Transportation,
    Assignment,
    ShortestPath,
    MaxFlow,
    MinSpanningTree,
    /// Agente viajero.
    Tsp,
}

impl NetType {
    pub fn bipartite(self) -> bool {
        matches!(self, NetType::Transportation | NetType::Assignment)
    }
    fn code(self) -> &'static str {
        match self {
            NetType::NetworkFlow => "CNF",
            NetType::Transportation => "TP",
            NetType::Assignment => "AP",
            NetType::ShortestPath => "SPP",
            NetType::MaxFlow => "MFP",
            NetType::MinSpanningTree => "MST",
            NetType::Tsp => "TSP",
        }
    }
    fn label(self) -> &'static str {
        match self {
            NetType::NetworkFlow => "Flujo en redes",
            NetType::Transportation => "Transporte",
            NetType::Assignment => "Asignación",
            NetType::ShortestPath => "Ruta más corta",
            NetType::MaxFlow => "Flujo máximo",
            NetType::MinSpanningTree => "Árbol de expansión mínima",
            NetType::Tsp => "Agente viajero",
        }
    }
}

/// Nodo con oferta y demanda.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct NetNode {
    pub name: String,
    /// Oferta (fuentes del transporte, nodos con oferta en flujo en redes).
    #[serde(default)]
    pub supply: f64,
    /// Demanda (destinos del transporte, nodos con demanda en flujo en redes).
    /// Con `max_demand`, es la demanda mínima.
    #[serde(default)]
    pub demand: f64,
    /// Flujo en redes: lo máximo que puede recibir el nodo ("M" = sin límite).
    /// Si falta, recibe exactamente su demanda.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_demand: Option<Bnd>,
    /// Flujo en redes: costo por unidad que pasa por el nodo (manejo en un centro de transbordo).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handling_cost: Option<f64>,
    /// Flujo en redes: unidades que puede manejar el nodo como máximo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<f64>,
    /// Posición del nodo en el editor visual (coordenadas de pantalla, Y hacia
    /// abajo). Solo sirve para dibujar: no cambia la solución ni se guarda en
    /// los archivos de WinQSB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
}

impl NetNode {
    /// ¿Usa los datos de nodo que WinQSB no tiene (costo, capacidad, demanda con rango)?
    pub fn extended(&self) -> bool {
        self.max_demand.is_some() || self.handling_cost.is_some_and(|c| c != 0.0) || self.capacity.is_some()
    }
}

/// Cotas de flujo de un arco (solo flujo en redes).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ArcBound {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub lower: f64,
    /// null = sin límite.
    #[serde(default)]
    pub upper: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NetModel {
    #[serde(default)]
    pub name: String,
    pub problem_type: NetType,
    #[serde(default)]
    pub objective: Objective,
    /// Red no dirigida: el coste de i a j vale también de j a i (árbol, agente viajero, rutas).
    #[serde(default)]
    pub symmetric: bool,
    /// Orígenes (transporte) u objetos (asignación).
    #[serde(default)]
    pub sources: Vec<NetNode>,
    /// Destinos (transporte) o tareas (asignación).
    #[serde(default)]
    pub destinations: Vec<NetNode>,
    /// Costes origen × destino; null = no hay conexión.
    #[serde(default)]
    pub costs: Vec<Vec<Option<f64>>>,
    /// Nodos (resto de tipos).
    #[serde(default)]
    pub nodes: Vec<NetNode>,
    /// Matriz nodo × nodo: coste, distancia o capacidad del arco; null = no hay arco.
    #[serde(default)]
    pub arcs: Vec<Vec<Option<f64>>>,
    /// Cotas de flujo por arco (flujo en redes).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bounds: Vec<ArcBound>,
    /// Nodo de inicio (ruta más corta) o fuente (flujo máximo). Por defecto, el primero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    /// Nodo final (ruta más corta) o sumidero (flujo máximo). Por defecto, el último.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    /// Posiciones guardadas en el modelo gráfico de WinQSB (fila, columna).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub positions: Option<Vec<(i64, i64)>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum InitialMethod {
    NorthwestCorner,
    MinimumCost,
    #[default]
    Vogel,
    Russell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum TspMethod {
    /// Óptimo con HiGHS y cortes de subciclos.
    #[default]
    Exact,
    NearestNeighbor,
    CheapestInsertion,
    /// Vecino más cercano mejorado con intercambios 2-opt.
    TwoOpt,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct NetOptions {
    /// Mostrar el método paso a paso.
    pub steps: bool,
    /// Solución inicial del método de transporte.
    pub initial: InitialMethod,
    /// Método para el agente viajero.
    pub tsp_method: TspMethod,
    /// Algoritmo didáctico del árbol de expansión: "kruskal" o "prim".
    pub mst_method: String,
    /// Agente viajero: nodo para las ecuaciones de subtour sin relajación (vacío = el primero).
    pub subtour_node: String,
    /// Agente viajero: forma de Miller-Tucker-Zemlin del modelo ("clase" o "libro").
    pub mtz_form: String,
}

impl Default for NetOptions {
    fn default() -> Self {
        NetOptions { steps: true, initial: InitialMethod::Vogel, tsp_method: TspMethod::Exact, mst_method: "kruskal".into(), subtour_node: String::new(), mtz_form: "clase".into() }
    }
}

impl NetModel {
    pub fn validate(&self) -> Result<(), ModError> {
        if self.problem_type.bipartite() {
            check_names("orígenes", &self.sources.iter().map(|n| &n.name).collect::<Vec<_>>())?;
            check_names("destinos", &self.destinations.iter().map(|n| &n.name).collect::<Vec<_>>())?;
            if self.sources.is_empty() || self.destinations.is_empty() {
                return invalid("hace falta al menos un origen y un destino");
            }
            if self.costs.len() != self.sources.len() || self.costs.iter().any(|r| r.len() != self.destinations.len()) {
                return invalid("la matriz de costes debe tener una fila por origen y una columna por destino");
            }
            if self.problem_type == NetType::Transportation {
                for n in self.sources.iter().chain(&self.destinations) {
                    if n.supply < 0.0 || n.demand < 0.0 || !n.supply.is_finite() || !n.demand.is_finite() {
                        return invalid(format!("la oferta y la demanda de {} deben ser números no negativos", n.name));
                    }
                }
            }
        } else {
            check_names("nodos", &self.nodes.iter().map(|n| &n.name).collect::<Vec<_>>())?;
            if self.nodes.len() < 2 {
                return invalid("la red necesita al menos dos nodos");
            }
            let n = self.nodes.len();
            if self.arcs.len() != n || self.arcs.iter().any(|r| r.len() != n) {
                return invalid("la matriz de arcos debe ser cuadrada, con una fila y una columna por nodo");
            }
        }
        let all = self.costs.iter().chain(&self.arcs).flatten().flatten();
        if all.clone().any(|x| !x.is_finite()) {
            return invalid("hay costes o distancias que no son números finitos");
        }
        Ok(())
    }

    /// Coste del arco i→j en los problemas de nodos (respeta la simetría).
    pub fn arc(&self, i: usize, j: usize) -> Option<f64> {
        let a = self.arcs[i][j];
        if self.symmetric && a.is_none() {
            self.arcs[j][i]
        } else {
            a
        }
    }

    pub fn node_index(&self, name: &str, default: usize) -> Result<usize, ModError> {
        if name.trim().is_empty() {
            return Ok(default);
        }
        self.nodes
            .iter()
            .position(|n| n.name.eq_ignore_ascii_case(name.trim()))
            .ok_or_else(|| ModError::Invalid(format!("no existe el nodo «{name}»")))
    }
}

fn node_names(prefix: &str, n: usize) -> Vec<NetNode> {
    (1..=n).map(|i| NetNode { name: format!("{prefix}{i}"), supply: 0.0, demand: 0.0, ..Default::default() }).collect()
}

pub struct Net;

impl TypedModule for Net {
    type Model = NetModel;
    type Options = NetOptions;

    fn file_tag(&self) -> &'static str {
        "NET"
    }

    fn spec(&self) -> ModuleSpec {
        let obj = Field::choice("objective", "Criterio", &[("min", "Minimizar"), ("max", "Maximizar")]);
        let nodes_dim = Field::integer("nodes", "Número de nodos", 5);
        let bip_layout = |supply: bool| {
            let mut rows = Axis::new("sources", "Origen").add("Añadir origen");
            let mut cols = Axis::new("destinations", "Destino").add("Añadir destino");
            if supply {
                rows = rows.extra(Field::number("supply", "Oferta"));
                cols = cols.extra(Field::number("demand", "Demanda"));
            }
            vec![
                Widget::params(None, vec![obj.clone()]),
                Widget::matrix(Some("Costes"), "costs", rows, cols, Field::number("c", "Coste").optional()),
            ]
        };
        let node_layout = |supply: bool, label: &str, sym: bool, ends: bool| {
            let mut rows = Axis::new("nodes", "Desde").add("Añadir nodo");
            let mut cols = Axis::new("nodes", "Hasta");
            if supply {
                rows = rows
                    .extra(Field::number("supply", "Oferta"))
                    .extra(Field::number("handling_cost", "Costo en el nodo").optional().help("Costo por unidad que pasa por el nodo (centro de transbordo)"))
                    .extra(Field::number("capacity", "Capacidad del nodo").optional().help("Unidades que puede manejar el nodo; vacío = sin límite"));
                cols = cols
                    .extra(Field::number("demand", "Demanda"))
                    .extra(
                        Field::bound("max_demand", "Demanda máxima", Value::Null)
                            .optional()
                            .help("Vacío = recibe exactamente su demanda. Con un valor o M, la demanda es el mínimo y puede recibir hasta este máximo"),
                    );
            }
            let mut params = vec![obj.clone()];
            if sym {
                params.push(Field::boolean("symmetric", "Red no dirigida (simétrica)", false));
            }
            if ends {
                params.push(Field::text("start", "Nodo inicial").optional().help("Vacío = el primero"));
                params.push(Field::text("end", "Nodo final").optional().help("Vacío = el último"));
            }
            let mut w = vec![
                Widget::params(None, params),
                Widget::matrix(Some(label), "arcs", rows, cols, Field::number("c", label).optional()),
            ];
            if supply {
                w.push(Widget::records(
                    Some("Cotas de flujo (opcional)"),
                    "bounds",
                    vec![
                        Field::text("from", "Desde"),
                        Field::text("to", "Hasta"),
                        Field::number("lower", "Mínimo"),
                        Field::number("upper", "Máximo").optional(),
                    ],
                    Some("Añadir cota"),
                ));
            }
            w
        };
        let pt = |t: NetType, desc: &str, dims: Vec<Field>, layout: Vec<Widget>| ProblemType {
            id: serde_json::to_value(t).unwrap().as_str().unwrap().to_string(),
            label: t.label().into(),
            description: desc.into(),
            dims,
            layout,
        };
        ModuleSpec {
            id: "net".into(),
            winqsb: "NET".into(),
            name: "Modelos de redes".into(),
            description: "Transporte, asignación, flujo en redes, ruta más corta, flujo máximo, árbol de expansión mínima y agente viajero".into(),
            file_ext: "NET".into(),
            problem_types: vec![
                pt(
                    NetType::Transportation,
                    "Enviar la oferta de los orígenes a la demanda de los destinos al menor coste",
                    vec![Field::integer("sources", "Orígenes", 3), Field::integer("destinations", "Destinos", 4)],
                    bip_layout(true),
                ),
                pt(
                    NetType::Assignment,
                    "Asignar cada objeto a una tarea al menor coste (método húngaro)",
                    vec![Field::integer("sources", "Objetos", 4), Field::integer("destinations", "Tareas", 4)],
                    bip_layout(false),
                ),
                pt(
                    NetType::NetworkFlow,
                    "Mover flujo entre nodos con oferta y demanda al menor costo (transbordo), con costo y capacidad en los nodos y demandas con mínimo y máximo",
                    vec![nodes_dim.clone()],
                    node_layout(true, "Coste por unidad", false, false),
                ),
                pt(
                    NetType::ShortestPath,
                    "Camino de menor distancia entre dos nodos",
                    vec![nodes_dim.clone()],
                    node_layout(false, "Distancia", true, true),
                ),
                pt(
                    NetType::MaxFlow,
                    "Máximo flujo de la fuente al sumidero según las capacidades",
                    vec![nodes_dim.clone()],
                    node_layout(false, "Capacidad", false, true),
                ),
                pt(
                    NetType::MinSpanningTree,
                    "Conectar todos los nodos con la menor longitud total",
                    vec![nodes_dim.clone()],
                    node_layout(false, "Distancia", false, false),
                ),
                pt(
                    NetType::Tsp,
                    "Recorrer todos los nodos una vez y volver al inicio con la menor distancia",
                    vec![nodes_dim],
                    node_layout(false, "Distancia", true, false),
                ),
            ],
            options: vec![
                Field::boolean("steps", "Mostrar paso a paso", true),
                Field::choice(
                    "initial",
                    "Solución inicial",
                    &[("vogel", "Vogel"), ("northwest_corner", "Esquina noroeste"), ("minimum_cost", "Costo mínimo"), ("russell", "Russell")],
                ).comparable()
                .show_if("problem_type", &["transportation"]),
                Field::choice(
                    "tsp_method",
                    "Método",
                    &[
                        ("exact", "Óptimo (cortes de subciclos)"),
                        ("nearest_neighbor", "Vecino más cercano"),
                        ("cheapest_insertion", "Inserción más barata"),
                        ("two_opt", "Vecino más cercano + 2-opt"),
                    ],
                ).comparable()
                .show_if("problem_type", &["tsp"]),
                Field::choice("mst_method", "Algoritmo", &[("kruskal", "Kruskal"), ("prim", "Prim")]).comparable().show_if("problem_type", &["min_spanning_tree"]),
                Field::choice(
                    "mtz_form",
                    "Relajación de subtours",
                    &[("clase", "Como en clase: U_i − U_j + (n−1)x_ij ≤ n−2"), ("libro", "Como en libros: U_i − U_j + n·x_ij ≤ n−1")],
                )
                .show_if("problem_type", &["tsp"]),
                Field::text("subtour_node", "Nodo para las ecuaciones sin relajación")
                    .optional()
                    .help("Vacío = el primero")
                    .show_if("problem_type", &["tsp"]),
            ],
        }
    }

    fn template(&self, problem_type: &str, dims: &Value) -> Result<NetModel, ModError> {
        let t: NetType = serde_json::from_value(Value::from(problem_type))
            .map_err(|_| ModError::Invalid(format!("tipo de problema desconocido: {problem_type}")))?;
        let mut m = NetModel {
            name: format!("{} nuevo", t.label()),
            problem_type: t,
            objective: Objective::Min,
            symmetric: false,
            sources: vec![],
            destinations: vec![],
            costs: vec![],
            nodes: vec![],
            arcs: vec![],
            bounds: vec![],
            start: None,
            end: None,
            positions: None,
        };
        if t.bipartite() {
            let (a, b) = if t == NetType::Assignment { ("Objeto", "Tarea") } else { ("Origen", "Destino") };
            let s = dim(dims, "sources", 3, 1, 500)?;
            let d = dim(dims, "destinations", 4, 1, 500)?;
            m.sources = node_names(a, s);
            m.destinations = node_names(b, d);
            m.costs = vec![vec![Some(0.0); d]; s];
        } else {
            let n = dim(dims, "nodes", 5, 2, 500)?;
            m.nodes = node_names("Nodo", n);
            m.arcs = vec![vec![None; n]; n];
            m.symmetric = matches!(t, NetType::MinSpanningTree | NetType::Tsp);
        }
        Ok(m)
    }

    fn solve(&self, m: &NetModel, o: &NetOptions) -> Result<Report, ModError> {
        m.validate()?;
        let mut rep = match m.problem_type {
            NetType::Transportation => transport::solve(m, o)?,
            NetType::Assignment => assign::solve(m, o)?,
            NetType::NetworkFlow => flow::solve(m, o)?,
            NetType::ShortestPath => paths::shortest(m, o)?,
            NetType::MaxFlow => paths::max_flow(m, o)?,
            NetType::MinSpanningTree => paths::mst(m, o)?,
            NetType::Tsp => tsp::solve(m, o)?,
        };
        if rep.problem.is_empty() {
            rep.problem = m.name.clone();
        }
        Ok(fuentes::cite(rep, m, o))
    }

    fn read(&self, bytes: &[u8]) -> Result<NetModel, ModError> {
        format::read(bytes)
    }

    fn write(&self, m: &NetModel) -> Result<Vec<u8>, ModError> {
        // WinQSB no tiene costo ni capacidad en los nodos: se guarda la red equivalente
        Ok(format::write(&flowx::expand(m)))
    }

    fn examples(&self) -> Vec<(&'static str, NetModel)> {
        let ex = |s: &str| serde_json::from_str::<NetModel>(s).unwrap();
        vec![
            ("transporte", ex(include_str!("ejemplos/transporte.json"))),
            ("asignacion", ex(include_str!("ejemplos/asignacion.json"))),
            ("ruta_mas_corta", ex(include_str!("ejemplos/ruta.json"))),
            ("flujo_maximo", ex(include_str!("ejemplos/flujo_maximo.json"))),
            ("arbol_minimo", ex(include_str!("ejemplos/arbol.json"))),
            ("agente_viajero", ex(include_str!("ejemplos/tsp.json"))),
            ("transbordo", ex(include_str!("ejemplos/transbordo.json"))),
            ("clase_transbordo_caso1", ex(include_str!("ejemplos/clase_transbordo_caso1.json"))),
            ("clase_transbordo_caso2", ex(include_str!("ejemplos/clase_transbordo_caso2.json"))),
            ("clase_transbordo_caso3", ex(include_str!("ejemplos/clase_transbordo_caso3.json"))),
            ("clase_problema1", ex(include_str!("ejemplos/clase_problema1.json"))),
            ("clase_ruta_binaria", ex(include_str!("ejemplos/clase_ruta_binaria.json"))),
            ("clase_ruta_ejercicio1", ex(include_str!("ejemplos/clase_ruta_ejercicio1.json"))),
            ("clase_arbol_ejercicio3", ex(include_str!("ejemplos/clase_arbol_ejercicio3.json"))),
            ("clase_arbol_ejercicio4", ex(include_str!("ejemplos/clase_arbol_ejercicio4.json"))),
            ("clase_flujo_maximo", ex(include_str!("ejemplos/clase_flujo_maximo.json"))),
            ("clase_flujo_maximo_2", ex(include_str!("ejemplos/clase_flujo_maximo_2.json"))),
            ("clase_tsp_4", ex(include_str!("ejemplos/clase_tsp_4.json"))),
            ("clase_tsp_6", ex(include_str!("ejemplos/clase_tsp_6.json"))),
            ("clase_tsp_bebidas", ex(include_str!("ejemplos/clase_tsp_bebidas.json"))),
        ]
    }
}
