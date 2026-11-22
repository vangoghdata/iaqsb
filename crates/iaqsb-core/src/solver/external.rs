//! Motores externos: si en el equipo están instalados, IAQSB les manda el
//! modelo en MPS y trae la respuesta. Así quien tenga licencia de Gurobi o CPLEX
//! los usa desde IAQSB, y quien no, usa HiGHS (incluido) o motores libres.
//!
//! - Gurobi: `gurobi_cl` o el paquete de Python `gurobipy`.
//! - CPLEX: `cplex` o el paquete de Python `cplex`.
//! - SCIP: `scip` o el paquete de Python `pyscipopt`.
//! - CBC: `cbc` (COIN-OR).
//! - GLPK: `glpsol`.
//!
//! El Python se busca como `python3`, `python` o `py` (o en IAQSB_PYTHON).

use super::*;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const BRIDGE: &str = include_str!("externo.py");

/// Un motor disponible en este equipo.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SolverInfo {
    /// Nombre para `params.solver`.
    pub id: String,
    /// Motor y versión.
    pub name: String,
    /// Cómo se llama: "incluido", "gurobi_cl", "python: gurobipy"…
    pub via: String,
    /// Qué resuelve.
    pub problems: String,
    pub note: String,
}

/// Busca un programa en el PATH (en Windows, también con .exe, .cmd o .bat).
pub fn find_exe(name: &str) -> Option<PathBuf> {
    let exts: &[&str] = if cfg!(windows) { &[".exe", ".cmd", ".bat", ""] } else { &[""] };
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        for e in exts {
            let p = dir.join(format!("{name}{e}"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// Programa de Python y sus argumentos previos ("py -3").
pub fn python() -> Option<&'static (PathBuf, Vec<String>)> {
    static P: OnceLock<Option<(PathBuf, Vec<String>)>> = OnceLock::new();
    P.get_or_init(|| {
        let mut cands: Vec<(PathBuf, Vec<String>)> = vec![];
        if let Ok(p) = std::env::var("IAQSB_PYTHON") {
            cands.push((PathBuf::from(p), vec![]));
        }
        for n in ["python3", "python"] {
            if let Some(p) = find_exe(n) {
                cands.push((p, vec![]));
            }
        }
        if let Some(p) = find_exe("py") {
            cands.push((p, vec!["-3".into()]));
        }
        cands.into_iter().find(|(p, pre)| {
            Command::new(p).args(pre).args(["-c", "import sys; sys.exit(0)"]).stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
        })
    })
    .as_ref()
}

fn bridge_path() -> Result<PathBuf, String> {
    let p = std::env::temp_dir().join(format!("iaqsb-externo-{}.py", env!("CARGO_PKG_VERSION")));
    if std::fs::read_to_string(&p).ok().as_deref() != Some(BRIDGE) {
        std::fs::write(&p, BRIDGE).map_err(|e| format!("no se pudo preparar el puente con Python: {e}"))?;
    }
    Ok(p)
}

/// Paquetes de Python con motor (se consulta una vez).
fn python_solvers() -> &'static std::collections::BTreeMap<String, String> {
    static S: OnceLock<std::collections::BTreeMap<String, String>> = OnceLock::new();
    S.get_or_init(|| {
        let mut out = std::collections::BTreeMap::new();
        let Some((py, pre)) = python() else { return out };
        let Ok(bridge) = bridge_path() else { return out };
        let res = temp_path("json");
        let ok = run_with_limit(Command::new(py).args(pre).arg(&bridge).args(["check", "-", "-"]).arg(&res), Duration::from_secs(60)).is_ok();
        if ok {
            if let Ok(v) = std::fs::read_to_string(&res).map(|t| serde_json::from_str::<Value>(&t).unwrap_or(Value::Null)) {
                if let Some(f) = v.get("found").and_then(|f| f.as_object()) {
                    for (k, v) in f {
                        out.insert(k.clone(), v.as_str().unwrap_or(k).to_string());
                    }
                }
            }
        }
        let _ = std::fs::remove_file(&res);
        out
    })
}

/// Motores disponibles en este equipo (HiGHS siempre).
pub fn available() -> Vec<SolverInfo> {
    let mut v = vec![SolverInfo {
        id: "highs".into(),
        name: highs::version(),
        via: "incluido".into(),
        problems: "LP, MIP, QP convexo".into(),
        note: "Motor por defecto; libre (MIT), sin límites de tamaño.".into(),
    }];
    let py = python_solvers();
    let entry = |id: &str, exe: &str, pkg: &str, problems: &str, note: &str| -> Option<SolverInfo> {
        if let Some(p) = find_exe(exe) {
            return Some(SolverInfo { id: id.into(), name: id.to_uppercase(), via: p.display().to_string(), problems: problems.into(), note: note.into() });
        }
        py.get(id).map(|ver| SolverInfo { id: id.into(), name: ver.clone(), via: format!("python: {pkg}"), problems: problems.into(), note: note.into() })
    };
    v.extend(entry("gurobi", "gurobi_cl", "gurobipy", "LP, MIP, QP", "Comercial; la licencia es del usuario (gurobipy sin licencia admite modelos de hasta 2000 variables y restricciones)."));
    v.extend(entry("cplex", "cplex", "cplex", "LP, MIP, QP", "Comercial; la licencia es del usuario (la edición Community admite hasta 1000 variables y restricciones)."));
    v.extend(entry("scip", "scip", "pyscipopt", "LP, MIP", "Libre (Apache 2.0)."));
    if let Some(p) = find_exe("cbc") {
        v.push(SolverInfo { id: "cbc".into(), name: "CBC".into(), via: p.display().to_string(), problems: "LP, MIP".into(), note: "Libre (EPL), de COIN-OR.".into() });
    }
    if let Some(p) = find_exe("glpsol") {
        v.push(SolverInfo { id: "glpk".into(), name: "GLPK".into(), via: p.display().to_string(), problems: "LP, MIP".into(), note: "Libre (GPL).".into() });
    }
    v
}

/// Ejecuta con un límite de tiempo (lo mata si se pasa).
fn run_with_limit(cmd: &mut Command, limit: Duration) -> Result<(), String> {
    let mut child = cmd.stdout(Stdio::null()).stderr(Stdio::piped()).stdin(Stdio::null()).spawn().map_err(|e| e.to_string())?;
    let t0 = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) if t0.elapsed() > limit => {
                let _ = child.kill();
                return Err("el motor no terminó a tiempo".into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(e.to_string()),
        }
    }
}

#[allow(dead_code)]
struct Raw {
    status: Status,
    x: Option<Vec<f64>>,
    objective: Option<f64>,
    bound: Option<f64>,
    gap: Option<f64>,
    duals: Option<Vec<f64>>,
    rc: Option<Vec<f64>>,
    nodes: i64,
    iterations: i64,
    version: String,
    message: String,
}

fn parse_status(s: &str) -> Status {
    match s {
        "optimal" => Status::Optimal,
        "infeasible" => Status::Infeasible,
        "unbounded" => Status::Unbounded,
        "infeasible_or_unbounded" => Status::InfeasibleOrUnbounded,
        "time_limit" => Status::TimeLimit,
        "node_limit" => Status::NodeLimit,
        "solution_limit" => Status::SolutionLimit,
        "iteration_limit" => Status::IterationLimit,
        "interrupted" => Status::Interrupted,
        _ => Status::Error,
    }
}

/// Resuelve con un motor externo.
pub fn solve(name: &str, m: &Model, p: &Params) -> Solution {
    let t0 = Instant::now();
    let id = match name {
        "glpsol" => "glpk",
        "gurobi_cl" | "grb" => "gurobi",
        "coin" | "coin-cbc" => "cbc",
        other => other,
    };
    // siempre se escribe minimizando (GLPK y algunos lectores MPS no leen OBJSENSE)
    let mut w = m.clone();
    if w.maximize {
        w.maximize = false;
        w.offset = -w.offset;
        for c in &mut w.cols {
            c.cost = -c.cost;
        }
        for q in &mut w.q {
            q.2 = -q.2;
        }
    }
    let mps = match write_text(&w, Format::Mps) {
        Ok(t) => t,
        Err(e) => return Solution::empty(m, Status::Error, id, e),
    };
    let names = safe_names(m.cols.iter().map(|c| c.name.as_str()), "x");
    let rnames = safe_names(m.rows.iter().map(|r| r.name.as_str()), "c");
    let dir = temp_path("d");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return Solution::empty(m, Status::Error, id, e.to_string());
    }
    let model_path = dir.join("modelo.mps");
    let _ = std::fs::write(&model_path, mps);
    let limit = Duration::from_secs_f64(p.time_limit.unwrap_or(3600.0) + 60.0);
    // la solución inicial, con los nombres como quedaron en el MPS
    let mut p = p.clone();
    if let Some(st) = p.start.take() {
        let map: std::collections::HashMap<&str, &str> = m.cols.iter().zip(&names).map(|(c, n)| (c.name.as_str(), n.as_str())).collect();
        p.start = Some(st.into_iter().filter_map(|(k, v)| map.get(k.as_str()).map(|n| (n.to_string(), v))).collect());
    }
    let p = &p;
    let log_path = dir.join("registro.log");
    let raw = match id {
        "gurobi" | "cplex" | "scip" => run_python_or_exe(id, &dir, &model_path, &log_path, &names, &rnames, p, limit),
        "cbc" => run_cbc(&dir, &model_path, &names, p, limit),
        "glpk" => run_glpk(&dir, &model_path, m.is_mip(), p, limit, &log_path),
        other => Err(format!("motor desconocido: {other}. Usa highs, gurobi, cplex, scip, cbc o glpk")),
    };
    let log = if p.log { std::fs::read_to_string(&log_path).ok() } else { None };
    let _ = std::fs::remove_dir_all(&dir);
    let mut raw = match raw {
        Ok(r) => r,
        Err(e) => return Solution::empty(m, Status::Error, id, e),
    };
    let flip = if m.maximize { -1.0 } else { 1.0 };
    let has = raw.x.is_some() && !matches!(raw.status, Status::Infeasible | Status::Error | Status::InfeasibleOrUnbounded);
    let x = raw.x.take().unwrap_or_else(|| vec![0.0; m.cols.len()]);
    let objective = if has { Some(m.objective_at(&x)) } else { None };
    let lp_opt = !m.is_mip() && raw.status == Status::Optimal;
    Solution {
        status: raw.status,
        has_solution: has,
        objective,
        bound: raw.bound.map(|b| flip * b).filter(|b| b.is_finite()),
        gap: raw.gap.filter(|g| g.is_finite() && *g < 1e20),
        row_activity: if has { m.activity(&x) } else { vec![0.0; m.rows.len()] },
        x,
        row_dual: if lp_opt { raw.duals.map(|d| d.into_iter().map(|v| flip * v).collect()) } else { None },
        reduced_cost: if lp_opt { raw.rc.map(|d| d.into_iter().map(|v| flip * v).collect()) } else { None },
        col_basis: None,
        row_basis: None,
        iterations: raw.iterations,
        nodes: raw.nodes,
        time: t0.elapsed().as_secs_f64(),
        solver: raw.version,
        is_mip: m.is_mip(),
        cols: m.cols.len(),
        rows: m.rows.len(),
        nonzeros: m.nonzeros(),
        objective_values: vec![],
        log,
        message: raw.message,
    }
}

fn by_name(map: Option<&serde_json::Map<String, Value>>, names: &[String]) -> Option<Vec<f64>> {
    let map = map?;
    if map.is_empty() {
        return None;
    }
    Some(names.iter().map(|n| map.get(n).and_then(|v| v.as_f64()).unwrap_or(0.0)).collect())
}

#[allow(clippy::too_many_arguments)]
fn run_python_or_exe(id: &str, dir: &Path, model: &Path, log: &Path, names: &[String], rnames: &[String], p: &Params, limit: Duration) -> Result<Raw, String> {
    let exe = match id {
        "gurobi" => find_exe("gurobi_cl"),
        "cplex" => find_exe("cplex"),
        _ => find_exe("scip"),
    };
    if let Some(exe) = exe {
        return run_exe(id, &exe, dir, model, log, names, p, limit);
    }
    if !python_solvers().contains_key(id) {
        return Err(format!(
            "{} no está instalado en este equipo (ni el programa ni el paquete de Python). Motores disponibles: {}",
            id.to_uppercase(),
            available().iter().map(|s| s.id.clone()).collect::<Vec<_>>().join(", ")
        ));
    }
    let (py, pre) = python().ok_or("no se encontró Python")?;
    let bridge = bridge_path()?;
    let mut pj = serde_json::to_value(p).unwrap_or(Value::Null);
    if let Some(o) = pj.as_object_mut() {
        if p.log {
            o.insert("log_file".into(), Value::String(log.to_string_lossy().into()));
        }
    }
    let ppath = dir.join("parametros.json");
    let rpath = dir.join("resultado.json");
    std::fs::write(&ppath, pj.to_string()).map_err(|e| e.to_string())?;
    run_with_limit(Command::new(py).args(pre).arg(&bridge).arg(id).arg(model).arg(&ppath).arg(&rpath), limit)?;
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&rpath).map_err(|_| format!("{} no devolvió resultado", id.to_uppercase()))?).map_err(|e| e.to_string())?;
    let f = |k: &str| v.get(k).and_then(|x| x.as_f64());
    Ok(Raw {
        status: parse_status(v["status"].as_str().unwrap_or("error")),
        x: by_name(v["x"].as_object(), names),
        objective: f("objective"),
        bound: f("bound"),
        gap: f("gap"),
        duals: by_name(v["duals"].as_object(), rnames),
        rc: by_name(v["rc"].as_object(), names),
        nodes: v["nodes"].as_i64().unwrap_or(0),
        iterations: v["iterations"].as_i64().unwrap_or(0),
        version: v["version"].as_str().unwrap_or(id).to_string(),
        message: v["message"].as_str().unwrap_or("").to_string(),
    })
}

#[allow(clippy::too_many_arguments)]
fn run_exe(id: &str, exe: &Path, dir: &Path, model: &Path, log: &Path, names: &[String], p: &Params, limit: Duration) -> Result<Raw, String> {
    let sol = dir.join("solucion.sol");
    let mut cmd = Command::new(exe);
    match id {
        "gurobi" => {
            cmd.arg(format!("ResultFile={}", sol.display())).arg(format!("LogFile={}", log.display()));
            if let Some(t) = p.time_limit {
                cmd.arg(format!("TimeLimit={t}"));
            }
            if let Some(g) = p.mip_gap {
                cmd.arg(format!("MIPGap={g}"));
            }
            if let Some(t) = p.threads {
                cmd.arg(format!("Threads={t}"));
            }
            if p.presolve == Some(false) {
                cmd.arg("Presolve=0");
            }
            cmd.arg(model);
        }
        "cplex" => {
            let mut script = vec![format!("read {}", model.display())];
            if let Some(t) = p.time_limit {
                script.push(format!("set timelimit {t}"));
            }
            if let Some(g) = p.mip_gap {
                script.push(format!("set mip tolerances mipgap {g}"));
            }
            if let Some(t) = p.threads {
                script.push(format!("set threads {t}"));
            }
            script.push("optimize".into());
            script.push(format!("write {} sol", sol.display()));
            script.push("quit".into());
            cmd.arg("-c").args(script);
        }
        _ => {
            let mut script = vec![];
            if let Some(t) = p.time_limit {
                script.push(format!("set limits time {t}"));
            }
            if let Some(g) = p.mip_gap {
                script.push(format!("set limits gap {g}"));
            }
            script.push(format!("read {}", model.display()));
            script.push("optimize".into());
            script.push(format!("write solution {}", sol.display()));
            script.push("quit".into());
            for s in script {
                cmd.arg("-c").arg(s);
            }
            cmd.arg("-l").arg(log);
        }
    }
    run_with_limit(&mut cmd, limit)?;
    let text = std::fs::read_to_string(&sol).unwrap_or_default();
    let logt = std::fs::read_to_string(log).unwrap_or_default();
    let mut vals: std::collections::HashMap<String, f64> = Default::default();
    let mut status = Status::Error;
    let mut objective = None;
    match id {
        "cplex" => {
            // XML: <header ... solutionStatusString="integer optimal solution" .../> <variable name="x" ... value="2"/>
            let attr = |line: &str, k: &str| -> Option<String> {
                let i = line.find(&format!("{k}=\""))? + k.len() + 2;
                Some(line[i..].split('"').next()?.to_string())
            };
            for line in text.lines() {
                let l = line.trim();
                if l.starts_with("<header") || l.contains("solutionStatusString") {
                    if let Some(s) = attr(l, "solutionStatusString") {
                        let s = s.to_lowercase();
                        status = if s.contains("optimal") { Status::Optimal } else if s.contains("infeasible") { Status::Infeasible } else if s.contains("unbounded") { Status::Unbounded } else if s.contains("time") { Status::TimeLimit } else { Status::Error };
                    }
                    objective = attr(l, "objectiveValue").and_then(|v| v.parse().ok()).or(objective);
                }
                if l.starts_with("<variable") {
                    if let (Some(n), Some(v)) = (attr(l, "name"), attr(l, "value").and_then(|v| v.parse::<f64>().ok())) {
                        vals.insert(n, v);
                    }
                }
            }
            if text.is_empty() && logt.to_lowercase().contains("infeasible") {
                status = Status::Infeasible;
            }
        }
        _ => {
            // Gurobi (.sol) y SCIP: "nombre valor" por línea
            for line in text.lines() {
                let l = line.trim();
                if l.starts_with('#') {
                    if let Some(v) = l.split('=').nth(1).and_then(|v| v.trim().parse().ok()) {
                        objective = Some(v);
                    }
                    continue;
                }
                if let Some(rest) = l.strip_prefix("objective value:") {
                    objective = rest.trim().parse().ok();
                    continue;
                }
                if let Some(rest) = l.strip_prefix("solution status:") {
                    let r = rest.to_lowercase();
                    status = if r.contains("optimal") { Status::Optimal } else if r.contains("infeasible") { Status::Infeasible } else if r.contains("unbounded") { Status::Unbounded } else if r.contains("time") { Status::TimeLimit } else { Status::Error };
                    continue;
                }
                let mut it = l.split_whitespace();
                if let (Some(n), Some(v)) = (it.next(), it.next().and_then(|v| v.parse::<f64>().ok())) {
                    vals.insert(n.to_string(), v);
                }
            }
            if id == "gurobi" {
                let lg = logt.to_lowercase();
                status = if lg.contains("optimal solution found") {
                    Status::Optimal
                } else if lg.contains("infeasible or unbounded") {
                    Status::InfeasibleOrUnbounded
                } else if lg.contains("infeasible") {
                    Status::Infeasible
                } else if lg.contains("unbounded") {
                    Status::Unbounded
                } else if lg.contains("time limit") {
                    Status::TimeLimit
                } else if !vals.is_empty() {
                    Status::Interrupted
                } else {
                    Status::Error
                };
            }
        }
    }
    let x = (!vals.is_empty()).then(|| names.iter().map(|n| vals.get(n).copied().unwrap_or(0.0)).collect());
    Ok(Raw { status, x, objective, bound: None, gap: None, duals: None, rc: None, nodes: 0, iterations: 0, version: id.to_uppercase(), message: String::new() })
}

fn run_cbc(dir: &Path, model: &Path, names: &[String], p: &Params, limit: Duration) -> Result<Raw, String> {
    let exe = find_exe("cbc").ok_or("CBC no está instalado en este equipo")?;
    let sol = dir.join("solucion.txt");
    let mut cmd = Command::new(exe);
    cmd.arg(model);
    if let Some(t) = p.time_limit {
        cmd.args(["-sec", &t.to_string()]);
    }
    if let Some(g) = p.mip_gap {
        cmd.args(["-ratio", &g.to_string()]);
    }
    if let Some(g) = p.mip_abs_gap {
        cmd.args(["-allow", &g.to_string()]);
    }
    if let Some(t) = p.threads {
        cmd.args(["-threads", &t.to_string()]);
    }
    if let Some(n) = p.node_limit {
        cmd.args(["-maxNodes", &n.to_string()]);
    }
    if p.presolve == Some(false) {
        cmd.args(["-presolve", "off"]);
    }
    cmd.args(["-solve", "-printingOptions", "all", "-solu"]).arg(&sol);
    run_with_limit(&mut cmd, limit)?;
    let text = std::fs::read_to_string(&sol).map_err(|_| "CBC no escribió la solución".to_string())?;
    let mut lines = text.lines();
    let head = lines.next().unwrap_or("").to_lowercase();
    let status = if head.starts_with("optimal") {
        Status::Optimal
    } else if head.contains("infeasible") {
        Status::Infeasible
    } else if head.contains("unbounded") {
        Status::Unbounded
    } else if head.contains("time") {
        Status::TimeLimit
    } else if head.contains("stopped") {
        Status::Interrupted
    } else {
        Status::Error
    };
    let mut x = vec![0.0; names.len()];
    let mut any = false;
    for l in lines {
        let f: Vec<&str> = l.split_whitespace().collect();
        // "   0 x   2   3" o con "**" delante si viola algo
        let f: Vec<&str> = f.into_iter().filter(|t| *t != "**").collect();
        if f.len() >= 3 {
            if let (Ok(i), Ok(v)) = (f[0].parse::<usize>(), f[2].parse::<f64>()) {
                if i < x.len() {
                    x[i] = v;
                    any = true;
                }
            }
        }
    }
    let feasible_head = !matches!(status, Status::Infeasible | Status::Error);
    Ok(Raw { status, x: (any && feasible_head).then_some(x), objective: None, bound: None, gap: None, duals: None, rc: None, nodes: 0, iterations: 0, version: "CBC".into(), message: String::new() })
}

fn run_glpk(dir: &Path, model: &Path, mip: bool, p: &Params, limit: Duration, log: &Path) -> Result<Raw, String> {
    let exe = find_exe("glpsol").ok_or("GLPK (glpsol) no está instalado en este equipo")?;
    let sol = dir.join("solucion.txt");
    let mut cmd = Command::new(exe);
    cmd.arg("--freemps").arg(model);
    if let Some(t) = p.time_limit {
        cmd.args(["--tmlim", &(t.ceil() as i64).max(1).to_string()]);
    }
    if let Some(g) = p.mip_gap {
        cmd.args(["--mipgap", &g.to_string()]);
    }
    if p.presolve == Some(true) {
        cmd.arg("--intopt");
    }
    cmd.arg("--log").arg(log).arg("-w").arg(&sol);
    run_with_limit(&mut cmd, limit)?;
    let text = std::fs::read_to_string(&sol).map_err(|_| "GLPK no escribió la solución".to_string())?;
    let mut status = Status::Error;
    let mut cols: Vec<(usize, f64)> = vec![];
    let mut n = 0usize;
    for l in text.lines() {
        let f: Vec<&str> = l.split_whitespace().collect();
        match f.first().copied() {
            Some("s") if f.len() >= 5 => {
                n = f[3].parse().unwrap_or(0);
                // MIP: s mip filas cols estado obj · LP: s bas filas cols primal dual obj
                status = if f[1] == "mip" {
                    match f[4] {
                        "o" => Status::Optimal,
                        "f" => Status::Interrupted,
                        "n" => Status::Infeasible,
                        _ => Status::Error,
                    }
                } else {
                    match (f[4], f.get(5).copied().unwrap_or("u")) {
                        ("f", "f") => Status::Optimal,
                        ("n", _) | ("i", _) => Status::Infeasible,
                        ("f", "n") => Status::Unbounded,
                        _ => Status::Error,
                    }
                };
            }
            Some("j") if f.len() >= 3 => {
                let j: usize = f[1].parse().unwrap_or(0);
                // MIP: j col valor · LP: j col estado primal dual
                let v = if mip { f[2].parse().ok() } else { f.get(3).and_then(|v| v.parse().ok()) };
                if let (true, Some(v)) = (j >= 1, v) {
                    cols.push((j - 1, v));
                }
            }
            _ => {}
        }
    }
    let x = if matches!(status, Status::Optimal | Status::Interrupted) && n > 0 {
        let mut x = vec![0.0; n];
        for (j, v) in cols {
            if j < n {
                x[j] = v;
            }
        }
        Some(x)
    } else {
        None
    };
    Ok(Raw { status, x, objective: None, bound: None, gap: None, duals: None, rc: None, nodes: 0, iterations: 0, version: "GLPK".into(), message: String::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wyndor_int() -> Model {
        let mut m = Model { maximize: true, ..Default::default() };
        let x = m.add_col("Puertas de vidrio", 3.0, 0.0, f64::INFINITY, Kind::Integer);
        let y = m.add_col("Ventanas", 5.0, 0.0, f64::INFINITY, Kind::Integer);
        m.add_row("Planta 1", vec![(x, 1.0)], f64::NEG_INFINITY, 4.0);
        m.add_row("Planta 2", vec![(y, 2.0)], f64::NEG_INFINITY, 12.0);
        m.add_row("Planta 3", vec![(x, 3.0), (y, 2.0)], f64::NEG_INFINITY, 18.0);
        m
    }

    /// Cada motor que esté instalado da el mismo óptimo que HiGHS.
    #[test]
    fn motores_instalados_coinciden() {
        let m = wyndor_int();
        for s in available() {
            let r = solve_single(&m, &Params { solver: Some(s.id.clone()), ..Default::default() });
            assert_eq!(r.status, Status::Optimal, "{}: {:?} {}", s.id, r.status, r.message);
            assert!((r.objective.unwrap() - 36.0).abs() < 1e-6, "{}: {:?}", s.id, r.objective);
            assert!((r.x[0] - 2.0).abs() < 1e-6 && (r.x[1] - 6.0).abs() < 1e-6, "{}: {:?}", s.id, r.x);
        }
        // duales de un LP con motores que los dan: mismos precios sombra que HiGHS
        let mut lp = m.clone();
        for c in &mut lp.cols {
            c.kind = Kind::Continuous;
        }
        let base = solve_single(&lp, &Params::default()).row_dual.unwrap();
        for s in available().into_iter().filter(|s| ["gurobi", "cplex"].contains(&s.id.as_str())) {
            let r = solve_single(&lp, &Params { solver: Some(s.id.clone()), ..Default::default() });
            let d = r.row_dual.unwrap_or_else(|| panic!("{} sin duales", s.id));
            for (a, b) in d.iter().zip(&base) {
                assert!((a - b).abs() < 1e-6, "{}: {d:?} vs {base:?}", s.id);
            }
        }
    }
}
