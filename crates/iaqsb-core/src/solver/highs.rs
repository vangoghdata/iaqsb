//! HiGHS por su interfaz en C: modelo disperso, opciones, solución, base,
//! rangos, IIS, relajación de factibilidad y archivos MPS/LP.

use super::*;
use highs_sys::*;
use std::ffi::{c_void, CStr, CString};
use std::time::Instant;

type I = HighsInt;

const STATUS_ERROR: I = -1;
const FORMAT_COLWISE: I = 1;
const HESSIAN_TRIANGULAR: I = 1;
const SENSE_MIN: I = 1;
const SENSE_MAX: I = -1;
const VAR_CONTINUOUS: I = 0;
const VAR_INTEGER: I = 1;
const VAR_SEMI_CONTINUOUS: I = 2;
const VAR_SEMI_INTEGER: I = 3;
const IIS_BOUND_FREE: I = 1;
const IIS_BOUND_LOWER: I = 2;
const IIS_BOUND_UPPER: I = 3;
const NAME_LEN: usize = 512;

/// Instancia de HiGHS (se libera al salir de alcance).
struct H(*mut c_void);

impl Drop for H {
    fn drop(&mut self) {
        unsafe { Highs_destroy(self.0) }
    }
}

fn c(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap()
}

impl H {
    fn new() -> H {
        let h = H(unsafe { Highs_create() });
        h.bool("output_flag", false);
        h
    }
    fn bool(&self, k: &str, v: bool) {
        unsafe { Highs_setBoolOptionValue(self.0, c(k).as_ptr(), v as I) };
    }
    fn int(&self, k: &str, v: i64) {
        unsafe { Highs_setIntOptionValue(self.0, c(k).as_ptr(), v.clamp(I::MIN as i64, I::MAX as i64) as I) };
    }
    fn dbl(&self, k: &str, v: f64) {
        unsafe { Highs_setDoubleOptionValue(self.0, c(k).as_ptr(), v) };
    }
    fn string(&self, k: &str, v: &str) {
        unsafe { Highs_setStringOptionValue(self.0, c(k).as_ptr(), c(v).as_ptr()) };
    }
    fn info_f(&self, k: &str) -> Option<f64> {
        let mut v = 0.0;
        let st = unsafe { Highs_getDoubleInfoValue(self.0, c(k).as_ptr(), &mut v) };
        (st != STATUS_ERROR && v.is_finite()).then_some(v)
    }
    fn info_i(&self, k: &str) -> i64 {
        let mut v: I = 0;
        let st = unsafe { Highs_getIntInfoValue(self.0, c(k).as_ptr(), &mut v) };
        if st == STATUS_ERROR {
            let mut w: i64 = 0;
            unsafe { Highs_getInt64InfoValue(self.0, c(k).as_ptr(), &mut w) };
            return w;
        }
        v as i64
    }

    /// Pasa el modelo (columnas dispersas) y los nombres.
    fn pass(&self, m: &Model) -> Result<(), String> {
        let n = m.cols.len();
        let nr = m.rows.len();
        // filas → columnas (formato CSC)
        let mut counts = vec![0usize; n];
        for r in &m.rows {
            for &(j, a) in &r.coefs {
                if a != 0.0 {
                    if j >= n {
                        return Err(format!("la restricción {} usa una variable que no existe", r.name));
                    }
                    counts[j] += 1;
                }
            }
        }
        let nnz: usize = counts.iter().sum();
        if nnz > I::MAX as usize || n > I::MAX as usize {
            return Err("el modelo es demasiado grande para HiGHS de 32 bits".into());
        }
        let mut start = vec![0 as I; n + 1];
        for j in 0..n {
            start[j + 1] = start[j] + counts[j] as I;
        }
        let mut fill: Vec<usize> = start[..n].iter().map(|&s| s as usize).collect();
        let mut index = vec![0 as I; nnz];
        let mut value = vec![0.0; nnz];
        for (i, r) in m.rows.iter().enumerate() {
            for &(j, a) in &r.coefs {
                if a != 0.0 {
                    let k = fill[j];
                    index[k] = i as I;
                    value[k] = a;
                    fill[j] += 1;
                }
            }
        }
        let inf = unsafe { Highs_getInfinity(self.0) };
        let clip = |x: f64| if x >= inf { inf } else if x <= -inf { -inf } else { x };
        let cost: Vec<f64> = m.cols.iter().map(|c| c.cost).collect();
        let (lower, upper): (Vec<f64>, Vec<f64>) = m
            .cols
            .iter()
            .map(|c| match c.kind {
                Kind::Binary => (c.lb.max(0.0), c.ub.min(1.0)),
                _ => (clip(c.lb), clip(c.ub)),
            })
            .unzip();
        let rlo: Vec<f64> = m.rows.iter().map(|r| clip(r.lo)).collect();
        let rhi: Vec<f64> = m.rows.iter().map(|r| clip(r.hi)).collect();
        let integrality: Vec<I> = m
            .cols
            .iter()
            .map(|c| match c.kind {
                Kind::Continuous => VAR_CONTINUOUS,
                Kind::Integer | Kind::Binary => VAR_INTEGER,
                Kind::SemiContinuous => VAR_SEMI_CONTINUOUS,
                Kind::SemiInteger => VAR_SEMI_INTEGER,
            })
            .collect();
        // Hessiano (triángulo inferior, por columnas)
        let mut q: Vec<(usize, usize, f64)> = vec![];
        for &(i, j, v) in &m.q {
            let (r, cc) = if i >= j { (i, j) } else { (j, i) };
            if v != 0.0 {
                q.push((r, cc, v));
            }
        }
        q.sort_by(|a, b| (a.1, a.0).cmp(&(b.1, b.0)));
        let mut q_start = vec![0 as I; n + 1];
        for &(_, cc, _) in &q {
            q_start[cc + 1] += 1;
        }
        for j in 0..n {
            q_start[j + 1] += q_start[j];
        }
        let q_index: Vec<I> = q.iter().map(|x| x.0 as I).collect();
        let q_value: Vec<f64> = q.iter().map(|x| x.2).collect();
        let has_q = !q.is_empty();
        let st = unsafe {
            Highs_passModel(
                self.0,
                n as I,
                nr as I,
                nnz as I,
                q.len() as I,
                FORMAT_COLWISE,
                HESSIAN_TRIANGULAR,
                if m.maximize { SENSE_MAX } else { SENSE_MIN },
                m.offset,
                cost.as_ptr(),
                lower.as_ptr(),
                upper.as_ptr(),
                rlo.as_ptr(),
                rhi.as_ptr(),
                start.as_ptr(),
                index.as_ptr(),
                value.as_ptr(),
                if has_q { q_start.as_ptr() } else { std::ptr::null() },
                if has_q { q_index.as_ptr() } else { std::ptr::null() },
                if has_q { q_value.as_ptr() } else { std::ptr::null() },
                integrality.as_ptr(),
            )
        };
        if st == STATUS_ERROR {
            return Err("HiGHS no aceptó el modelo (revisa cotas y coeficientes)".into());
        }
        for (j, col) in m.cols.iter().enumerate() {
            if !col.name.is_empty() {
                unsafe { Highs_passColName(self.0, j as I, c(&col.name).as_ptr()) };
            }
        }
        for (i, r) in m.rows.iter().enumerate() {
            if !r.name.is_empty() {
                unsafe { Highs_passRowName(self.0, i as I, c(&r.name).as_ptr()) };
            }
        }
        Ok(())
    }

    fn params(&self, p: &Params) {
        if let Some(t) = p.time_limit.filter(|t| *t > 0.0) {
            self.dbl("time_limit", t);
        }
        if let Some(g) = p.mip_gap.filter(|g| *g >= 0.0) {
            self.dbl("mip_rel_gap", g);
        }
        if let Some(g) = p.mip_abs_gap.filter(|g| *g >= 0.0) {
            self.dbl("mip_abs_gap", g);
        }
        if let Some(t) = p.threads.filter(|t| *t > 0) {
            self.int("threads", t as i64);
        }
        if let Some(pre) = p.presolve {
            self.string("presolve", if pre { "on" } else { "off" });
        }
        match p.method.unwrap_or_default() {
            Method::Auto => {}
            Method::DualSimplex => {
                self.string("solver", "simplex");
                self.int("simplex_strategy", 1);
            }
            Method::PrimalSimplex => {
                self.string("solver", "simplex");
                self.int("simplex_strategy", 4);
            }
            Method::Barrier => self.string("solver", "ipm"),
            Method::Pdlp => self.string("solver", "pdlp"),
        }
        if let Some(s) = p.seed {
            self.int("random_seed", s);
        }
        if let Some(n) = p.node_limit.filter(|n| *n > 0) {
            self.int("mip_max_nodes", n);
        }
        if let Some(n) = p.solution_limit.filter(|n| *n > 0) {
            self.int("mip_max_improving_sols", n);
        }
        if let Some(t) = p.feasibility_tol.filter(|t| *t > 0.0) {
            self.dbl("primal_feasibility_tolerance", t);
            self.dbl("mip_feasibility_tolerance", t);
        }
        if let Some(t) = p.integrality_tol.filter(|t| *t > 0.0) {
            self.dbl("mip_feasibility_tolerance", t);
        }
        // opciones propias de HiGHS, con el tipo que HiGHS espera
        for (k, v) in &p.raw {
            let mut ty: I = -1;
            let st = unsafe { Highs_getOptionType(self.0, c(k).as_ptr(), &mut ty) };
            if st == STATUS_ERROR {
                continue;
            }
            match (ty, v) {
                (0, serde_json::Value::Bool(b)) => self.bool(k, *b),
                (0, v) => self.bool(k, v.as_f64().is_some_and(|x| x != 0.0) || v.as_str().is_some_and(|s| matches!(s, "true" | "on" | "1"))),
                (1, v) => {
                    if let Some(x) = v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())) {
                        self.int(k, x.round() as i64)
                    }
                }
                (2, v) => {
                    if let Some(x) = v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())) {
                        self.dbl(k, x)
                    }
                }
                (3, v) => self.string(k, &v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())),
                _ => {}
            }
        }
    }
}

pub fn version() -> String {
    let v = unsafe { CStr::from_ptr(Highs_version()) };
    format!("HiGHS {}", v.to_string_lossy())
}

fn status_of(code: I) -> Status {
    match code {
        7 => Status::Optimal,
        8 => Status::Infeasible,
        9 => Status::InfeasibleOrUnbounded,
        10 => Status::Unbounded,
        11 | 12 => Status::Optimal,
        13 => Status::TimeLimit,
        14 => Status::IterationLimit,
        16 => Status::SolutionLimit,
        17 => Status::Interrupted,
        _ => Status::Error,
    }
}

fn basis_of(v: I) -> BasisStatus {
    match v {
        0 => BasisStatus::Lower,
        1 => BasisStatus::Basic,
        2 => BasisStatus::Upper,
        3 => BasisStatus::Zero,
        _ => BasisStatus::Nonbasic,
    }
}

pub fn solve(m: &Model, p: &Params) -> Solution {
    let t0 = Instant::now();
    if m.cols.is_empty() {
        let mut s = Solution::empty(m, Status::Optimal, &version(), "");
        s.has_solution = true;
        s.objective = Some(m.offset);
        // filas sin variables: 0 debe estar entre sus cotas
        if m.rows.iter().any(|r| r.lo > 1e-9 || r.hi < -1e-9) {
            s.status = Status::Infeasible;
            s.has_solution = false;
            s.objective = None;
        }
        return s;
    }
    let h = H::new();
    if let Err(e) = h.pass(m) {
        return Solution::empty(m, Status::Error, &version(), e);
    }
    h.params(p);
    let log_path = p.log.then(|| temp_path("log"));
    if let Some(lp) = &log_path {
        h.bool("output_flag", true);
        h.bool("log_to_console", false);
        h.string("log_file", &lp.to_string_lossy());
    }
    if let Some(start) = &p.start {
        let idx: std::collections::HashMap<&str, usize> = m.cols.iter().enumerate().map(|(j, c)| (c.name.as_str(), j)).collect();
        let (ix, vx): (Vec<I>, Vec<f64>) = start.iter().filter_map(|(k, v)| idx.get(k.as_str()).map(|&j| (j as I, *v))).unzip();
        if !ix.is_empty() {
            unsafe { Highs_setSparseSolution(h.0, ix.len() as I, ix.as_ptr(), vx.as_ptr()) };
        }
    }
    let run = unsafe { Highs_run(h.0) };
    let code = unsafe { Highs_getModelStatus(h.0) };
    let mut status = if run == STATUS_ERROR && code == 0 { Status::Error } else { status_of(code) };
    let n = m.cols.len();
    let nr = m.rows.len();
    let primal_ok = h.info_i("primal_solution_status") == 2;
    let mut x = vec![0.0; n];
    let mut cd = vec![0.0; n];
    let mut rv = vec![0.0; nr];
    let mut rd = vec![0.0; nr];
    unsafe { Highs_getSolution(h.0, x.as_mut_ptr(), cd.as_mut_ptr(), rv.as_mut_ptr(), rd.as_mut_ptr()) };
    let mip = m.is_mip();
    let has_solution = primal_ok && !matches!(status, Status::Infeasible | Status::Error);
    if status == Status::Error && primal_ok {
        status = Status::Interrupted;
    }
    let dual_ok = !mip && m.q.is_empty() && status == Status::Optimal && h.info_i("dual_solution_status") == 2;
    let (cb, rb) = if dual_ok {
        let mut cs = vec![0 as I; n];
        let mut rs = vec![0 as I; nr];
        let st = unsafe { Highs_getBasis(h.0, cs.as_mut_ptr(), rs.as_mut_ptr()) };
        if st == STATUS_ERROR {
            (None, None)
        } else {
            (Some(cs.into_iter().map(basis_of).collect()), Some(rs.into_iter().map(basis_of).collect()))
        }
    } else {
        (None, None)
    };
    let objective = has_solution.then(|| unsafe { Highs_getObjectiveValue(h.0) });
    let (bound, gap) = if mip {
        (h.info_f("mip_dual_bound"), h.info_f("mip_gap").filter(|g| *g < 1e30))
    } else if status == Status::Optimal {
        (objective, Some(0.0))
    } else {
        (None, None)
    };
    let iterations = h.info_i("simplex_iteration_count").max(0) + h.info_i("ipm_iteration_count").max(0) + h.info_i("pdlp_iteration_count").max(0);
    let nodes = if mip { h.info_i("mip_node_count").max(0) } else { 0 };
    drop(h);
    let log = log_path.and_then(|lp| {
        let t = std::fs::read_to_string(&lp).ok();
        let _ = std::fs::remove_file(&lp);
        t
    });
    let message = match status {
        Status::TimeLimit if has_solution => "se acabó el tiempo; es la mejor solución encontrada".into(),
        Status::TimeLimit => "se acabó el tiempo sin encontrar una solución factible".into(),
        Status::InfeasibleOrUnbounded => "el modelo no es factible o no está acotado".into(),
        _ => String::new(),
    };
    Solution {
        status,
        has_solution,
        objective,
        bound,
        gap,
        x: if has_solution { x } else { vec![0.0; n] },
        row_activity: if has_solution { rv } else { vec![0.0; nr] },
        row_dual: dual_ok.then_some(rd),
        reduced_cost: dual_ok.then_some(cd),
        col_basis: cb,
        row_basis: rb,
        iterations,
        nodes,
        time: t0.elapsed().as_secs_f64(),
        solver: version(),
        is_mip: mip,
        cols: n,
        rows: nr,
        nonzeros: m.nonzeros(),
        objective_values: vec![],
        log,
        message,
    }
}

pub fn ranging(m: &Model, p: &Params) -> Option<Ranging> {
    let h = H::new();
    h.pass(m).ok()?;
    h.params(p);
    unsafe { Highs_run(h.0) };
    if status_of(unsafe { Highs_getModelStatus(h.0) }) != Status::Optimal {
        return None;
    }
    let n = m.cols.len();
    let nr = m.rows.len();
    let v = || vec![0.0f64; n];
    let (mut cuv, mut cuo, mut cdv, mut cdo) = (v(), v(), v(), v());
    let (mut buv, mut buo, mut bdv, mut bdo) = (v(), v(), v(), v());
    let iv = || vec![0 as I; n];
    let (mut cui, mut cuu, mut cdi, mut cdu, mut bui, mut buu, mut bdi, mut bdu) = (iv(), iv(), iv(), iv(), iv(), iv(), iv(), iv());
    let (mut ruv, mut ruo, mut rdv, mut rdo) = (vec![0.0; nr], vec![0.0; nr], vec![0.0; nr], vec![0.0; nr]);
    let (mut rui, mut ruu, mut rdi, mut rdu) = (vec![0 as I; nr], vec![0 as I; nr], vec![0 as I; nr], vec![0 as I; nr]);
    let st = unsafe {
        Highs_getRanging(
            h.0,
            cuv.as_mut_ptr(),
            cuo.as_mut_ptr(),
            cui.as_mut_ptr(),
            cuu.as_mut_ptr(),
            cdv.as_mut_ptr(),
            cdo.as_mut_ptr(),
            cdi.as_mut_ptr(),
            cdu.as_mut_ptr(),
            buv.as_mut_ptr(),
            buo.as_mut_ptr(),
            bui.as_mut_ptr(),
            buu.as_mut_ptr(),
            bdv.as_mut_ptr(),
            bdo.as_mut_ptr(),
            bdi.as_mut_ptr(),
            bdu.as_mut_ptr(),
            ruv.as_mut_ptr(),
            ruo.as_mut_ptr(),
            rui.as_mut_ptr(),
            ruu.as_mut_ptr(),
            rdv.as_mut_ptr(),
            rdo.as_mut_ptr(),
            rdi.as_mut_ptr(),
            rdu.as_mut_ptr(),
        )
    };
    if st == STATUS_ERROR {
        return None;
    }
    let inf = unsafe { Highs_getInfinity(h.0) };
    let norm = |x: f64| if x >= inf { f64::INFINITY } else if x <= -inf { f64::NEG_INFINITY } else { x };
    let cost = (0..n).map(|j| Range { low: norm(cdv[j]), high: norm(cuv[j]) }).collect();
    let rhs = (0..nr).map(|i| Range { low: norm(rdv[i]), high: norm(ruv[i]) }).collect();
    Some(Ranging { cost, rhs })
}

fn iis_bound(b: I) -> IisBound {
    match b {
        IIS_BOUND_LOWER => IisBound::Lower,
        IIS_BOUND_UPPER => IisBound::Upper,
        IIS_BOUND_FREE => IisBound::Free,
        _ => IisBound::Both,
    }
}

pub fn iis(m: &Model) -> Option<Iis> {
    let h = H::new();
    h.pass(m).ok()?;
    // IIS irreducible, dando prioridad a quitar restricciones antes que cotas
    h.int("iis_strategy", 6);
    unsafe { Highs_run(h.0) };
    let (mut nc, mut nr): (I, I) = (0, 0);
    let null_i: *mut I = std::ptr::null_mut();
    let st = unsafe { Highs_getIis(h.0, &mut nc, &mut nr, null_i, null_i, null_i, null_i, null_i, null_i) };
    if st == STATUS_ERROR || (nc == 0 && nr == 0) {
        return None;
    }
    let (n, mr) = (m.cols.len(), m.rows.len());
    let mut ci = vec![0 as I; n.max(1)];
    let mut ri = vec![0 as I; mr.max(1)];
    let mut cb = vec![0 as I; n.max(1)];
    let mut rb = vec![0 as I; mr.max(1)];
    let mut cs = vec![0 as I; n.max(1)];
    let mut rs = vec![0 as I; mr.max(1)];
    let st = unsafe { Highs_getIis(h.0, &mut nc, &mut nr, ci.as_mut_ptr(), ri.as_mut_ptr(), cb.as_mut_ptr(), rb.as_mut_ptr(), cs.as_mut_ptr(), rs.as_mut_ptr()) };
    if st == STATUS_ERROR {
        return None;
    }
    Some(Iis {
        rows: (0..nr as usize).map(|k| (ri[k] as usize, iis_bound(rb[k]))).collect(),
        cols: (0..nc as usize).map(|k| (ci[k] as usize, iis_bound(cb[k]))).collect(),
    })
}

pub fn feasibility_relaxation(m: &Model, rhs_penalty: f64, bound_penalty: f64) -> Option<Relaxation> {
    let h = H::new();
    h.pass(m).ok()?;
    let st = unsafe { Highs_feasibilityRelaxation(h.0, bound_penalty, bound_penalty, rhs_penalty, std::ptr::null(), std::ptr::null(), std::ptr::null()) };
    if st == STATUS_ERROR {
        return None;
    }
    let n = m.cols.len();
    let nr = m.rows.len();
    let mut x = vec![0.0; n];
    let mut cd = vec![0.0; n];
    let mut rv = vec![0.0; nr];
    let mut rd = vec![0.0; nr];
    unsafe { Highs_getSolution(h.0, x.as_mut_ptr(), cd.as_mut_ptr(), rv.as_mut_ptr(), rd.as_mut_ptr()) };
    let act = m.activity(&x);
    let tol = 1e-7;
    let mut violations = vec![];
    let mut total = 0.0;
    for (i, r) in m.rows.iter().enumerate() {
        let (a, side) = if act[i] < r.lo - tol { (r.lo - act[i], "lower") } else if act[i] > r.hi + tol { (act[i] - r.hi, "upper") } else { continue };
        total += rhs_penalty.max(0.0) * a;
        violations.push(Violation { kind: "row".into(), index: i, name: r.name.clone(), amount: a, side: side.into() });
    }
    for (j, col) in m.cols.iter().enumerate() {
        let (a, side) = if x[j] < col.lb - tol { (col.lb - x[j], "lower") } else if x[j] > col.ub + tol { (x[j] - col.ub, "upper") } else { continue };
        total += bound_penalty.max(0.0) * a;
        violations.push(Violation { kind: "col".into(), index: j, name: col.name.clone(), amount: a, side: side.into() });
    }
    Some(Relaxation { total, violations, x })
}

pub fn read_text(text: &str, format: Format) -> Result<Model, String> {
    let path = temp_path(format.ext());
    std::fs::write(&path, text).map_err(|e| format!("no se pudo preparar el archivo: {e}"))?;
    let h = H::new();
    let st = unsafe { Highs_readModel(h.0, c(&path.to_string_lossy()).as_ptr()) };
    let _ = std::fs::remove_file(&path);
    if st == STATUS_ERROR {
        return Err(format!("el texto no es un modelo {} válido", format.ext().to_uppercase()));
    }
    get_model(&h)
}

fn get_model(h: &H) -> Result<Model, String> {
    let n = unsafe { Highs_getNumCol(h.0) } as usize;
    let nr = unsafe { Highs_getNumRow(h.0) } as usize;
    let nnz = unsafe { Highs_getNumNz(h.0) } as usize;
    let qnz = unsafe { Highs_getHessianNumNz(h.0) }.max(0) as usize;
    let (mut gn, mut gr, mut gz, mut gq, mut sense): (I, I, I, I, I) = (0, 0, 0, 0, 0);
    let mut offset = 0.0;
    let mut cost = vec![0.0; n];
    let mut cl = vec![0.0; n];
    let mut cu = vec![0.0; n];
    let mut rl = vec![0.0; nr];
    let mut ru = vec![0.0; nr];
    let mut start = vec![0 as I; n + 1];
    let mut index = vec![0 as I; nnz.max(1)];
    let mut value = vec![0.0; nnz.max(1)];
    let mut qs = vec![0 as I; n + 1];
    let mut qi = vec![0 as I; qnz.max(1)];
    let mut qv = vec![0.0; qnz.max(1)];
    let mut integ = vec![0 as I; n];
    let st = unsafe {
        Highs_getModel(
            h.0,
            FORMAT_COLWISE,
            HESSIAN_TRIANGULAR,
            &mut gn,
            &mut gr,
            &mut gz,
            &mut gq,
            &mut sense,
            &mut offset,
            cost.as_mut_ptr(),
            cl.as_mut_ptr(),
            cu.as_mut_ptr(),
            rl.as_mut_ptr(),
            ru.as_mut_ptr(),
            start.as_mut_ptr(),
            index.as_mut_ptr(),
            value.as_mut_ptr(),
            qs.as_mut_ptr(),
            qi.as_mut_ptr(),
            qv.as_mut_ptr(),
            integ.as_mut_ptr(),
        )
    };
    if st == STATUS_ERROR {
        return Err("no se pudo leer el modelo".into());
    }
    let inf = unsafe { Highs_getInfinity(h.0) };
    let norm = |x: f64| if x >= inf { f64::INFINITY } else if x <= -inf { f64::NEG_INFINITY } else { x };
    let name = |col: bool, k: usize| -> String {
        let mut buf = vec![0u8; NAME_LEN + 1];
        let st = unsafe {
            if col {
                Highs_getColName(h.0, k as I, buf.as_mut_ptr() as *mut _)
            } else {
                Highs_getRowName(h.0, k as I, buf.as_mut_ptr() as *mut _)
            }
        };
        if st == STATUS_ERROR {
            return format!("{}{}", if col { "x" } else { "c" }, k + 1);
        }
        let end = buf.iter().position(|&b| b == 0).unwrap_or(0);
        let s = String::from_utf8_lossy(&buf[..end]).into_owned();
        if s.is_empty() {
            format!("{}{}", if col { "x" } else { "c" }, k + 1)
        } else {
            s
        }
    };
    let mut m = Model { maximize: sense == SENSE_MAX, offset, ..Default::default() };
    for j in 0..n {
        let kind = match integ.get(j).copied().unwrap_or(0) {
            VAR_INTEGER if cl[j] >= 0.0 && cu[j] <= 1.0 && cl[j] > -1.0 => Kind::Binary,
            VAR_INTEGER => Kind::Integer,
            VAR_SEMI_CONTINUOUS => Kind::SemiContinuous,
            VAR_SEMI_INTEGER => Kind::SemiInteger,
            _ => Kind::Continuous,
        };
        m.cols.push(Col { name: name(true, j), cost: cost[j], lb: norm(cl[j]), ub: norm(cu[j]), kind });
    }
    let mut rows: Vec<Vec<(usize, f64)>> = vec![vec![]; nr];
    // HiGHS da n inicios de columna; el fin de la última es el total de no ceros
    let gz = gz.max(0) as usize;
    let end = |s: &[I], j: usize, total: usize| if j + 1 < n { s[j + 1] as usize } else { total };
    for j in 0..n {
        for k in start[j] as usize..end(&start, j, gz) {
            rows[index[k] as usize].push((j, value[k]));
        }
    }
    for (i, coefs) in rows.into_iter().enumerate() {
        m.rows.push(Row { name: name(false, i), coefs, lo: norm(rl[i]), hi: norm(ru[i]) });
    }
    let gq = gq.max(0) as usize;
    if gq > 0 {
        for j in 0..n {
            for k in qs[j] as usize..end(&qs, j, gq) {
                m.q.push((qi[k] as usize, j, qv[k]));
            }
        }
    }
    Ok(m)
}

pub fn write_text(m: &Model, format: Format) -> Result<String, String> {
    let mut safe = m.clone();
    // en el formato LP los corchetes no son nombres válidos para HiGHS: x[P1,C1] → x_P1_C1
    let lp = format == Format::Lp;
    let tidy = |n: &str| -> String {
        if !lp {
            return n.to_string();
        }
        let mut t: String = n.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
        while t.contains("__") {
            t = t.replace("__", "_");
        }
        t.trim_end_matches('_').to_string()
    };
    let cn: Vec<String> = m.cols.iter().map(|c| tidy(&c.name)).collect();
    let rn: Vec<String> = m.rows.iter().map(|r| tidy(&r.name)).collect();
    for (c, nm) in safe.cols.iter_mut().zip(safe_names(cn.iter().map(|s| s.as_str()), "x")) {
        c.name = nm;
    }
    for (r, nm) in safe.rows.iter_mut().zip(safe_names(rn.iter().map(|s| s.as_str()), "c")) {
        r.name = nm;
    }
    if safe.name.trim().is_empty() {
        safe.name = "IAQSB".into();
    }
    let h = H::new();
    h.pass(&safe)?;
    let path = temp_path(format.ext());
    let st = unsafe { Highs_writeModel(h.0, c(&path.to_string_lossy()).as_ptr()) };
    let out = std::fs::read_to_string(&path);
    let _ = std::fs::remove_file(&path);
    if st == STATUS_ERROR {
        return Err("no se pudo escribir el modelo".into());
    }
    out.map_err(|e| e.to_string())
}
