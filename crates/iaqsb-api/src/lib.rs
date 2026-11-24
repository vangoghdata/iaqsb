//! IAQSB · API común.
//!
//! Todo lo que hacen la app de escritorio, la línea de comandos y el servidor MCP
//! pasa por aquí: una única puerta, con JSON de entrada y de salida.

use schemars::schema_for;
use serde::Serialize;
use serde_json::{json, Value};

pub use iaqsb_core as core;
mod cadena;
mod compare;
mod excel;
mod informe;
pub mod whatif;
pub use informe::{base64_decode, export_report, sanitize, today, warm_up};
/// Dibujo de las gráficas de los informes (SVG y PNG) y resumen en Markdown.
pub use iaqsb_render as render;
pub use compare::{compare, comparable_fields};
pub use iaqsb_formats as formats;
pub use iaqsb_lp as lp;
pub use iaqsb_modules as modules;

use iaqsb_core::i18n::{self, translate_value, Lang};
use iaqsb_core::solver;
use iaqsb_core::{ModError, Module};
use std::sync::OnceLock;

pub const PRODUCT: &str = "IAQSB";
pub const TAGLINE: &str = "Hecho por IA Nativa";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("módulo desconocido: {0}. Usa list_modules para ver los disponibles")]
    UnknownModule(String),
    #[error("el módulo {0} todavía no está disponible en esta versión")]
    NotYet(String),
    #[error("datos no válidos: {0}")]
    BadInput(String),
    #[error(transparent)]
    Lp(#[from] iaqsb_lp::LpError),
    #[error(transparent)]
    Format(#[from] iaqsb_formats::FormatError),
    #[error(transparent)]
    Module(#[from] ModError),
    /// Error ya traducido (código, mensaje).
    #[error("{1}")]
    Translated(&'static str, String),
}

impl ApiError {
    /// El mismo error con el mensaje en el idioma pedido.
    pub fn in_lang(self, lang: Lang) -> ApiError {
        if lang == Lang::Es {
            return self;
        }
        let msg = i18n::translate(&self.to_string(), lang);
        ApiError::Translated(self.code(), msg)
    }

    pub fn code(&self) -> &'static str {
        match self {
            ApiError::UnknownModule(_) => "unknown_module",
            ApiError::NotYet(_) => "not_available",
            ApiError::BadInput(_) | ApiError::Lp(_) => "invalid_input",
            ApiError::Format(_) | ApiError::Module(ModError::Format(_)) => "invalid_file",
            ApiError::Module(ModError::Invalid(_)) => "invalid_input",
            ApiError::Translated(c, _) => c,
        }
    }
}

pub type Result<T> = std::result::Result<T, ApiError>;

#[derive(Debug, Clone, Serialize)]
pub struct ModuleInfo {
    pub id: &'static str,
    /// Archivo del módulo original de WinQSB.
    pub winqsb: &'static str,
    pub name: &'static str,
    pub solves: &'static str,
}

/// Los 19 módulos de WinQSB y los que agrega IAQSB, en el orden del menú.
pub const MODULES: &[ModuleInfo] = &[
    ModuleInfo { id: "lp", winqsb: "LP-ILP", name: "Programación lineal y entera", solves: "Modelos lineales con variables continuas, enteras, binarias y libres: símplex, branch and bound, sensibilidad y método gráfico" },
    ModuleInfo { id: "net", winqsb: "NET", name: "Modelos de redes", solves: "Transporte, asignación, flujo en redes, ruta más corta, flujo máximo, árbol de expansión mínima y agente viajero" },
    ModuleInfo { id: "pert", winqsb: "PERT-CPM", name: "PERT y ruta crítica", solves: "Ruta crítica, holguras, probabilidad de terminación, compresión de tiempos y simulación" },
    ModuleInfo { id: "gp", winqsb: "GP-IGP", name: "Programación por metas", solves: "Metas con prioridades, continuas y enteras" },
    ModuleInfo { id: "qp", winqsb: "QP-IQP", name: "Programación cuadrática", solves: "Objetivo cuadrático con restricciones lineales, continuas y enteras" },
    ModuleInfo { id: "nlp", winqsb: "NLP", name: "Programación no lineal", solves: "Optimización no lineal con y sin restricciones" },
    ModuleInfo { id: "dp", winqsb: "DP", name: "Programación dinámica", solves: "Etapas (diligencia), mochila y producción-inventario" },
    ModuleInfo { id: "da", winqsb: "DA", name: "Análisis de decisiones", solves: "Tablas de pagos, árboles de decisión, análisis bayesiano y teoría de juegos" },
    ModuleInfo { id: "mkp", winqsb: "MKP", name: "Procesos de Markov", solves: "Probabilidades de transición, estado estable y cadenas absorbentes" },
    ModuleInfo { id: "qa", winqsb: "QA", name: "Teoría de colas", solves: "Colas de una etapa: M/M/s exacto con capacidad y población finitas, lotes, servicio presionado y llegadas desalentadas; M/G/1, G/G/s y simulación; costos, capacidad y sensibilidad" },
    ModuleInfo { id: "qss", winqsb: "QSS", name: "Simulación de colas", solves: "Simulación por eventos discretos de sistemas de varias etapas: poblaciones, servidores, colas y recolectores, reglas de entrada y salida (ensamble y desensamble), 10 disciplinas, bloqueo, lotes y tipos de cliente" },
    ModuleInfo { id: "its", winqsb: "ITS", name: "Inventarios", solves: "Lote económico, descuentos, vendedor de periódicos, tamaño de lote dinámico y sistemas de revisión" },
    ModuleInfo { id: "fc", winqsb: "FC", name: "Pronósticos y regresión", solves: "Promedios móviles, suavizamiento exponencial, Holt-Winters y regresión" },
    ModuleInfo { id: "st", winqsb: "Nuevo", name: "Series de tiempo (ARIMA, ETS, TBATS)", solves: "Exploración de series (autocorrelación, Ljung-Box, descomposición, Box-Cox) y pronósticos modernos: ARIMA, ETS, TBATS, Theta y Croston automáticos, comparación de modelos en un periodo de prueba e intervalos de predicción, con el mejor motor disponible (R con forecast, Python con statsforecast o el nativo en Rust)" },
    ModuleInfo { id: "ap", winqsb: "AP", name: "Planificación agregada", solves: "Planes de producción agregada simples, de transporte y lineales" },
    ModuleInfo { id: "pmp", winqsb: "Nuevo", name: "Programa maestro de producción", solves: "Programa maestro básico y de ensamble bajo pedido (ATO): barreras de tiempo, disponible para promesa, respuesta a solicitudes de pedidos, capacidad aproximada (RCCP) y PMP óptimo por programación entera" },
    ModuleInfo { id: "mrp", winqsb: "MRP", name: "Planificación de requerimientos de materiales", solves: "Explosión de la lista de materiales y lotificación" },
    ModuleInfo { id: "mrp2", winqsb: "Nuevo", name: "MRP II por matrices (Gozinto)", solves: "Explosión de materiales con la matriz Gozinto: requerimientos brutos y netos, demanda neta, necesidades a programar, tiempos de máquina frente a la capacidad de cada departamento y ajuste por calidad" },
    ModuleInfo { id: "job", winqsb: "JOB", name: "Programación de trabajos", solves: "Secuenciación en taller y en flujo con reglas de prioridad y Johnson" },
    ModuleInfo { id: "fll", winqsb: "FLL", name: "Localización y distribución de instalaciones", solves: "Localización, distribución en planta y balanceo de líneas" },
    ModuleInfo { id: "qcc", winqsb: "QCC", name: "Gráficos de control de calidad", solves: "36 gráficos de control por variables y atributos (Shewhart, CUSUM, EWMA, medias móviles, tendencia, modificados, de aceptación y estandarizados), reglas de fuera de control, curva OC, capacidad, histograma con ji cuadrada, gráfico de probabilidad y Pareto" },
    ModuleInfo { id: "asa", winqsb: "ASA", name: "Muestreo de aceptación", solves: "Planes por atributos (simple, doble, múltiple, secuencial, ChSP-1, CSP-1, SkSP-2) y por variables (media, métodos k y M, secuencial), curvas OC, AOQ, ATI, ASN y costos, y búsqueda del plan" },
    ModuleInfo { id: "vrp", winqsb: "Nuevo", name: "Ruteo de vehículos", solves: "Rutas de reparto desde un depósito con vehículos de capacidad limitada: barrido, ahorros de Clarke y Wright, vecino más cercano y óptimo por programación entera, con mapa de rutas" },
    ModuleInfo { id: "inv", winqsb: "Nuevo", name: "Gestión de inventarios", solves: "EOQ y POQ al revés (despejes en cadena), clasificación ABC con Pareto, punto de reorden con demanda variable (sistemas continuo y periódico) y simulación día a día de la política (Q, R) con ventas perdidas o back orders, escenarios y búsqueda de la mejor combinación" },
    ModuleInfo { id: "fin", winqsb: "Nuevo", name: "Finanzas del inventario", solves: "Efecto del inventario en la rentabilidad (ROI con escenarios y reducción de inventario), en la liquidez (razón corriente, prueba ácida e inventario obsoleto) y en el capital de trabajo (ciclo operativo, desfase y utilidad por ciclo, también al revés)" },
    ModuleInfo { id: "alm", winqsb: "Nuevo", name: "Almacenes", solves: "Paletizado (cajas por estiba según altura, montacargas y resistencia), estibas por lote y área mínima de la bodega en bloque con plano a escala; dimensionado del almacén propio o rentado con alternativas, costo mes a mes y tamaño óptimo" },
    ModuleInfo { id: "kpi", winqsb: "Nuevo", name: "Indicadores logísticos", solves: "A partir de los pedidos: entregas a tiempo, completas, documentación, pedido perfecto, OTIF, ciclo de la orden, urgentes frente a estándar (prueba t de Welch) y costo logístico sobre ventas frente al estándar" },
    ModuleInfo { id: "modo", winqsb: "Nuevo", name: "Selección del modo de transporte", solves: "Trade-off transporte-inventarios: costo total de cada medio (flete más inventario en la planta, en tránsito y en el centro de distribución), gráficos y sensibilidad a la demanda" },
];

fn registry() -> &'static Vec<Box<dyn Module>> {
    static R: OnceLock<Vec<Box<dyn Module>>> = OnceLock::new();
    R.get_or_init(iaqsb_modules::all)
}

/// Módulo genérico registrado, si existe.
pub fn generic(id: &str) -> Option<&'static dyn Module> {
    let info = module(id).ok()?;
    registry().iter().find(|m| m.spec().id == info.id).map(|b| b.as_ref())
}

fn module(id: &str) -> Result<&'static ModuleInfo> {
    let key = id.trim().to_ascii_lowercase();
    MODULES
        .iter()
        .find(|m| m.id == key || m.winqsb.eq_ignore_ascii_case(&key))
        .ok_or_else(|| ApiError::UnknownModule(id.into()))
}

fn is_available(id: &str) -> bool {
    id == "lp" || generic(id).is_some()
}

fn available(id: &str) -> Result<&'static ModuleInfo> {
    let m = module(id)?;
    if !is_available(m.id) {
        return Err(ApiError::NotYet(m.winqsb.into()));
    }
    Ok(m)
}

fn from_value<T: serde::de::DeserializeOwned>(v: Value, what: &str) -> Result<T> {
    serde_json::from_value(v).map_err(|e| ApiError::BadInput(format!("{what}: {e}")))
}

/// Lista de módulos.
pub fn list_modules() -> Value {
    let mods: Vec<Value> = MODULES
        .iter()
        .map(|m| {
            let ext = if m.id == "lp" { "LPP".to_string() } else { generic(m.id).map(|g| g.spec().file_ext).unwrap_or_default() };
            json!({"id": m.id, "winqsb": m.winqsb, "name": m.name, "solves": m.solves, "file_ext": ext, "available": is_available(m.id)})
        })
        .collect();
    json!({ "product": PRODUCT, "version": VERSION, "tagline": TAGLINE, "modules": mods })
}

/// Esquemas JSON del modelo, las opciones y un ejemplo de un módulo.
pub fn describe_module(id: &str) -> Result<Value> {
    let m = available(id)?;
    if m.id == "lp" {
        return Ok(json!({
            "module": m,
            "model_schema": schema_for!(lp::LpModel),
            "options_schema": schema_for!(lp::SolveOptions),
            "report_schema": schema_for!(lp::LpReport),
            "example": example_model("lp")?,
            "notes": [
                "Las cotas aceptan números o \"M\" / \"-M\" para infinito, como en WinQSB.",
                "Tipos de variable: continuous, integer, binary, unrestricted.",
                "Si hay variables enteras o binarias se usa branch and bound; el análisis de sensibilidad solo se da para problemas continuos.",
                "options.trace = true devuelve las tablas del símplex de cada iteración."
            ]
        }));
    }
    let g = generic(m.id).unwrap();
    let spec = g.spec();
    Ok(json!({
        "module": m,
        "problem_types": spec.problem_types.iter().map(|p| json!({"id": p.id, "label": p.label, "description": p.description})).collect::<Vec<_>>(),
        "model_schema": g.model_schema(),
        "options_schema": g.options_schema(),
        "examples": g.examples().into_iter().map(|(n, v)| json!({"name": n, "model": v})).collect::<Vec<_>>(),
        "notes": [
            "Cada modelo lleva problem_type. Usa template para obtener un modelo en blanco con las dimensiones que quieras.",
            "El informe trae headline (resumen), data (resultados estructurados) y sections (bloques para mostrar).",
            "options.steps = true incluye el método paso a paso."
        ]
    }))
}

/// Especificación de entradas de un módulo genérico (para el editor de la aplicación).
pub fn spec(id: &str) -> Result<Value> {
    let m = available(id)?;
    let g = generic(m.id).ok_or_else(|| ApiError::BadInput("LP-ILP tiene su propio editor".into()))?;
    Ok(serde_json::to_value(g.spec()).unwrap())
}

/// Modelo en blanco.
pub fn template(id: &str, problem_type: &str, dims: &Value) -> Result<Value> {
    let m = available(id)?;
    let g = generic(m.id).ok_or_else(|| ApiError::BadInput("LP-ILP no usa plantillas".into()))?;
    Ok(g.template(problem_type, dims)?)
}

/// Resuelve un modelo del módulo indicado.
pub fn solve(id: &str, model: Value, options: Option<Value>) -> Result<Value> {
    let m = available(id)?;
    if m.id == "lp" {
        let source = example_of("lp", &model).and_then(|n| iaqsb_core::biblio::data_source("lp", &n));
        let opts: lp::SolveOptions = match options {
            Some(Value::Null) | None => Default::default(),
            Some(mut o) => {
                // también se aceptan los parámetros del motor dentro de "engine"
                if let Some(e) = o.get("engine").cloned().filter(|e| e.is_object()) {
                    if let (Some(oo), Some(ee)) = (o.as_object_mut(), e.as_object()) {
                        oo.remove("engine");
                        for (k, v) in ee {
                            oo.entry(k.clone()).or_insert(v.clone());
                        }
                    }
                }
                from_value(o, "opciones")?
            }
        };
        let mut algebra: Option<Box<lp::algebra::Compiled>> = None;
        let (rep, enteras) = match lp_input(model)? {
            LpInput::Model(model) => {
                let enteras = model.variables.iter().any(|v| matches!(v.var_type, lp::VarType::Integer | lp::VarType::Binary | lp::VarType::SemiInteger));
                (lp::solve(&model, &opts)?, enteras)
            }
            LpInput::Engine(em) => {
                let enteras = em.is_mip();
                (lp::engine::solve_engine(&em, &opts.engine_params(), opts.diagnose), enteras)
            }
            LpInput::Algebra(c) => {
                let enteras = c.model.is_mip();
                let externo = opts.engine_params().solver.as_deref().is_some_and(|x| !x.eq_ignore_ascii_case("highs"));
                let mut r = match if externo { None } else { qp_global(&c.model)? } {
                    // objetivo cuadrático no convexo o con enteras: búsqueda global del módulo QP-IQP
                    // convexo con enteras: las enteras se fijan donde quedaron y HiGHS
                    // resuelve exacto el cuadrático convexo que queda
                    Some((x, _, nota, true)) => {
                        let mut fijo = c.model.clone();
                        for (col, v) in fijo.cols.iter_mut().zip(&x) {
                            if matches!(col.kind, solver::Kind::Integer | solver::Kind::Binary) {
                                col.lb = v.round();
                                col.ub = v.round();
                                col.kind = solver::Kind::Continuous;
                            }
                        }
                        let mut r = lp::engine::solve_engine(&fijo, &opts.engine_params(), false);
                        if r.status == lp::Status::Optimal {
                            r.message = format!("Solución entera óptima: {}.", lp::model::fmt_num(r.objective_value.unwrap_or(0.0)));
                        }
                        r.sensitivity = false;
                        r.method = "Búsqueda global de QP-IQP (ramificación y acotamiento) y HiGHS con las enteras fijas".into();
                        r.extra.notes.extend(nota);
                        r
                    }
                    Some((x, z, nota, false)) => {
                        let local = nota.len() > 1;
                        let mut fijo = c.model.clone();
                        fijo.q.clear();
                        for (col, v) in fijo.cols.iter_mut().zip(&x) {
                            let v = if matches!(col.kind, solver::Kind::Integer | solver::Kind::Binary) { v.round() } else { *v };
                            col.lb = v;
                            col.ub = v;
                        }
                        let mut r = lp::engine::solve_engine(&fijo, &opts.engine_params(), false);
                        r.objective_value = Some(z);
                        r.sensitivity = false;
                        r.alternate_optima = false;
                        r.message = if local {
                            // sin prueba de optimalidad global: se dice así
                            r.status = lp::Status::Feasible;
                            format!("Óptimo local (sin garantía de ser el global): objetivo = {}.", lp::model::fmt_num(z))
                        } else if fijo.is_mip() {
                            format!("Solución entera óptima: {}.", lp::model::fmt_num(z))
                        } else {
                            format!("Solución óptima encontrada. Objetivo = {}.", lp::model::fmt_num(z))
                        };
                        // sin sensibilidad: los costos reducidos del modelo fijado no significan nada
                        for v in &mut r.variables {
                            v.reduced_cost = 0.0;
                            v.cost_min = None;
                            v.cost_max = None;
                        }
                        for c in &mut r.constraints {
                            c.shadow_price = None;
                            c.rhs_min = None;
                            c.rhs_max = None;
                        }
                        r.method = "Búsqueda global de QP-IQP (puntos KKT y ramificación y acotamiento)".into();
                        r.extra.notes.extend(nota);
                        r
                    }
                    None => lp::engine::solve_engine(&c.model, &opts.engine_params(), opts.diagnose),
                };
                let por_qp = r.method.contains("QP-IQP");
                r.extra.notes.extend(c.notes.iter().filter(|n| !(por_qp && n.starts_with("Objetivo cuadrático con"))).cloned());
                algebra = Some(c);
                (r, enteras)
            }
        };
        let headline = lp_headline(&rep);
        let mut v = serde_json::to_value(rep).unwrap();
        v["headline"] = Value::String(headline);
        // modelo algebraico: la solución por índices (x[i,j] en tabla de doble entrada)
        if let Some(c) = &algebra {
            let col = |k: &str| -> Vec<f64> { v["variables"].as_array().map(|a| a.iter().map(|x| x[k].as_f64().unwrap_or(0.0)).collect()).unwrap_or_default() };
            let row = |k: &str| -> Vec<f64> { v["constraints"].as_array().map(|a| a.iter().map(|x| x[k].as_f64().unwrap_or(0.0)).collect()).unwrap_or_default() };
            let ok = v["status"] == "optimal" || v["status"] == "feasible";
            if ok {
                let (x, rc, lhs, sp) = (col("value"), col("reduced_cost"), row("lhs"), row("shadow_price"));
                let lp_dual = v["sensitivity"].as_bool() == Some(true);
                v["tables"] = json!(lp::algebra::tables(c, &x, lp_dual.then_some(rc.as_slice()), lp_dual.then_some(sp.as_slice()), Some(&lhs)));
            }
            v["algebra"] = lp::algebra::stats(c);
        }
        // fuentes: métodos usados y origen de los datos, con la misma sección «Fuentes» de los demás módulos
        let mut refs: Vec<iaqsb_core::biblio::Citation> = vec![];
        let flag = if enteras { "true" } else { "false" };
        let metodo = v["method"].as_str().unwrap_or("").to_string();
        let diag = if v.get("diagnosis").is_some_and(|d| !d.is_null()) { "true" } else { "false" };
        for (id, para) in iaqsb_core::biblio::method_citations("lp", |k| match k {
            "enteras" => vec![flag.into()],
            "metodo" => vec![metodo.clone()],
            "diagnostico" => vec![diag.into()],
            _ => vec![],
        }) {
            add_citation(&mut refs, id, para);
        }
        let block = iaqsb_core::report::Block::Sources { data: source.clone(), refs: refs.clone() };
        v["references"] = serde_json::to_value(&refs).unwrap();
        if let Some(d) = &source {
            v["data_source"] = serde_json::to_value(d).unwrap();
        }
        v["sections"] = json!([{ "id": "fuentes", "title": "Fuentes", "blocks": [block] }]);
        return Ok(v);
    }
    let g = generic(m.id).unwrap();
    let mut opts = options.unwrap_or(Value::Null);
    // parámetros del motor para los programas lineales y enteros que use el módulo
    let engine: solver::Params = match opts.as_object_mut().and_then(|o| o.remove("engine")) {
        Some(e) => from_value(e, "options.engine")?,
        None => Default::default(),
    };
    let previous = solver::defaults();
    solver::set_defaults(engine);
    let _ = solver::take_runs();
    let solved = g.solve(&model, &opts);
    let runs = solver::take_runs();
    solver::set_defaults(previous);
    let mut rep = solved?;
    if rep.data_source.is_none() {
        rep.data_source = example_of(m.id, &model).and_then(|n| iaqsb_core::biblio::data_source(m.id, &n));
    }
    // métodos citados según fuentes/metodos.json
    let spec = g.spec();
    let text = |v: &Value| -> Option<String> {
        match v {
            Value::String(s) if !s.is_empty() => Some(s.clone()),
            Value::Bool(b) => Some(b.to_string()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        }
    };
    let cites = iaqsb_core::biblio::method_citations(m.id, |k: &str| -> Vec<String> {
        if k == "seccion" {
            return rep.sections.iter().map(|s| s.id.clone()).collect();
        }
        if k == "problem_type" {
            return model.get(k).and_then(text).into_iter().collect();
        }
        let given = opts.get(k).and_then(text);
        let data = rep.data.get(k).and_then(text);
        let default = spec.options.iter().find(|f| f.key == k).and_then(|f| text(&f.default));
        given.or(data).or(default).or_else(|| model.get(k).and_then(text)).into_iter().collect()
    });
    for (id, para) in cites {
        rep.add_cite(id, para);
    }
    let mut v = serde_json::to_value(rep.finish_sources()).unwrap();
    if !runs.is_empty() {
        v["engine"] = engine_summary(&runs);
    }
    Ok(v)
}

/// Resumen de las resoluciones del motor hechas por un módulo.
fn engine_summary(runs: &[solver::Solution]) -> Value {
    let time: f64 = runs.iter().map(|r| r.time).sum();
    let worst_gap = runs.iter().filter_map(|r| r.gap).fold(0.0f64, f64::max);
    let last = runs.last().unwrap();
    let mut solvers: Vec<String> = runs.iter().map(|r| r.solver.clone()).collect();
    solvers.dedup();
    json!({
        "solver": solvers.join(", "),
        "runs": runs.len(),
        "time": (time * 1e4).round() / 1e4,
        "gap": if runs.iter().any(|r| r.is_mip) { json!(worst_gap) } else { Value::Null },
        "limited": runs.iter().any(|r| !matches!(r.status, solver::Status::Optimal | solver::Status::Infeasible | solver::Status::Unbounded)),
        "cols": runs.iter().map(|r| r.cols).max(), "rows": runs.iter().map(|r| r.rows).max(),
        "status": last.status,
    })
}

fn add_citation(refs: &mut Vec<iaqsb_core::biblio::Citation>, id: &str, para: &str) {
    if let Some(c) = refs.iter_mut().find(|c| c.id == id) {
        if !c.used_for.iter().any(|u| u == para) {
            c.used_for.push(para.into());
        }
    } else {
        refs.push(iaqsb_core::biblio::Citation::new(id, para));
    }
}

/// Un modelo de LP-ILP: la hoja de WinQSB (JSON) o un texto MPS/LP de cualquier tamaño.
pub enum LpInput {
    Model(lp::LpModel),
    Engine(solver::Model),
    /// Modelo escrito en el lenguaje algebraico de IAQSB (con sus índices).
    Algebra(Box<lp::algebra::Compiled>),
}

/// Nombres del formato del lenguaje algebraico.
pub fn is_algebra_format(f: &str) -> bool {
    matches!(lp::algebra::parse::norm(f.trim()).as_str(), "iaqsb" | "ml" | "algebra" | "algebraico" | "algebraica" | "modelo" | "lenguaje" | "linguagem" | "ampl")
}

/// Lee un modelo de LP-ILP: JSON de IAQSB o {"format": "lp" | "mps", "text": "…"}.
pub fn lp_input(model: Value) -> Result<LpInput> {
    if let (Some(f), Some(t)) = (model.get("format").and_then(|v| v.as_str()), model.get("text").and_then(|v| v.as_str())) {
        if is_algebra_format(f) {
            let name = model.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let c = lp::algebra::compile_text(t, name, model.get("data")).map_err(|e| ApiError::BadInput(e.to_string()))?;
            return Ok(LpInput::Algebra(Box::new(c)));
        }
        let fmt = solver::Format::parse(f).ok_or_else(|| ApiError::BadInput(format!("formato desconocido: {f} (usa lp, mps o iaqsb)")))?;
        let mut em = solver::read_text(t, fmt).map_err(ApiError::BadInput)?;
        if let Some(n) = model.get("name").and_then(|v| v.as_str()) {
            em.name = n.to_string();
        }
        return Ok(LpInput::Engine(em));
    }
    let mut m: lp::LpModel = from_value(model, "modelo")?;
    m.normalize()?;
    Ok(LpInput::Model(m))
}

/// El modelo de LP-ILP como modelo del motor (sea cual sea su forma).
/// Variables como máximo para la búsqueda global de un objetivo cuadrático no convexo o entero.
const QP_GLOBAL_MAX: usize = 40;

/// Un objetivo cuadrático que HiGHS no resuelve bien (no convexo para el sentido
/// pedido, o con variables enteras) se resuelve con la búsqueda global del módulo
/// QP-IQP. Devuelve los valores de las variables, el objetivo y una nota; `None` si
/// el motor sirve (lineal o cuadrático convexo continuo).
fn qp_global(m: &solver::Model) -> Result<Option<(Vec<f64>, f64, Vec<String>, bool)>> {
    if m.q.is_empty() {
        return Ok(None);
    }
    let n = m.cols.len();
    let s = if m.maximize { -1.0 } else { 1.0 };
    let semi = m.cols.iter().any(|c| matches!(c.kind, solver::Kind::SemiContinuous | solver::Kind::SemiInteger));
    if semi {
        return Ok(None);
    }
    // convexidad solo con las variables que están en los términos cuadráticos (la parte
    // lineal no la cambia): diagonal → los signos; si no, valores propios de esa submatriz
    let convexa = if m.q.iter().all(|&(r, c, _)| r == c) {
        let mut d = std::collections::HashMap::new();
        for &(r, _, a) in &m.q {
            *d.entry(r).or_insert(0.0) += s * a;
        }
        d.values().all(|v: &f64| *v >= -1e-12)
    } else {
        let mut idx: Vec<usize> = m.q.iter().flat_map(|&(r, c, _)| [r, c]).collect();
        idx.sort_unstable();
        idx.dedup();
        if idx.len() > 2000 {
            // demasiado grande para revisarlo aquí: lo decide el motor
            return Ok(None);
        }
        let pos: std::collections::HashMap<usize, usize> = idx.iter().enumerate().map(|(k, &j)| (j, k)).collect();
        let k = idx.len();
        let mut h = vec![vec![0.0; k]; k];
        for &(r, c, a) in &m.q {
            let (i, j) = (pos[&r], pos[&c]);
            h[i][j] += s * a;
            if i != j {
                h[j][i] += s * a;
            }
        }
        iaqsb_modules::qpsolve::is_convex(&h)
    };
    if convexa && !m.is_mip() {
        return Ok(None);
    }
    if n > QP_GLOBAL_MAX {
        return Err(ApiError::BadInput(format!(
            "el objetivo cuadrático no es convexo (al minimizar) ni cóncavo (al maximizar), o tiene variables enteras, y el modelo tiene {n} variables: la búsqueda global llega hasta {QP_GLOBAL_MAX}. Reformula el objetivo o usa un motor con programación cuadrática no convexa o entera, como Gurobi"
        )));
    }
    // ½ xᵀQx con Q en el triángulo inferior → f = Σ quad[i][j]·xi·xj
    let mut quad = vec![vec![0.0; n]; n];
    for &(r, c, a) in &m.q {
        if r == c {
            quad[r][c] += a / 2.0;
        } else {
            quad[r][c] += a;
        }
    }
    let bnd = |x: f64| if x.is_infinite() { json!(if x > 0.0 { "M" } else { "-M" }) } else { json!(x) };
    let vars: Vec<Value> = m
        .cols
        .iter()
        .map(|c| {
            let kind = match c.kind {
                solver::Kind::Integer => "integer",
                solver::Kind::Binary => "binary",
                _ if c.lb.is_infinite() => "unrestricted",
                _ => "continuous",
            };
            json!({ "name": c.name, "c": c.cost, "lower": bnd(c.lb), "upper": bnd(c.ub), "type": kind })
        })
        .collect();
    let (mut cons, mut coefs) = (vec![], vec![]);
    for r in &m.rows {
        let mut a = vec![0.0; n];
        for &(j, v) in &r.coefs {
            a[j] += v;
        }
        let mut add = |dir: &str, rhs: f64, suf: &str| {
            cons.push(json!({ "name": format!("{}{suf}", r.name), "direction": dir, "rhs": rhs }));
            coefs.push(a.clone());
        };
        if r.lo.is_finite() && r.hi.is_finite() && (r.hi - r.lo).abs() <= 1e-12 {
            add("=", r.hi, "");
        } else {
            if r.lo.is_finite() {
                add(">=", r.lo, if r.hi.is_finite() { " (mín.)" } else { "" });
            }
            if r.hi.is_finite() {
                add("<=", r.hi, if r.lo.is_finite() { " (máx.)" } else { "" });
            }
        }
    }
    let modelo = json!({
        "name": m.name, "problem_type": "quadratic", "objective": if m.maximize { "max" } else { "min" },
        "variables": vars, "quad": quad, "constraints": cons, "coefs": coefs
    });
    let g = generic("qp").ok_or_else(|| ApiError::BadInput("falta el módulo QP-IQP".into()))?;
    let rep = serde_json::to_value(g.solve(&modelo, &json!({ "steps": false }))?).unwrap();
    let st = rep["status"].as_str().unwrap_or("");
    if st != "optimal" && st != "feasible" {
        let msg = rep["headline"].as_str().unwrap_or("").to_string();
        return Err(ApiError::BadInput(format!("la búsqueda global de QP-IQP para el objetivo cuadrático no encontró solución: {msg}")));
    }
    let d = &rep["data"];
    let x: Vec<f64> = d["variables"].as_array().map(|a| a.iter().map(|v| v["value"].as_f64().unwrap_or(0.0)).collect()).unwrap_or_default();
    if x.len() != n {
        return Ok(None);
    }
    let mut x = x;
    let mut z = d["objective_value"].as_f64().unwrap_or(0.0) + m.offset;
    let local = d["local_only"].as_bool() == Some(true);
    if local {
        // sin búsqueda exhaustiva: se mejora con Frank-Wolfe (programación lineal sucesiva)
        // desde varios puntos de partida y se queda el mejor
        if let Some((xb, zb)) = mejorar_local(m, &quad, &x) {
            if (m.maximize && zb > z) || (!m.maximize && zb < z) {
                x = xb;
                z = zb;
            }
        }
    }
    let nota = if convexa {
        "El objetivo cuadrático tiene variables enteras: se usó la búsqueda global del módulo QP-IQP para las enteras. No hay análisis de sensibilidad."
    } else if m.maximize {
        "El objetivo cuadrático no es cóncavo, así que puede tener óptimos locales: se usó la búsqueda global del módulo QP-IQP. No hay análisis de sensibilidad."
    } else {
        "El objetivo cuadrático no es convexo, así que puede tener óptimos locales: se usó la búsqueda global del módulo QP-IQP. No hay análisis de sensibilidad."
    }
    .to_string();
    let mut nota = vec![nota];
    if local {
        nota.push("El modelo es grande para la búsqueda exhaustiva: es el mejor óptimo local encontrado.".into());
    }
    Ok(Some((x, z, nota, convexa)))
}

/// Búsqueda local para un objetivo cuadrático no convexo: Frank-Wolfe con búsqueda
/// exacta en la recta (cada paso resuelve un PL con el gradiente) desde el punto dado,
/// desde el óptimo de la parte lineal y desde vértices al azar (semilla fija).
fn mejorar_local(m: &solver::Model, quad: &[Vec<f64>], x0: &[f64]) -> Option<(Vec<f64>, f64)> {
    let n = m.cols.len();
    let s = if m.maximize { -1.0 } else { 1.0 };
    let c: Vec<f64> = m.cols.iter().map(|c| c.cost).collect();
    // g(x) = s·f(x) (se minimiza); f = c·x + Σ quad_ij x_i x_j
    let f = |x: &[f64]| -> f64 {
        let mut v: f64 = c.iter().zip(x).map(|(a, b)| a * b).sum();
        for i in 0..n {
            for j in 0..n {
                if quad[i][j] != 0.0 {
                    v += quad[i][j] * x[i] * x[j];
                }
            }
        }
        v
    };
    let grad = |x: &[f64]| -> Vec<f64> {
        (0..n).map(|k| s * (c[k] + (0..n).map(|j| (quad[k][j] + quad[j][k]) * x[j]).sum::<f64>())).collect()
    };
    let mut lp = m.clone();
    lp.q.clear();
    lp.objectives.clear();
    lp.maximize = false;
    lp.offset = 0.0;
    let p = solver::Params { time_limit: Some(5.0), ..Default::default() };
    let mut vertice = |w: &[f64]| -> Option<Vec<f64>> {
        for (col, wk) in lp.cols.iter_mut().zip(w) {
            col.cost = *wk;
        }
        let r = solver::solve(&lp, &p);
        (r.has_solution && r.x.len() == n).then_some(r.x)
    };
    let entera = m.is_mip();
    let fw = |mut x: Vec<f64>, vertice: &mut dyn FnMut(&[f64]) -> Option<Vec<f64>>| -> Vec<f64> {
        for _ in 0..200 {
            let g = grad(&x);
            let Some(y) = vertice(&g) else { break };
            let d: Vec<f64> = y.iter().zip(&x).map(|(a, b)| a - b).collect();
            let b: f64 = g.iter().zip(&d).map(|(a, b)| a * b).sum();
            let mut cc = 0.0;
            for i in 0..n {
                for j in 0..n {
                    if quad[i][j] != 0.0 {
                        cc += s * quad[i][j] * d[i] * d[j];
                    }
                }
            }
            // φ(t) = b·t + cc·t², t en [0, 1] (con enteras solo se salta al vértice)
            let t = if entera { 1.0 } else if cc > 1e-15 { (-b / (2.0 * cc)).clamp(0.0, 1.0) } else { 1.0 };
            let mejora = b * t + cc * t * t;
            if mejora > -1e-10 * (1.0 + f(&x).abs()) {
                break;
            }
            x = x.iter().zip(&d).map(|(a, b)| a + t * b).collect();
        }
        x
    };
    let mut inicios: Vec<Vec<f64>> = vec![x0.to_vec()];
    if let Some(v) = vertice(&c.iter().map(|a| s * a).collect::<Vec<_>>()) {
        inicios.push(v);
    }
    let mut semilla: u64 = 0x9E37_79B9_7F4A_7C15;
    for _ in 0..24 {
        let w: Vec<f64> = (0..n)
            .map(|_| {
                semilla ^= semilla << 13;
                semilla ^= semilla >> 7;
                semilla ^= semilla << 17;
                (semilla % 2001) as f64 / 1000.0 - 1.0
            })
            .collect();
        if let Some(v) = vertice(&w) {
            inicios.push(v);
        }
    }
    let mut mejor: Option<(Vec<f64>, f64)> = None;
    for x in inicios {
        let x = fw(x, &mut vertice);
        let v = s * f(&x);
        if mejor.as_ref().is_none_or(|(_, b)| v < *b) {
            mejor = Some((x, v));
        }
    }
    mejor.map(|(x, v)| (x, s * v + m.offset))
}

pub fn lp_engine_model(model: Value) -> Result<solver::Model> {
    Ok(match lp_input(model)? {
        LpInput::Model(m) => {
            m.validate()?;
            lp::engine::to_engine(&m, false)
        }
        LpInput::Engine(e) => e,
        LpInput::Algebra(c) => c.model,
    })
}

/// Convierte un modelo de LP-ILP a otro formato: "lp", "mps", "lpp" (WinQSB) o "json".
pub fn convert_lp(model: Value, to: &str) -> Result<Value> {
    let em = lp_engine_model(model)?;
    let cells = em.cols.len().saturating_mul(em.rows.len());
    Ok(match to.trim().trim_start_matches('.').to_lowercase().as_str() {
        "lp" | "mps" => json!({ "content": solver::write_text(&em, solver::Format::parse(to).unwrap()).map_err(ApiError::BadInput)? }),
        "lpp" | "winqsb" => json!({ "content": formats::decode_ansi(&formats::lpp::write_engine(&em)) }),
        // la hoja escrita en el lenguaje algebraico de IAQSB
        "iaqsb" | "ml" | "algebra" | "algebraico" if cells <= lp::engine::DENSE_LIMIT => json!({ "content": lp::algebra::from_sheet(&lp::engine::from_engine(&em)) }),
        "json" if cells <= lp::engine::DENSE_LIMIT => json!({ "model": lp::engine::from_engine(&em) }),
        "json" => return Err(ApiError::BadInput(format!("el modelo es muy grande para la hoja ({} × {}); guárdalo en MPS o LP", em.cols.len(), em.rows.len()))),
        other => return Err(ApiError::BadInput(format!("formato desconocido: {other} (usa lp, mps, lpp o json)"))),
    })
}

/// Por qué un modelo de LP-ILP no es factible.
pub fn diagnose_lp(model: Value) -> Result<Value> {
    let em = lp_engine_model(model)?;
    let s = solver::solve(&em, &solver::Params::default());
    if s.has_solution {
        return Ok(json!({"feasible": true, "status": s.status, "message": "El modelo es factible."}));
    }
    match lp::engine::diagnose(&em) {
        Some(d) => Ok(json!({"feasible": false, "status": s.status, "diagnosis": d})),
        None => Ok(json!({"feasible": false, "status": s.status, "message": "No se encontró un subsistema infactible (puede ser no acotado)."})),
    }
}

/// Motores disponibles y parámetros que aceptan.
pub fn solver_info() -> Value {
    json!({
        "solvers": solver::external::available(),
        "default": solver::highs_version(),
        "params_schema": schemars::schema_for!(solver::Params),
        "notes": [
            "Los parámetros van en options (LP-ILP) u options.engine (cualquier módulo): solver, time_limit, mip_gap, mip_abs_gap, threads, presolve, method, seed, node_limit, solution_limit, feasibility_tol, integrality_tol, log, start.",
            "Gurobi y CPLEX se usan si están instalados en el equipo (programa o paquete de Python); la licencia es del usuario.",
            "LP-ILP acepta modelos de cualquier tamaño como {\"format\": \"mps\" | \"lp\", \"text\": \"…\"}."
        ]
    })
}

/// Si el modelo es uno de los ejemplos incluidos (mismo nombre), su clave.
pub fn example_of(module: &str, model: &Value) -> Option<String> {
    let name = model.get("name")?.as_str()?;
    if name.trim().is_empty() {
        return None;
    }
    // también reconoce el nombre traducido (los ejemplos se muestran en el idioma de la app)
    examples()
        .into_iter()
        .find(|(m, _, e)| {
            m == module
                && e.get("name").and_then(|x| x.as_str()).map_or(false, |n| n == name || Lang::ALL.iter().any(|l| i18n::translate(n, *l) == name))
        })
        .map(|e| e.1)
}

/// Origen de los datos de un ejemplo (o None si no está registrado).
pub fn example_source(module: &str, name: &str) -> Option<Value> {
    iaqsb_core::biblio::data_source(module, name).map(|d| serde_json::to_value(d).unwrap())
}

/// Bibliografía completa: APA 7, cita en el texto, BibTeX y estado.
pub fn bibliography() -> Value {
    use iaqsb_core::biblio as b;
    Value::Array(
        b::sorted()
            .into_iter()
            .map(|r| {
                json!({
                    "id": r.id, "cite": b::cite(r), "apa": b::apa(r), "link": b::link(r),
                    "verified": b::verified(r), "pending": r.pendiente, "note": r.nota, "topic": r.tema,
                    "bibtex": b::bibtex(r)
                })
            })
            .collect(),
    )
}

/// Frase de resumen del informe de PL (los demás módulos la traen en su `Report`).
fn lp_headline(r: &lp::LpReport) -> String {
    let f = |x: f64| {
        let y = (x * 1e4).round() / 1e4;
        let y = if y == 0.0 { 0.0 } else { y };
        format!("{y}")
    };
    match (r.status, r.objective_value) {
        (lp::Status::Optimal, Some(z)) => {
            let sense = if r.objective == lp::Objective::Max { "Máximo" } else { "Mínimo" };
            let vars: Vec<String> = r.variables.iter().filter(|v| v.value.abs() > 1e-9).take(8).map(|v| format!("{} = {}", v.name, f(v.value))).collect();
            let more = if r.variables.iter().filter(|v| v.value.abs() > 1e-9).count() > 8 { ", …" } else { "" };
            format!("{sense} Z = {} en {}{more}{}.", f(z), if vars.is_empty() { "el origen".into() } else { vars.join(", ") }, if r.alternate_optima { " (hay óptimos alternativos)" } else { "" })
        }
        _ => r.message.clone(),
    }
}

/// Método gráfico (solo LP con dos variables).
pub fn graphic(model: Value) -> Result<Value> {
    let model: lp::LpModel = from_value(model, "modelo")?;
    Ok(serde_json::to_value(lp::graphic::graphic(&model, 0, 1)?).unwrap())
}

/// Importa un archivo de WinQSB y devuelve {module, layout, model}.
pub fn import_bytes(bytes: &[u8]) -> Result<Value> {
    // libro de Excel hecho por IAQSB (trae el módulo)
    if iaqsb_core::zip::is_zip(bytes) {
        return excel::xlsx_to_model(bytes, None, None);
    }
    let head = formats::decode_ansi(&bytes[..bytes.len().min(64)]);
    let up = head.to_ascii_uppercase();
    if up.starts_with("LP") && !up.starts_with("LP-") {
        let (model, layout) = formats::lpp::read(bytes)?;
        return Ok(json!({
            "module": "lp",
            "layout": match layout { formats::lpp::LppLayout::Matrix => "matrix", _ => "normal" },
            "model": model
        }));
    }
    // MPS o LP (los formatos de Gurobi, CPLEX y HiGHS)
    if let Some(fmt) = detect_math_format(bytes) {
        let text = String::from_utf8_lossy(bytes).into_owned();
        let em = solver::read_text(&text, fmt).map_err(ApiError::BadInput)?;
        let size = json!({"cols": em.cols.len(), "rows": em.rows.len(), "nonzeros": em.nonzeros(), "integer": em.cols.iter().filter(|c| c.kind.is_integer()).count()});
        if em.cols.len().saturating_mul(em.rows.len()) <= lp::engine::DENSE_LIMIT {
            return Ok(json!({"module": "lp", "layout": "matrix", "model": lp::engine::from_engine(&em), "size": size}));
        }
        return Ok(json!({"module": "lp", "layout": "matrix", "large": true, "size": size,
            "model": {"name": em.name, "format": fmt.ext(), "text": text}}));
    }
    for g in registry() {
        if g.recognizes(&head) {
            let model = g.read_file(bytes)?;
            return Ok(json!({"module": g.spec().id, "layout": "matrix", "model": model}));
        }
    }
    let tag: String = up.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    Err(ApiError::BadInput(format!(
        "archivo de WinQSB no reconocido o de un módulo aún no disponible (cabecera {tag:?})"
    )))
}

/// ¿Es un archivo MPS o LP? (por su contenido)
pub fn detect_math_format(bytes: &[u8]) -> Option<solver::Format> {
    let text = String::from_utf8_lossy(&bytes[..bytes.len().min(20_000)]);
    let mut words = text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('*') && !l.starts_with('\\'))
        .map(|l| l.split_whitespace().next().unwrap_or("").to_uppercase());
    let first = words.next()?;
    if ["NAME", "ROWS", "OBJSENSE", "OBJSENS"].contains(&first.as_str()) {
        return Some(solver::Format::Mps);
    }
    // las palabras clave pueden estar lejos del inicio (un objetivo con miles de términos)
    let low = String::from_utf8_lossy(bytes).to_lowercase();
    let starts = ["max", "maximize", "maximise", "maximum", "min", "minimize", "minimise", "minimum"].iter().any(|w| first.eq_ignore_ascii_case(w) || first.to_lowercase().starts_with(&format!("{w}:")));
    if starts && (low.contains("subject to") || low.contains("such that") || low.contains("\ns.t.") || low.contains("\nst\n") || low.contains("\nst ") || low.contains("\nend")) {
        return Some(solver::Format::Lp);
    }
    None
}

/// Exporta un modelo al formato de WinQSB. `layout` solo se usa en LP-ILP: "matrix" o "normal".
pub fn export_bytes(id: &str, model: Value, layout: Option<&str>) -> Result<Vec<u8>> {
    let m = available(id)?;
    if m.id == "lp" {
        // MPS o LP de la industria
        if let Some(f) = layout.and_then(solver::Format::parse) {
            let em = lp_engine_model(model)?;
            return solver::write_text(&em, f).map(String::into_bytes).map_err(ApiError::BadInput);
        }
        return match lp_input(model)? {
            LpInput::Model(model) => {
                model.validate()?;
                let l = match layout.unwrap_or("matrix") {
                    "normal" | "NormalModel" => formats::lpp::LppLayout::Normal,
                    _ => formats::lpp::LppLayout::Matrix,
                };
                Ok(formats::lpp::write(&model, l))
            }
            // un modelo grande en texto: WinQSB en forma algebraica (solo los no ceros)
            LpInput::Engine(em) => Ok(formats::lpp::write_engine(&em)),
            LpInput::Algebra(c) => Ok(formats::lpp::write_engine(&c.model)),
        };
    }
    Ok(generic(m.id).unwrap().write_file(&model)?)
}

/// Ejemplos incluidos: (módulo, nombre, modelo).
pub fn examples() -> Vec<(String, String, Value)> {
    let mut v: Vec<(String, String, Value)> = [
        ("wyndor", include_str!("../ejemplos/lp_wyndor.json")),
        ("dieta", include_str!("../ejemplos/lp_dieta.json")),
        ("produccion_entera", include_str!("../ejemplos/lp_produccion_entera.json")),
        ("mochila", include_str!("../ejemplos/lp_mochila.json")),
    ]
    .into_iter()
    .map(|(n, t)| ("lp".to_string(), n.to_string(), serde_json::from_str(t).unwrap()))
    .collect();
    for g in registry() {
        let id = g.spec().id;
        for (n, m) in g.examples() {
            v.push((id.clone(), n, m));
        }
    }
    v
}

pub fn example_model(id: &str) -> Result<Value> {
    let m = available(id)?;
    examples()
        .into_iter()
        .find(|e| e.0 == m.id)
        .map(|e| e.2)
        .ok_or_else(|| ApiError::NotYet(m.winqsb.into()))
}

/// Punto de entrada genérico por nombre de operación (lo usan la app, la CLI y el MCP).
///
/// `args.lang` ("es" o "en", también `language`) elige el idioma de lo que se
/// devuelve: informes, ejemplos, especificaciones, fuentes y mensajes de error.
pub fn call(op: &str, args: Value) -> Result<Value> {
    let lang = Lang::parse(args.get("lang").or_else(|| args.get("language")).and_then(|v| v.as_str()));
    match call_es(op, args, lang) {
        Ok(mut v) => {
            // los archivos y el BibTeX se devuelven tal cual
            match op {
                "import_text" | "export_text" | "bibtex" | "render" | "digest" | "export_report" | "export_xlsx" | "echo_text" | "xlsx_text" => {}
                // en la lista de ejemplos, «name» es la clave del ejemplo: no se traduce
                "examples" => {
                    for e in v.as_array_mut().into_iter().flatten() {
                        for k in ["model", "source"] {
                            if let Some(x) = e.get_mut(k) {
                                translate_value(x, lang);
                            }
                        }
                    }
                }
                _ => translate_value(&mut v, lang),
            }
            Ok(v)
        }
        Err(e) => Err(e.in_lang(lang)),
    }
}

fn call_es(op: &str, args: Value, lang: Lang) -> Result<Value> {
    let s = |k: &str| args.get(k).and_then(|v| v.as_str()).map(|s| s.to_string());
    let module = || s("module").unwrap_or_else(|| "lp".into());
    let need = |k: &str| args.get(k).cloned().ok_or_else(|| ApiError::BadInput(format!("falta {k}")));
    match op {
        "list_modules" => Ok(list_modules()),
        "describe_module" => describe_module(&module()),
        "guide" => {
            let id = module_id(&module()).unwrap_or_else(module);
            let g = iaqsb_core::guias::guia(&id, lang).ok_or_else(|| ApiError::BadInput(format!("el módulo «{id}» no tiene guía (hay de: {})", iaqsb_core::guias::modulos().join(", "))))?;
            let md = iaqsb_core::guias::markdown(&g);
            Ok(json!({"module": id, "guide": g, "markdown": md}))
        }
        "spec" => spec(&module()),
        "template" => template(&module(), &s("problem_type").unwrap_or_default(), args.get("dims").unwrap_or(&Value::Null)),
        "solve" => solve(&module(), need("model")?, args.get("options").cloned()),
        "graphic" => graphic(need("model")?),
        "import_text" => {
            let t = s("content").ok_or_else(|| ApiError::BadInput("falta content".into()))?;
            import_bytes(t.as_bytes())
        }
        "export_text" => {
            let bytes = export_bytes(&module(), need("model")?, s("layout").as_deref())?;
            Ok(json!({ "content": formats::decode_ansi(&bytes) }))
        }
        "examples" => {
            let only = s("module").and_then(|m| module_id(&m));
            Ok(Value::Array(
                examples()
                    .into_iter()
                    .filter(|e| only.as_deref().map_or(true, |o| o == e.0))
                    .map(|(m, n, model)| {
                        let src = example_source(&m, &n);
                        json!({"module": m, "name": n, "model": model, "source": src})
                    })
                    .collect(),
            ))
        }
        "bibliography" => Ok(bibliography()),
        "solver_info" => Ok(solver_info()),
        "convert" => convert_lp(need("model")?, &s("to").unwrap_or_else(|| "mps".into())),
        "diagnose" => diagnose_lp(need("model")?),
        // traduce un texto suelto al idioma pedido (útil para revisar el catálogo)
        "translate" => Ok(json!({ "text": i18n::translate(&s("text").unwrap_or_default(), lang) })),
        "compare" => {
            let vals = args.get("values").and_then(|v| v.as_array()).cloned();
            let vars = args.get("variants").and_then(|v| v.as_array()).cloned();
            compare(&module(), need("model")?, args.get("options").cloned(), s("vary"), vals, vars, s("metric"), args.get("full").and_then(|v| v.as_bool()).unwrap_or(false))
        }
        "comparable" => {
            let opts = args.get("options").cloned().unwrap_or(json!({}));
            Ok(json!(comparable_fields(&module(), &need("model")?, &opts).iter().map(|f| json!({"key": f.key, "label": f.label})).collect::<Vec<_>>()))
        }
        "bibtex" => Ok(json!({ "content": iaqsb_core::biblio::bibtex_all() })),
        // ajuste automático de los parámetros del motor (modelo de LP-ILP en cualquier forma)
        "tune" => {
            let em = lp_engine_model(need("model")?)?;
            let base: solver::Params = match args.get("options") {
                Some(o) => serde_json::from_value(o.get("engine").cloned().unwrap_or_else(|| o.clone())).unwrap_or_default(),
                None => solver::Params::default(),
            };
            let budget = args.get("budget").and_then(|v| v.as_f64()).unwrap_or(20.0);
            let repeats = args.get("repeats").and_then(|v| v.as_u64()).unwrap_or(2) as usize;
            let mut v = serde_json::to_value(solver::tune::tune(&em, &base, budget, repeats)).unwrap();
            // solo lo que cambia (los parámetros listos para pasarlos en options)
            let compact = |p: &mut Value| {
                if let Some(o) = p.as_object_mut() {
                    o.retain(|k, x| !(x.is_null() || (k == "log" && x == &json!(false)) || x.as_object().is_some_and(|m| m.is_empty())));
                }
            };
            for t in v["trials"].as_array_mut().into_iter().flatten() {
                compact(&mut t["params"]);
            }
            compact(&mut v["baseline"]["params"]);
            compact(&mut v["best"]);
            Ok(v)
        }
        // ¿qué pasa si…? barrido de un dato y paramétrico exacto de LP
        // devuelve el texto tal cual (para guardar un texto con save_export en la app)
        "echo_text" => Ok(json!({ "content": s("content").unwrap_or_default() })),
        // revisión del texto del lenguaje algebraico (para el editor): errores con línea y columna
        "check_model" => Ok(lp::algebra::check(&s("text").unwrap_or_default(), args.get("data"))),
        "sweep" => whatif::sweep(&module(), &need("model")?, args.get("options"), &s("path").ok_or_else(|| ApiError::BadInput("falta path".into()))?, &need("values")?, s("metric").as_deref()),
        "parametric" => {
            let num = |k: &str| args.get(k).and_then(|v| v.as_f64()).ok_or_else(|| ApiError::BadInput(format!("falta {k}")));
            whatif::parametric_lp(&lp_dense(need("model")?)?, &s("kind").unwrap_or_else(|| "cost".into()), &need("index")?, num("from")?, num("to")?, args.get("options"))
        }
        // informe para entregar: docx, pdf, html, xlsx o md (base64 o content)
        "export_report" => export_report(&args, lang),
        // modelo a Excel y de vuelta
        "export_xlsx" => {
            let b = excel::model_to_xlsx(&module(), need("model")?, lang)?;
            Ok(json!({ "base64": iaqsb_render::base64(&b), "filename": format!("{}.xlsx", sanitize(args["model"]["name"].as_str().unwrap_or("modelo"))) }))
        }
        "import_xlsx" => {
            let b = base64_decode(&s("base64").ok_or_else(|| ApiError::BadInput("falta base64".into()))?)?;
            excel::xlsx_to_model(&b, s("module").as_deref(), s("problem_type").as_deref())
        }
        // un libro de Excel cualquiera como texto (cada hoja en TSV), para que la IA lea los
        // datos de un ejercicio adjunto: { base64, max_chars? } → { text, sheets }
        // cadena jerárquica: ap → pmp (desagregación) y pmp → mrp (programa maestro del MRP)
        "link" => cadena::link(&args),
        "xlsx_text" => {
            let b = base64_decode(&s("base64").ok_or_else(|| ApiError::BadInput("falta base64".into()))?)?;
            let hojas = iaqsb_core::xlsx::read(&b).map_err(ApiError::BadInput)?;
            let tope = args.get("max_chars").and_then(|v| v.as_u64()).unwrap_or(60_000) as usize;
            let mut out = String::new();
            for h in &hojas {
                out.push_str(&format!("### {}\n", h.name));
                for fila in h.trimmed() {
                    out.push_str(&fila.iter().map(|c| c.text()).collect::<Vec<_>>().join("\t"));
                    out.push('\n');
                }
                out.push('\n');
                if out.len() > tope {
                    let mut corte = tope;
                    while !out.is_char_boundary(corte) {
                        corte -= 1;
                    }
                    out.truncate(corte);
                    out.push_str("\n[…]\n");
                    break;
                }
            }
            Ok(json!({ "text": out, "sheets": hojas.iter().map(|h| h.name.clone()).collect::<Vec<_>>() }))
        }
        // tabla pegada desde Excel (texto con tabuladores) como filas de celdas
        "parse_pasted" => {
            let rows = iaqsb_core::xlsx::parse_pasted(&s("text").unwrap_or_default());
            Ok(json!(rows.iter().map(|r| r.iter().map(|c| match c {
                iaqsb_core::xlsx::Cell::Num(x) => json!(x),
                iaqsb_core::xlsx::Cell::Empty => Value::Null,
                other => json!(other.text()),
            }).collect::<Vec<_>>()).collect::<Vec<_>>()))
        }
        // resumen en Markdown de un informe (para pegarlo en una IA): detail = summary | steps | full
        "digest" => {
            let report = need("report")?;
            let detail = match s("detail").as_deref() {
                Some("full") => iaqsb_render::digest::Detail::Full,
                Some("steps") => iaqsb_render::digest::Detail::Steps,
                _ => iaqsb_render::digest::Detail::Summary,
            };
            let max_rows = args.get("max_rows").and_then(|v| v.as_u64()).unwrap_or(40) as usize;
            let md = iaqsb_render::digest::markdown(&report, &iaqsb_render::digest::DigestOpts { detail, max_rows, report_id: None });
            Ok(json!({ "markdown": i18n::translate_markdown(&md, lang) }))
        }
        // gráfica de un bloque del informe, para descargarla: SVG o PNG en base64
        "render" => {
            let block = need("block")?;
            let svg = iaqsb_render::svg(&block).ok_or_else(|| ApiError::BadInput("el bloque no es una gráfica".into()))?;
            let svg = i18n::translate_svg(&svg, lang);
            match s("format").as_deref() {
                Some("png") => {
                    let scale = args.get("scale").and_then(|v| v.as_f64()).unwrap_or(2.0).clamp(0.5, 4.0) as f32;
                    let png = iaqsb_render::png(&svg, scale).map_err(ApiError::BadInput)?;
                    Ok(json!({ "png": iaqsb_render::base64(&png) }))
                }
                _ => Ok(json!({ "svg": i18n::translate_svg(&svg, lang) })),
            }
        }
        other => Err(ApiError::BadInput(format!("operación desconocida: {other}"))),
    }
}

/// Modelo de LP-ILP como hoja (coeficientes densos), aunque venga con `terms` o en texto MPS/LP.
pub fn lp_dense(model: Value) -> Result<Value> {
    if model.get("format").is_some() {
        return Ok(convert_lp(model, "json")?["model"].clone());
    }
    let mut m: lp::LpModel = from_value(model, "modelo")?;
    m.normalize()?;
    Ok(serde_json::to_value(m).unwrap())
}

/// El modelo como libro de Excel (en el idioma pedido).
pub fn model_to_xlsx(module: &str, model: Value, lang: Option<&str>) -> Result<Vec<u8>> {
    excel::model_to_xlsx(module, model, Lang::parse(lang))
}

/// Un libro de Excel como modelo ({module, layout, model}).
pub fn xlsx_to_model(bytes: &[u8], module: Option<&str>, problem_type: Option<&str>) -> Result<Value> {
    excel::xlsx_to_model(bytes, module, problem_type)
}

/// Adivina el módulo de un modelo JSON sin envoltorio {module, model}.
pub fn guess_module(model: &Value) -> Option<String> {
    let pt = model.get("problem_type")?.as_str()?;
    registry()
        .iter()
        .find(|g| g.spec().problem_types.iter().any(|p| p.id == pt))
        .map(|g| g.spec().id)
}

/// Identificador interno de un módulo a partir de su id o su nombre en WinQSB.
pub fn module_id(id: &str) -> Option<String> {
    module(id).ok().map(|m| m.id.to_string())
}
