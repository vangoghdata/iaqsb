//! Herramientas del servidor MCP, pensadas para que las use un modelo de IA:
//! respuestas cortas en Markdown, con lo grande disponible a pedido.

use iaqsb_api as api;
use iaqsb_render as render;
use render::digest::{self, Detail, DigestOpts};
use api::core::i18n::{self, Lang};
use serde_json::{json, Value};
use std::sync::Mutex;

/// Idioma de la llamada en curso (argumento language o variable IAQSB_LANG).
static LANG: Mutex<Lang> = Mutex::new(Lang::Es);

fn lang() -> Lang {
    *LANG.lock().unwrap()
}

/// Texto en Markdown en el idioma de la llamada.
fn md(t: impl Into<String>) -> Value {
    text(i18n::translate_markdown(&t.into(), lang()))
}

/// Datos en el idioma de la llamada.
fn tv(mut v: Value) -> Value {
    i18n::translate_value(&mut v, lang());
    v
}

pub const INSTRUCTIONS: &str = "IAQSB resuelve problemas de investigación de operaciones y logística: los 19 módulos de WinQSB \
(lp, net, pert, gp, qp, nlp, dp, ap, da, mkp, its, fc, qa, qss, job, mrp, fll, qcc, asa) y seis de logística (vrp ruteo, modo de transporte, \
inv gestión de inventarios, fin finanzas del inventario, alm almacenes, kpi indicadores) y tres de planificación y pronóstico: \
pmp programa maestro (básico y ATO, disponible para promesa, solicitudes de pedidos, RCCP y PMP óptimo por programación entera), \
mrp2 MRP II por matrices (Gozinto) y st series de tiempo (exploración: ACF, PACF, Ljung-Box, descomposición, Box-Cox; pronóstico: \
ARIMA, ETS, TBATS, Theta, Croston automáticos o con órdenes dados, y comparación de modelos en un periodo de prueba). En st el motor \
de pronóstico es el mejor disponible en el equipo (options.forecast_engine = auto: R con el paquete forecast, si no Python con statsforecast, \
si no el nativo en Rust); el informe dice cuál usó.\n\
Si el usuario te da una foto, un PDF o una tabla de un ejercicio: lee el enunciado, di qué datos extrajiste (para que los revise), \
elige el módulo, plantea el modelo con describe_module y resuélvelo con solve; nunca inventes datos que no se ven. \
Los archivos de WinQSB se leen con import_winqsb_file y los libros de Excel con import_excel.\n\
Flujo: 1) describe_module(module, problem_type) da el esquema del modelo, las opciones y un ejemplo del tipo de problema; \
2) solve(module, model, options); 3) explica el resultado.\n\
Las respuestas vienen en Markdown y son cortas a propósito: resultado, métricas, tablas principales y las gráficas descritas con sus números. \
Cada informe trae un report_id: con get_report_part(report_id, section | step | path) se pide lo que falte (el paso a paso, una tabla larga, \
data.lp_model con el modelo lineal equivalente). Con images=true se reciben las gráficas en PNG (con options.steps=true, también las de cada iteración). \
Para comparar métodos (NNH, CIH, 2-opt y óptimo; soluciones iniciales del transporte; métodos de lote, de pronóstico o de despacho) usa compare_methods.\n\
Motor: HiGHS (incluido) resuelve LP, MIP y QP de cualquier tamaño. En LP-ILP (module lp) el modelo puede ser el JSON de IAQSB (con coefs densos o terms dispersos) \
o un texto en formato LP de CPLEX o MPS: model = {\"format\": \"lp\", \"text\": \"Maximize\\n obj: 3 x + 5 y\\nSubject To\\n c1: x <= 4\\nEnd\"}. \
Parámetros del motor (como en Gurobi): options.time_limit, mip_gap, threads, presolve, method, seed, node_limit, solver (highs, gurobi, cplex, scip, cbc, glpk si están instalados; ver solver_info), log; \
en los demás módulos van en options.engine. Si un modelo no es factible, el informe trae el diagnóstico (IIS y relajación de factibilidad); también diagnose_infeasibility. \
convert_model pasa un modelo a LP, MPS, WinQSB (.LPP) o al lenguaje algebraico.\n\
Lenguaje algebraico (lo más cómodo para escribir modelos grandes): model = {\"format\": \"iaqsb\", \"text\": \"set I = {A, B}; set J = 1..3; param c{I, J} = [[4, 6, 9], [5, 3, 7]]; \
param s{I} = [30, 40]; param d{J} = [20, 25, 15]; var x{I, J} >= 0; minimize costo: sum{i in I, j in J} c[i,j]*x[i,j]; \
subject to of{i in I}: sum{j in J} x[i,j] <= s[i]; de{j in J}: sum{i in I} x[i,j] >= d[j];\"} (palabras clave también en español y portugués: conjunto, parametro, variable, minimizar, sujeto a, suma, en). \
Variables: >= a, <= b, integer, binary, free, semicontinuous. Índices con condición: sum{i in I, j in J: c[i,j] > 0}; conjuntos de pares: set A = {(a,b), (b,c)}; rangos 1..T. \
Restricciones generales (se reformulan solas con big-M a partir de las cotas): abs(e), max(e1, e2), min(…), pwl(x; (0,0), (10,50), (20,80)), and(b1, b2), or(…), not(b), \
indicadoras «y[i] = 1 -> e <= b» o «si y[i] = 1 entonces …», disyunciones «e1 <= b1 o e2 <= b2», atmost(k, …), atleast, exactly, implies(a, b); objetivo cuadrático; varios objetivos por prioridad. \
check_model revisa el texto (errores con línea y columna). El informe trae las variables por índices (tables). \
Más herramientas: what_if (barre un dato, en paralelo), parametric_analysis (objetivo vs un costo o lado derecho, con los puntos de quiebre exactos), \
tune_parameters (ajuste automático del motor), export_report (informe en Word, PDF, HTML, Excel o Markdown para entregar), export_excel e import_excel.\n\
Cita las fuentes del informe (APA 7) y advierte las que digan «por verificar». Las cotas infinitas se escriben \"M\" o \"-M\". Responde en el idioma del usuario.\n\
Idioma: todas las herramientas aceptan language = \"es\" (por defecto), \"en\" (inglés) o \"pt\" (portugués) para recibir informes, ejemplos y fuentes en ese idioma. \
Language: every tool accepts language = \"en\" to get reports, examples and sources in English (or \"pt\" for Portuguese). \
Idioma: todas as ferramentas aceitam language = \"pt\" para receber relatórios, exemplos e fontes em português.";

fn module_prop() -> Value {
    json!({"type": "string", "description": "Módulo: lp, net, pert, gp, qp, nlp, dp, ap, pmp, da, mkp, its, fc, st, qa, qss, job, mrp, mrp2, fll, qcc, asa, vrp, modo, inv, fin, alm o kpi (también vale el nombre de WinQSB, p. ej. \"LP-ILP\")"})
}

fn view_props() -> Value {
    json!({
        "detail": {"type": "string", "enum": ["summary", "steps", "full", "json"], "description": "summary (por defecto): resultado y tablas principales; steps (por defecto si options.steps=true): además el paso a paso; full: todas las secciones; json: el informe completo en JSON (muy largo)"},
        "images": {"type": "boolean", "description": "true = agrega las gráficas en PNG"},
        "max_images": {"type": "integer", "minimum": 1, "maximum": 40, "description": "Máximo de imágenes (por defecto 12)"}
    })
}

fn merge(mut a: Value, b: Value) -> Value {
    if let (Some(x), Some(y)) = (a.as_object_mut(), b.as_object()) {
        for (k, v) in y {
            x.insert(k.clone(), v.clone());
        }
    }
    a
}

pub fn tools() -> Value {
    let mut t = tools_es();
    // todas aceptan el idioma de la respuesta
    for tool in t.as_array_mut().into_iter().flatten() {
        if let Some(props) = tool.pointer_mut("/inputSchema/properties").and_then(|p| p.as_object_mut()) {
            props.insert("language".into(), json!({"type": "string", "enum": ["es", "en", "pt"], "description": "Idioma de la respuesta / response language / idioma da resposta: es (por defecto), en o pt"}));
        }
    }
    t
}

fn tools_es() -> Value {
    let ro = json!({"readOnlyHint": true, "idempotentHint": true, "openWorldHint": false});
    json!([
        {
            "name": "list_modules",
            "title": "Listar módulos",
            "description": "Lista los módulos con lo que resuelve cada uno. Úsala para elegir el módulo de un problema.",
            "inputSchema": {"type": "object", "properties": {}},
            "annotations": ro
        },
        {
            "name": "describe_module",
            "title": "Describir un módulo",
            "description": "Esquema JSON del modelo y de las opciones de un módulo, sus tipos de problema y un ejemplo listo para usar (del tipo de problema pedido). Llámala antes de armar un modelo.",
            "inputSchema": {"type": "object", "properties": {
                "module": module_prop(),
                "problem_type": {"type": "string", "description": "Tipo de problema (p. ej. tsp, transportation, eoq); elige el ejemplo que se devuelve"}
            }, "required": ["module"]},
            "annotations": ro
        },
        {
            "name": "solve",
            "title": "Resolver",
            "description": "Resuelve un modelo y devuelve un informe en Markdown: resultado, métricas, tablas, gráficas descritas con sus números, motor usado (tiempo, cota, brecha), fuentes y un report_id para pedir más con get_report_part. options.steps=true agrega el paso a paso (y detail=full lo muestra entero). images=true agrega las gráficas en PNG. En module lp el modelo puede ser texto LP, MPS o del lenguaje algebraico de IAQSB, de cualquier tamaño ({format: lp|mps|iaqsb, text}); los parámetros del motor (time_limit, mip_gap, threads, solver…) van en options (u options.engine en los demás módulos).",
            "inputSchema": {"type": "object", "properties": merge(json!({
                "module": module_prop(),
                "model": {"type": "object", "description": "Modelo según el esquema de describe_module"},
                "options": {"type": "object", "description": "Opciones del módulo (ver describe_module); steps=true incluye el paso a paso"}
            }), view_props()), "required": ["module", "model"]},
            "annotations": ro
        },
        {
            "name": "compare_methods",
            "title": "Comparar métodos",
            "description": "Resuelve el mismo modelo con varios métodos y devuelve la tabla comparativa (métricas, mejor, empates y diferencia con la mejor) y la solución resumida de cada método. Sin vary compara todos los valores de la opción de método del tipo de problema. Con values se eligen algunos; con variants, combinaciones libres de opciones (cada una con \"label\").",
            "inputSchema": {"type": "object", "properties": merge(json!({
                "module": module_prop(),
                "model": {"type": "object"},
                "options": {"type": "object", "description": "Opciones comunes a todas las variantes"},
                "vary": {"type": "string", "description": "Opción que cambia (vacío = la de método)"},
                "values": {"type": "array", "description": "Valores de esa opción a probar (vacío = todos)"},
                "variants": {"type": "array", "description": "En lugar de vary: objetos de opciones, cada uno con \"label\" opcional"},
                "metric": {"type": "string", "description": "Métrica que decide la mejor (p. ej. MAD); vacío = automática"}
            }), view_props()), "required": ["module", "model"]},
            "annotations": ro
        },
        {
            "name": "solver_info",
            "title": "Motores de optimización",
            "description": "Motores disponibles en este equipo (HiGHS siempre; Gurobi, CPLEX, SCIP, CBC y GLPK si están instalados) y el esquema de los parámetros del motor (tiempo, brecha, hilos, método…).",
            "inputSchema": {"type": "object", "properties": {}},
            "annotations": ro
        },
        {
            "name": "diagnose_infeasibility",
            "title": "Diagnosticar infactibilidad",
            "description": "Para un modelo de LP-ILP no factible: el subsistema irreducible infactible (IIS: restricciones y cotas que no se pueden cumplir a la vez) y la relajación de factibilidad (el menor cambio de los lados derechos que lo vuelve factible). Acepta el JSON de IAQSB o {format: lp|mps, text}.",
            "inputSchema": {"type": "object", "properties": {"model": {"type": "object"}}, "required": ["model"]},
            "annotations": ro
        },
        {
            "name": "convert_model",
            "title": "Convertir un modelo",
            "description": "Convierte un modelo de LP-ILP (JSON de IAQSB o {format: lp|mps, text}) a formato LP de CPLEX, MPS, WinQSB (.LPP, algebraico) o JSON de IAQSB. Con path lo guarda en el equipo.",
            "inputSchema": {"type": "object", "properties": {
                "model": {"type": "object"},
                "to": {"type": "string", "enum": ["lp", "mps", "lpp", "json", "iaqsb"]},
                "path": {"type": "string", "description": "Ruta donde guardar (opcional)"}
            }, "required": ["model", "to"]},
            "annotations": {"readOnlyHint": false, "destructiveHint": true, "openWorldHint": false}
        },
        {
            "name": "check_model",
            "title": "Revisar un modelo algebraico",
            "description": "Revisa un modelo escrito en el lenguaje algebraico de IAQSB sin resolverlo: errores con línea y columna, o el tamaño (variables, enteras, restricciones, no ceros, conjuntos, familias de variables) y las reformulaciones de las restricciones generales.",
            "inputSchema": {"type": "object", "properties": {"text": {"type": "string"}, "data": {"type": "object", "description": "Datos que reemplazan los del texto (opcional)"}}, "required": ["text"]},
            "annotations": ro
        },
        {
            "name": "link_models",
            "title": "Cadena jerárquica: plan agregado → programa maestro → MRP",
            "description": "Pasa el resultado de un nivel de la planificación jerárquica al siguiente y devuelve los modelos listos para resolver. from=ap, to=pmp: resuelve el plan agregado (model de module ap) y desagrega su producción en productos (products: [{name, share, lot_size, on_hand, safety_stock}]) como programas maestros básicos. from=pmp, to=mrp: resuelve el programa maestro (model de module pmp) y pone su PMP como programa maestro del artículo item (por defecto el primero) en el modelo MRP target. Luego resuelve cada modelo devuelto con solve.",
            "inputSchema": {"type": "object", "properties": {
                "from": {"type": "string", "enum": ["ap", "pmp"]},
                "to": {"type": "string", "enum": ["pmp", "mrp"]},
                "model": {"type": "object", "description": "Modelo del nivel de origen"},
                "options": {"type": "object"},
                "products": {"type": "array", "description": "ap → pmp: productos de la familia con su participación (share) y, si se quiere, lot_size, on_hand y safety_stock"},
                "target": {"type": "object", "description": "pmp → mrp: modelo MRP (artículos, lista de materiales, inventarios) que recibe el PMP"},
                "item": {"type": "string", "description": "pmp → mrp: id del artículo del MRP que corresponde al producto del PMP"}
            }, "required": ["from", "to", "model"]},
            "annotations": ro
        },
        {
            "name": "what_if",
            "title": "¿Qué pasa si…?",
            "description": "Resuelve el modelo con varios valores de un dato (en paralelo) y devuelve, para cada valor, el estado, la métrica (en LP-ILP, el objetivo) y el titular; en LP-ILP, también dónde cambia la solución. path: puntero al dato (p. ej. /constraints/2/rhs, /variables/0/cost o demand). values: lista o {from, to, steps}.",
            "inputSchema": {"type": "object", "properties": {
                "module": module_prop(),
                "model": {"type": "object"},
                "options": {"type": "object"},
                "path": {"type": "string"},
                "values": {"description": "Lista de números o {from, to, steps}"},
                "metric": {"type": "string", "description": "Etiqueta de la métrica a seguir (por defecto, el objetivo o la primera métrica)"}
            }, "required": ["module", "model", "path", "values"]},
            "annotations": ro
        },
        {
            "name": "parametric_analysis",
            "title": "Análisis paramétrico",
            "description": "LP-ILP: el objetivo como función de un costo c(j) (kind=cost) o de un lado derecho b(i) (kind=rhs) en [from, to], por tramos lineales con los puntos exactos donde cambia la base óptima (a partir de los rangos de sensibilidad). index: número o nombre de la variable o restricción.",
            "inputSchema": {"type": "object", "properties": {
                "model": {"type": "object"},
                "kind": {"type": "string", "enum": ["cost", "rhs"]},
                "index": {"type": ["integer", "string"]},
                "from": {"type": "number"},
                "to": {"type": "number"}
            }, "required": ["model", "kind", "index", "from", "to"]},
            "annotations": ro
        },
        {
            "name": "tune_parameters",
            "title": "Ajustar los parámetros del motor",
            "description": "Ajuste automático (como el tuning tool de Gurobi) para un modelo de LP-ILP: prueba combinaciones de parámetros de HiGHS (presolve, método, heurísticas, simetría, hilos, semilla), mide cada una y recomienda la más rápida con el mismo resultado. Devuelve los parámetros listos para pasar en options.",
            "inputSchema": {"type": "object", "properties": {
                "model": {"type": "object"},
                "options": {"type": "object"},
                "budget": {"type": "number", "description": "Segundos en total (por defecto 20)"}
            }, "required": ["model"]},
            "annotations": ro
        },
        {
            "name": "export_report",
            "title": "Exportar el informe",
            "description": "Informe para entregar (portada, planteamiento, tablas y figuras numeradas al estilo APA, referencias) en docx (Word), pdf, html, xlsx (Excel) o md. Usa report_id de un solve anterior o resuelve module + model. Guarda el archivo en path (si no, en la carpeta temporal) y devuelve la ruta.",
            "inputSchema": {"type": "object", "properties": {
                "report_id": {"type": "string"},
                "module": module_prop(),
                "model": {"type": "object"},
                "options": {"type": "object"},
                "format": {"type": "string", "enum": ["docx", "pdf", "html", "xlsx", "md"]},
                "steps": {"type": "boolean", "description": "Incluir el paso a paso"},
                "author": {"type": "string"}, "course": {"type": "string"}, "institution": {"type": "string"},
                "decimal_comma": {"type": "boolean"},
                "path": {"type": "string", "description": "Ruta del archivo a guardar"}
            }, "required": ["format"]},
            "annotations": {"readOnlyHint": false, "destructiveHint": true, "openWorldHint": false}
        },
        {
            "name": "export_excel",
            "title": "Modelo a Excel",
            "description": "Guarda el modelo de cualquier módulo como libro de Excel (.xlsx): una hoja por tabla del editor (como en la aplicación) y una hoja IAQSB con el modelo completo. Se puede editar en Excel y volver a abrir con import_excel.",
            "inputSchema": {"type": "object", "properties": {"module": module_prop(), "model": {"type": "object"}, "path": {"type": "string"}}, "required": ["module", "model", "path"]},
            "annotations": {"readOnlyHint": false, "destructiveHint": true, "openWorldHint": false}
        },
        {
            "name": "import_excel",
            "title": "Excel a modelo",
            "description": "Lee un libro de Excel (.xlsx) como modelo: uno hecho por IAQSB (trae el módulo) o uno propio indicando module (y problem_type). En LP-ILP, la hoja va como en WinQSB: variables en la primera fila, el objetivo en la segunda (Max o Min) y cada restricción con su sentido (<=, >=, =) y su lado derecho.",
            "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}, "module": {"type": "string"}, "problem_type": {"type": "string"}}, "required": ["path"]},
            "annotations": ro
        },
        {
            "name": "get_report_part",
            "title": "Ver una parte del informe",
            "description": "Devuelve una parte de un informe ya calculado: una sección (section), un paso del paso a paso (step: número o parte del título), o un valor de los datos (path, p. ej. data.lp_model o data.tour). Con images=true, las gráficas de esa parte en PNG.",
            "inputSchema": {"type": "object", "properties": {
                "report_id": {"type": "string"},
                "section": {"type": "string", "description": "id o título de la sección"},
                "step": {"type": ["string", "integer"], "description": "Número del paso (desde 1) o parte de su título"},
                "path": {"type": "string", "description": "Ruta con puntos dentro del informe, p. ej. data.lp_model"},
                "format": {"type": "string", "enum": ["markdown", "json"], "description": "markdown (por defecto) o json"},
                "images": {"type": "boolean"},
                "max_images": {"type": "integer", "minimum": 1, "maximum": 40}
            }, "required": ["report_id"]},
            "annotations": ro
        },
        {
            "name": "graphic_method",
            "title": "Método gráfico",
            "description": "Para un LP de dos variables: región factible (vértices), rectas de las restricciones y punto óptimo.",
            "inputSchema": {"type": "object", "properties": {"model": {"type": "object"}}, "required": ["model"]},
            "annotations": ro
        },
        {
            "name": "import_winqsb_file",
            "title": "Abrir archivo de WinQSB",
            "description": "Lee un archivo de cualquier módulo de WinQSB (.LPP, .NET, .CPM, .ITS, .FCC, .QAA, .JOB…) desde una ruta o su contenido y devuelve el modelo en JSON de IAQSB.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string", "description": "Ruta del archivo en el equipo"},
                "content": {"type": "string", "description": "Contenido del archivo, si no se da la ruta"}
            }},
            "annotations": {"readOnlyHint": true, "openWorldHint": false}
        },
        {
            "name": "export_winqsb_file",
            "title": "Guardar en formato WinQSB",
            "description": "Convierte un modelo al formato de archivo de WinQSB del módulo. Con path lo guarda; si no, devuelve el texto.",
            "inputSchema": {"type": "object", "properties": {
                "module": module_prop(),
                "model": {"type": "object"},
                "layout": {"type": "string", "enum": ["matrix", "normal"], "description": "Solo LP-ILP: matriz (por defecto) o modelo normal"},
                "path": {"type": "string", "description": "Ruta donde guardar (opcional)"}
            }, "required": ["module", "model"]},
            "annotations": {"readOnlyHint": false, "destructiveHint": true, "openWorldHint": false}
        },
        {
            "name": "get_bibliography",
            "title": "Bibliografía",
            "description": "Bibliografía de IAQSB en APA 7 (con DOI y si está verificada o por verificar); con bibtex=true, en BibTeX.",
            "inputSchema": {"type": "object", "properties": {"bibtex": {"type": "boolean"}}},
            "annotations": ro
        },
        {
            "name": "get_example",
            "title": "Obtener un ejemplo",
            "description": "Sin nombre: lista los ejemplos (módulo, nombre y fuente). Con nombre: el modelo listo para solve y su fuente.",
            "inputSchema": {"type": "object", "properties": {"name": {"type": "string"}, "module": {"type": "string", "description": "Filtra la lista por módulo"}}},
            "annotations": ro
        }
    ])
}

// ---------------------------------------------------------------- informes guardados

static REPORTS: Mutex<(u64, Vec<(String, Value)>)> = Mutex::new((0, Vec::new()));

fn keep(r: &Value) -> String {
    let mut g = REPORTS.lock().unwrap();
    g.0 += 1;
    let id = format!("r{}", g.0);
    g.1.push((id.clone(), r.clone()));
    if g.1.len() > 40 {
        g.1.remove(0);
    }
    id
}

fn fetch(id: &str) -> Result<Value, String> {
    let g = REPORTS.lock().unwrap();
    g.1.iter()
        .find(|x| x.0 == id)
        .map(|x| x.1.clone())
        .ok_or_else(|| format!("no hay ningún informe {id} (se guardan los últimos 40 de esta sesión); vuelve a llamar a solve"))
}

// ---------------------------------------------------------------- respuesta

/// Lo que devuelve una herramienta: bloques de contenido MCP.
pub struct Reply {
    pub content: Vec<Value>,
    pub structured: Option<Value>,
}

fn text(t: impl Into<String>) -> Value {
    json!({"type": "text", "text": t.into()})
}

/// JSON compacto (sin sangrías: gasta menos tokens).
fn j(v: &Value) -> String {
    serde_json::to_string(v).unwrap_or_default()
}

fn images(report: &Value, args: &Value, out: &mut Vec<Value>) {
    if args.get("images").and_then(|x| x.as_bool()) != Some(true) {
        return;
    }
    let max = args.get("max_images").and_then(|x| x.as_u64()).unwrap_or(12).clamp(1, 40) as usize;
    let (figs, left) = render::figures(report, render::Pick { steps: true, max });
    if figs.is_empty() {
        out.push(md("Esta parte no tiene gráficas."));
        return;
    }
    for (k, f) in figs.iter().enumerate() {
        match render::png(&i18n::translate_svg(&f.svg, lang()), 1.5) {
            Ok(png) => {
                let what = if f.title == f.place { f.title.clone() } else { format!("{} — {}", f.title, f.place) };
                out.push(md(format!("Figura {}: {what}", k + 1)));
                out.push(json!({"type": "image", "data": render::base64(&png), "mimeType": "image/png"}));
            }
            Err(e) => out.push(md(format!("Figura {}: no se pudo dibujar ({e})", k + 1))),
        }
    }
    if left > 0 {
        out.push(md(format!("Hay {left} gráficas más; súbelas con max_images o pídelas por sección o paso con get_report_part.")));
    }
}

fn report_reply(r: Value, args: &Value) -> Reply {
    let id = keep(&r);
    let steps_on = args.pointer("/options/steps").and_then(|x| x.as_bool()) == Some(true);
    let detail = args.get("detail").and_then(|x| x.as_str()).unwrap_or(if steps_on { "steps" } else { "summary" });
    let mut out = vec![];
    if detail == "json" {
        out.push(text(format!("report_id: {id}\n{}", j(&r))));
    } else {
        let d = match detail {
            "full" => Detail::Full,
            "steps" => Detail::Steps,
            _ => Detail::Summary,
        };
        let o = DigestOpts { detail: d, max_rows: if d == Detail::Full { 60 } else { 25 }, report_id: Some(id.clone()) };
        out.push(md(digest::markdown(&r, &o)));
    }
    images(&r, args, &mut out);
    let (data, _) = digest::compact(r.get("data").unwrap_or(&Value::Null), 1200, "data");
    let structured = json!({
        "report_id": id,
        "status": r.get("status"),
        "headline": r.get("headline"),
        "objective_value": r.get("objective_value"),
        "data": data
    });
    Reply { content: out, structured: Some(structured) }
}

fn plain(v: Value) -> Reply {
    let v = tv(v);
    Reply { content: vec![text(j(&v))], structured: if v.is_object() { Some(v) } else { None } }
}

pub fn call(name: &str, args: &Value) -> Result<Reply, String> {
    let l = Lang::parse(args.get("language").or_else(|| args.get("lang")).and_then(|v| v.as_str()).map(str::to_string).or_else(|| std::env::var("IAQSB_LANG").ok()).as_deref());
    *LANG.lock().unwrap() = l;
    call_es(name, args).map_err(|e| i18n::translate(&e, l))
}

fn call_es(name: &str, args: &Value) -> Result<Reply, String> {
    let s = |k: &str| args.get(k).and_then(|v| v.as_str()).filter(|x| !x.is_empty());
    let need = |k: &str| args.get(k).cloned().ok_or_else(|| format!("falta el argumento {k}"));
    let module = s("module").unwrap_or("lp");
    match name {
        "list_modules" => {
            let v = tv(api::list_modules());
            let mut t = String::from("| módulo | WinQSB | nombre | resuelve | archivos |\n| :-- | :-- | :-- | :-- | :-- |\n");
            for m in v["modules"].as_array().into_iter().flatten() {
                t += &format!(
                    "| {} | {} | {} | {} | .{} |\n",
                    m["id"].as_str().unwrap_or(""),
                    m["winqsb"].as_str().unwrap_or(""),
                    m["name"].as_str().unwrap_or(""),
                    m["solves"].as_str().unwrap_or("").replace('|', "/"),
                    m["file_ext"].as_str().unwrap_or("")
                );
            }
            Ok(Reply { content: vec![md(t)], structured: Some(v) })
        }
        "describe_module" => {
            let mut d = tv(api::describe_module(module).map_err(|e| e.to_string())?);
            if let Some(exs) = d.get("examples").and_then(|e| e.as_array()).cloned() {
                let want = s("problem_type");
                let pick = exs
                    .iter()
                    .find(|e| want.is_some() && e.pointer("/model/problem_type").and_then(|p| p.as_str()) == want)
                    .or_else(|| exs.first())
                    .cloned();
                let list: Vec<Value> = exs
                    .iter()
                    .map(|e| json!({"name": e["name"], "problem_type": e.pointer("/model/problem_type"), "title": e.pointer("/model/name")}))
                    .collect();
                let obj = d.as_object_mut().unwrap();
                obj.remove("examples");
                obj.insert("example".into(), pick.unwrap_or(Value::Null));
                obj.insert("other_examples".into(), json!(list));
                obj.insert("hint".into(), json!("get_example(name) da cualquier otro ejemplo completo."));
            }
            Ok(Reply { content: vec![text(j(&tv(d)))], structured: None })
        }
        "solve" => {
            let r = tv(api::solve(module, need("model")?, args.get("options").cloned()).map_err(|e| e.to_string())?);
            Ok(report_reply(r, args))
        }
        "compare_methods" => {
            let mut a = args.clone();
            a["module"] = json!(module);
            a["full"] = json!(false);
            a["lang"] = json!(lang().code());
            let r = api::call("compare", a).map_err(|e| e.to_string())?;
            Ok(report_reply(r, args))
        }
        "get_report_part" => {
            let r = fetch(s("report_id").ok_or("falta report_id")?)?;
            let step = args.get("step").map(|v| match v {
                Value::String(x) => x.clone(),
                other => other.to_string(),
            });
            let part = digest::part(&r, s("section"), step.as_deref(), s("path"))?;
            let mut out = vec![];
            if s("format") == Some("json") || s("path").is_some() {
                out.push(text(j(&part)));
            } else {
                out.push(md(digest::part_markdown(&part, 200)));
            }
            // las gráficas de la parte
            let wrapped = if part.get("type").and_then(|t| t.as_str()) == Some("step") {
                json!({"sections": [{"id": "parte", "title": "Paso a paso", "blocks": [{"type": "steps", "steps": [part]}]}]})
            } else if part.get("blocks").is_some() {
                json!({"sections": [part]})
            } else {
                json!({"sections": [{"id": "parte", "title": "Paso a paso", "blocks": [part]}]})
            };
            images(&wrapped, args, &mut out);
            Ok(Reply { content: out, structured: None })
        }
        "graphic_method" => api::graphic(need("model")?).map(plain).map_err(|e| e.to_string()),
        "solver_info" => {
            let v = api::solver_info();
            let mut t = String::from("| solver | motor | cómo | resuelve | nota |\n| :-- | :-- | :-- | :-- | :-- |\n");
            for x in v["solvers"].as_array().into_iter().flatten() {
                t += &format!("| {} | {} | {} | {} | {} |\n", x["id"].as_str().unwrap_or(""), x["name"].as_str().unwrap_or(""), x["via"].as_str().unwrap_or(""), x["problems"].as_str().unwrap_or(""), x["note"].as_str().unwrap_or(""));
            }
            t += "\nParámetros: solver, time_limit, mip_gap, mip_abs_gap, threads, presolve, method (auto, dual_simplex, primal_simplex, barrier, pdlp), seed, node_limit, solution_limit, feasibility_tol, integrality_tol, log, start. En LP-ILP van en options; en los demás módulos, en options.engine.";
            Ok(Reply { content: vec![md(t)], structured: Some(tv(v)) })
        }
        "diagnose_infeasibility" => api::diagnose_lp(need("model")?).map(plain).map_err(|e| e.to_string()),
        "check_model" => {
            let v = api::call("check_model", json!({"text": s("text").unwrap_or(""), "data": args.get("data"), "lang": lang().code()})).map_err(|e| e.to_string())?;
            Ok(plain(v))
        }
        "link_models" => {
            let mut a = args.clone();
            a["lang"] = json!(lang().code());
            let v = api::call("link", a).map_err(|e| e.to_string())?;
            let mut t = format!("{}
", v["summary"].as_str().unwrap_or(""));
            for m in v["models"].as_array().into_iter().flatten() {
                t += &format!("
- {} · «{}»", m["module"].as_str().unwrap_or(""), m["model"]["name"].as_str().unwrap_or(""));
            }
            t += "

Los modelos completos están en el resultado estructurado (models[].model): resuélvelos con solve.";
            Ok(Reply { content: vec![md(t)], structured: Some(v) })
        }
        "what_if" => {
            let mut a = json!({"module": module, "model": need("model")?, "path": s("path").ok_or("falta path")?, "values": need("values")?, "lang": lang().code()});
            if let Some(o) = args.get("options") {
                a["options"] = o.clone();
            }
            if let Some(m) = s("metric") {
                a["metric"] = json!(m);
            }
            let v = api::call("sweep", a).map_err(|e| e.to_string())?;
            let mut t = format!("¿Qué pasa si cambia {}? (valor actual: {})\n\n| valor | estado | métrica | titular |\n| --: | :-- | --: | :-- |\n", v["path"].as_str().unwrap_or(""), v["base_value"]);
            for p in v["points"].as_array().into_iter().flatten() {
                t += &format!("| {} | {} | {} | {} |\n", p["value"], p["status"].as_str().unwrap_or(""), p["metric"], p["headline"].as_str().unwrap_or("").replace('|', "/"));
            }
            for c in v["solution_changes"].as_array().into_iter().flatten() {
                t += &format!("\nEntre {} y {} cambian las variables positivas: {} → {}.", c["between"][0], c["between"][1], c["before"], c["after"]);
            }
            Ok(Reply { content: vec![md(t)], structured: Some(v) })
        }
        "parametric_analysis" => {
            let v = api::call("parametric", json!({"model": need("model")?, "kind": s("kind").unwrap_or("cost"), "index": need("index")?, "from": need("from")?, "to": need("to")?, "lang": lang().code()})).map_err(|e| e.to_string())?;
            let mut t = format!("Análisis paramétrico de {} ({}) en [{}, {}]\n\n| desde | hasta | estado | pendiente | Z al inicio | Z al final |\n| --: | --: | :-- | --: | --: | --: |\n", v["name"].as_str().unwrap_or(""), v["kind"].as_str().unwrap_or(""), v["from"], v["to"]);
            for sg in v["segments"].as_array().into_iter().flatten() {
                t += &format!("| {} | {} | {} | {} | {} | {} |\n", sg["from"], sg["to"], sg["status"].as_str().unwrap_or(""), sg["slope"], sg["objective_from"], sg["objective_to"]);
            }
            Ok(Reply { content: vec![md(t)], structured: Some(v) })
        }
        "tune_parameters" => {
            let v = api::call("tune", json!({"model": need("model")?, "options": args.get("options"), "budget": args.get("budget"), "lang": lang().code()})).map_err(|e| e.to_string())?;
            let mut t = format!("{}\n\n| configuración | tiempo (s) | mismo resultado | rapidez |\n| :-- | --: | :-: | --: |\n", v["summary"].as_str().unwrap_or(""));
            for tr in v["trials"].as_array().into_iter().flatten() {
                t += &format!("| {} | {:.4} | {} | ×{:.2} |\n", tr["name"].as_str().unwrap_or(""), tr["time"].as_f64().unwrap_or(0.0), if tr["same_result"].as_bool() == Some(true) { "sí" } else { "no" }, tr["speedup"].as_f64().unwrap_or(1.0));
            }
            t += &format!("\nParámetros recomendados (para options): {}", v["best"]);
            Ok(Reply { content: vec![md(t)], structured: Some(v) })
        }
        "export_report" => {
            let mut a = args.clone();
            if let Some(id) = s("report_id") {
                a["report"] = fetch(id)?;
            }
            a["module"] = json!(a.get("module").and_then(|m| m.as_str()).map(|m| api::module_id(m).unwrap_or_else(|| m.to_string())).or_else(|| a["report"]["module"].as_str().map(str::to_string)).unwrap_or_else(|| if a["report"].get("variables").is_some() { "lp".into() } else { module.to_string() }));
            a["lang"] = json!(lang().code());
            let v = api::call("export_report", a).map_err(|e| e.to_string())?;
            let bytes = match v.get("base64").and_then(|b| b.as_str()) {
                Some(b) => api::base64_decode(b).map_err(|e| e.to_string())?,
                None => v["content"].as_str().unwrap_or("").as_bytes().to_vec(),
            };
            let path = match s("path") {
                Some(p) => std::path::PathBuf::from(p),
                None => {
                    let dir = std::env::temp_dir().join("IAQSB");
                    let _ = std::fs::create_dir_all(&dir);
                    dir.join(v["filename"].as_str().unwrap_or("informe"))
                }
            };
            std::fs::write(&path, &bytes).map_err(|e| format!("no se pudo escribir {}: {e}", path.display()))?;
            Ok(plain(json!({"saved": path.to_string_lossy(), "bytes": bytes.len(), "format": s("format")})))
        }
        "export_excel" => {
            let b = api::model_to_xlsx(module, need("model")?, Some(lang().code())).map_err(|e| e.to_string())?;
            let p = s("path").ok_or("falta path")?;
            std::fs::write(p, &b).map_err(|e| format!("no se pudo escribir {p}: {e}"))?;
            Ok(plain(json!({"saved": p, "bytes": b.len()})))
        }
        "import_excel" => {
            let p = s("path").ok_or("falta path")?;
            let b = std::fs::read(p).map_err(|e| format!("no se pudo leer {p}: {e}"))?;
            let m = s("module").map(|m| api::module_id(m).unwrap_or_else(|| m.to_string()));
            let v = api::xlsx_to_model(&b, m.as_deref(), s("problem_type")).map_err(|e| e.to_string())?;
            Ok(plain(v))
        }
        "convert_model" => {
            let to = s("to").unwrap_or("mps");
            let v = api::convert_lp(need("model")?, to).map_err(|e| e.to_string())?;
            let content = v.get("content").and_then(|c| c.as_str()).map(str::to_string).unwrap_or_else(|| j(&v["model"]));
            if let Some(p) = s("path") {
                // WinQSB lee sus archivos en ANSI (Windows-1252)
                let bytes = if matches!(to.trim_start_matches('.').to_lowercase().as_str(), "lpp" | "winqsb") {
                    api::formats::encode_ansi(&content)
                } else {
                    content.clone().into_bytes()
                };
                std::fs::write(p, bytes).map_err(|e| format!("no se pudo escribir {p}: {e}"))?;
                Ok(plain(json!({"saved": p, "bytes": content.len()})))
            } else {
                Ok(Reply { content: vec![text(content)], structured: None })
            }
        }
        "import_winqsb_file" => {
            let bytes = if let Some(p) = s("path") {
                std::fs::read(p).map_err(|e| format!("no se pudo leer {p}: {e}"))?
            } else if let Some(c) = s("content") {
                c.as_bytes().to_vec()
            } else {
                return Err("indica path o content".into());
            };
            api::import_bytes(&bytes).map(plain).map_err(|e| e.to_string())
        }
        "export_winqsb_file" => {
            let bytes = api::export_bytes(module, need("model")?, s("layout")).map_err(|e| e.to_string())?;
            if let Some(p) = s("path") {
                std::fs::write(p, &bytes).map_err(|e| format!("no se pudo escribir {p}: {e}"))?;
                Ok(plain(json!({"saved": p, "bytes": bytes.len()})))
            } else {
                Ok(Reply { content: vec![text(api::formats::decode_ansi(&bytes))], structured: None })
            }
        }
        "get_bibliography" => {
            if args.get("bibtex").and_then(|v| v.as_bool()) == Some(true) {
                let b = api::call("bibtex", json!({})).map_err(|e| e.to_string())?;
                Ok(Reply { content: vec![text(b["content"].as_str().map(str::to_string).unwrap_or_else(|| j(&b)))], structured: None })
            } else {
                let b = tv(api::bibliography());
                let mut t = String::new();
                for r in b.as_array().into_iter().flatten() {
                    t += &format!(
                        "- [{}] {}{}\n",
                        r["id"].as_str().unwrap_or(""),
                        r["apa"].as_str().unwrap_or(""),
                        if r["verified"].as_bool() == Some(false) { " **(por verificar)**" } else { "" }
                    );
                }
                Ok(Reply { content: vec![md(t)], structured: None })
            }
        }
        "get_example" => {
            let ex = api::call("examples", json!({"lang": lang().code()})).map_err(|e| e.to_string())?;
            match s("name") {
                None => {
                    let mut t = String::from("| módulo | nombre | fuente |\n| :-- | :-- | :-- |\n");
                    for e in ex.as_array().into_iter().flatten() {
                        if s("module").is_some_and(|m| e["module"] != m) {
                            continue;
                        }
                        t += &format!("| {} | {} | {} |\n", e["module"].as_str().unwrap_or(""), e["name"].as_str().unwrap_or(""), e["source"]["summary"].as_str().unwrap_or("").replace('|', "/"));
                    }
                    Ok(Reply { content: vec![md(t)], structured: None })
                }
                Some(n) => ex
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|e| e["name"] == n)
                    .map(|e| plain(json!({"module": e["module"], "model": e["model"], "source": e["source"]["summary"]})))
                    .ok_or_else(|| format!("no hay ningún ejemplo llamado {n}; get_example sin nombre los lista")),
            }
        }
        other => Err(format!("herramienta desconocida: {other}")),
    }
}

#[cfg(test)]
mod pruebas_motor {
    use super::*;

    fn texto(r: &Reply) -> String {
        r.content.iter().filter_map(|c| c["text"].as_str()).collect::<Vec<_>>().join("\n")
    }

    const LP: &str = "Maximize\n obj: x + y\nSubject To\n c1: x + y <= 4\n c2: x + y >= 6\nEnd\n";

    #[test]
    fn herramientas_del_motor() {
        let r = call("solver_info", &json!({})).unwrap();
        assert!(texto(&r).contains("highs"));

        let r = call("diagnose_infeasibility", &json!({"model": {"format": "lp", "text": LP}})).unwrap();
        let t = texto(&r);
        assert!(t.contains("c1") && t.contains("c2"), "{t}");

        let r = call("convert_model", &json!({"model": {"format": "lp", "text": LP}, "to": "mps"})).unwrap();
        assert!(texto(&r).contains("ROWS"));

        let r = call("solve", &json!({"module": "lp", "model": {"format": "lp", "text": LP}})).unwrap();
        let t = texto(&r);
        assert!(t.contains("factible"), "{t}");
    }

    const ML: &str = "set I = {A, B}; set J = 1..3; param c{I, J} = [[4, 6, 9], [5, 3, 7]]; param s{I} = [30, 40]; param d{J} = [20, 25, 15]; var x{I, J} >= 0; minimize costo: sum{i in I, j in J} c[i,j]*x[i,j]; subject to of{i in I}: sum{j in J} x[i,j] <= s[i]; de{j in J}: sum{i in I} x[i,j] >= d[j];";

    #[test]
    fn herramientas_nuevas() {
        let r = call("check_model", &json!({"text": ML})).unwrap();
        assert!(texto(&r).contains("\"ok\":true"), "{}", texto(&r));
        let r = call("check_model", &json!({"text": "var x; maximize z: 3x +;"})).unwrap();
        assert!(texto(&r).contains("line"), "{}", texto(&r));
        let model = json!({"format": "iaqsb", "text": ML});
        let r = call("solve", &json!({"module": "lp", "model": model})).unwrap();
        let t = texto(&r);
        assert!(t.contains("x[") || t.contains("Variable x"), "{t}");
        let wy = api::example_model("lp").unwrap();
        let r = call("what_if", &json!({"module": "lp", "model": wy, "path": "/constraints/2/rhs", "values": {"from": 12, "to": 24, "steps": 3}})).unwrap();
        assert!(texto(&r).contains("| 20.0 |") && texto(&r).contains("| 38.0 |"), "{}", texto(&r));
        let r = call("parametric_analysis", &json!({"model": wy, "kind": "cost", "index": 0, "from": 0, "to": 10})).unwrap();
        assert!(texto(&r).contains("Análisis paramétrico"));
        let r = call("tune_parameters", &json!({"model": model, "budget": 2})).unwrap();
        assert!(texto(&r).contains("Parámetros recomendados"));
        let dir = std::env::temp_dir().join("iaqsb_prueba_mcp");
        let _ = std::fs::create_dir_all(&dir);
        let pdf = dir.join("informe.pdf");
        call("export_report", &json!({"module": "lp", "model": wy, "format": "pdf", "path": pdf.to_string_lossy()})).unwrap();
        assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
        let r = call("solve", &json!({"module": "lp", "model": wy})).unwrap();
        let id = r.structured.as_ref().and_then(|s| s["report_id"].as_str()).unwrap().to_string();
        let docx = dir.join("informe.docx");
        call("export_report", &json!({"report_id": id, "format": "docx", "path": docx.to_string_lossy()})).unwrap();
        assert!(std::fs::read(&docx).unwrap().starts_with(b"PK"));
        let xl = dir.join("modelo.xlsx");
        call("export_excel", &json!({"module": "lp", "model": wy, "path": xl.to_string_lossy()})).unwrap();
        let r = call("import_excel", &json!({"path": xl.to_string_lossy()})).unwrap();
        assert!(texto(&r).contains("Puertas"), "{}", texto(&r));
    }
}
