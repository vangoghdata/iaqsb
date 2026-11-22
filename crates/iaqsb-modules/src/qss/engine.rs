//! Motor de simulación por eventos discretos de QSS: poblaciones de clientes (C),
//! servidores (S), colas (Q) y recolectores de desechos (G), con las reglas de
//! entrada y salida, disciplinas de cola, bloqueo, lotes, tiempos de traslado,
//! ensamble y desensamble de WinQSB.

use super::{Discipline, InRule, Kind, OutRule};
use crate::pert::dist::Law;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Una conexión: destino, probabilidad (si se dio) y tiempo de traslado.
#[derive(Clone, Debug)]
pub struct Link {
    pub to: usize,
    pub prob: Option<f64>,
    pub transfer: f64,
}

/// Componente ya validado.
#[derive(Clone)]
pub struct Node {
    pub name: String,
    pub kind: Kind,
    pub follow: Vec<Link>,
    /// Componentes que conectan hacia este (para los servidores: sus colas).
    pub preds: Vec<usize>,
    pub in_rule: InRule,
    pub out_rule: OutRule,
    pub disc: Discipline,
    /// Capacidad de la cola (None = ilimitada).
    pub cap: Option<usize>,
    pub attr: f64,
    pub ia: Option<Law>,
    pub batch: Option<Law>,
    /// Tiempo de servicio por tipo de cliente (None = cualquier tipo).
    pub service: Vec<(Option<String>, Law)>,
}

impl Node {
    /// Distribución del servicio para un tipo de cliente.
    pub fn law_for(&self, ctype: &str) -> Option<&Law> {
        self.service
            .iter()
            .find(|(t, _)| t.as_deref().map(|t| t.eq_ignore_ascii_case(ctype)).unwrap_or(false))
            .or_else(|| self.service.iter().find(|(t, _)| t.is_none()))
            .or_else(|| self.service.first())
            .map(|(_, l)| l)
    }
}

#[derive(Clone, Copy, Debug)]
struct Cust {
    id: u64,
    /// Población de origen (índice del componente C).
    ctype: usize,
    attr: f64,
    born: f64,
    work: f64,
    wait: f64,
    /// Entrada a la cola actual.
    q_in: f64,
}

#[derive(Clone, Copy)]
enum Srv {
    Idle,
    Busy(Cust, f64),
    Blocked(Cust, f64),
}

#[derive(PartialEq)]
struct Ev {
    t: f64,
    seq: u64,
    kind: u8, // 0 llegada de población, 1 fin de servicio, 2 llega un traslado
    id: usize,
    cust: Option<CustKey>,
}
#[derive(PartialEq, Clone, Copy)]
struct CustKey(u64);
impl Eq for Ev {}
impl PartialOrd for Ev {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Ev {
    fn cmp(&self, o: &Self) -> Ordering {
        o.t.partial_cmp(&self.t).unwrap_or(Ordering::Equal).then(o.seq.cmp(&self.seq))
    }
}

/// Estadísticas de un componente.
#[derive(Default, Clone, Debug)]
pub struct Stat {
    /// C: clientes generados; S: atendidos; Q: entradas; G: recolectados.
    pub count: u64,
    /// C: clientes que no pudieron entrar (colas llenas).
    pub lost: u64,
    /// Q: longitud ponderada por el tiempo; S: tiempo ocupado.
    pub area: f64,
    pub blocked_area: f64,
    pub level: usize,
    pub max_level: usize,
    /// Q: espera; S: servicio; G: tiempo en el sistema.
    pub sum_t: f64,
    pub max_t: f64,
    pub min_t: f64,
    pub n_t: u64,
}

/// Estadísticas de los clientes de una población que terminaron.
#[derive(Default, Clone, Debug)]
pub struct CustStat {
    pub arrived: u64,
    pub balked: u64,
    pub finished: u64,
    pub garbage: u64,
    pub sum_sys: f64,
    pub max_sys: f64,
    pub min_sys: f64,
    pub sum_wait: f64,
    pub sum_work: f64,
    pub in_sys_area: f64,
    pub in_sys: i64,
}

pub struct SimOut {
    pub stats: Vec<Stat>,
    pub cust: Vec<CustStat>,
    pub trace: Vec<(f64, String, String, String)>,
    /// Instante en que se detuvo (tiempo simulado o máximo de observaciones).
    pub end: f64,
}

pub struct Sim<'a> {
    nodes: &'a [Node],
    rng: ChaCha8Rng,
    now: f64,
    warmup: f64,
    seq: u64,
    next_id: u64,
    ev: BinaryHeap<Ev>,
    queues: Vec<Vec<Cust>>,
    transit: Vec<usize>,
    in_transit: Vec<(u64, Cust)>,
    srv: Vec<Srv>,
    reserved: Vec<bool>,
    rr: Vec<usize>,
    stats: Vec<Stat>,
    cust: Vec<CustStat>,
    last: f64,
    trace: Vec<(f64, String, String, String)>,
    trace_n: usize,
    done: u64,
}

impl<'a> Sim<'a> {
    fn push(&mut self, t: f64, kind: u8, id: usize, cust: Option<CustKey>) {
        self.seq += 1;
        self.ev.push(Ev { t, seq: self.seq, kind, id, cust });
    }
    fn rec(&self) -> bool {
        self.now >= self.warmup
    }
    fn log(&mut self, what: &str, node: usize, c: &Cust) {
        if self.trace.len() < self.trace_n {
            let label = format!("{} #{}", self.nodes[c.ctype].name, c.id);
            self.trace.push((self.now, what.to_string(), self.nodes[node].name.clone(), label));
        }
    }
    /// Avanza las áreas ponderadas por el tiempo hasta `t`.
    fn advance(&mut self, t: f64) {
        let from = self.last.max(self.warmup);
        if t > from {
            let dt = t - from;
            for (i, n) in self.nodes.iter().enumerate() {
                match n.kind {
                    Kind::Queue => self.stats[i].area += dt * self.queues[i].len() as f64,
                    Kind::Server => match self.srv[i] {
                        Srv::Busy(..) => self.stats[i].area += dt,
                        Srv::Blocked(..) => self.stats[i].blocked_area += dt,
                        Srv::Idle => {}
                    },
                    _ => {}
                }
            }
            for c in self.cust.iter_mut() {
                c.in_sys_area += dt * c.in_sys.max(0) as f64;
            }
        }
        self.last = t;
    }

    fn occupancy(&self, q: usize) -> usize {
        self.queues[q].len() + self.transit[q]
    }
    /// Servidores libres que siguen a la cola `q` y toman clientes de ella.
    fn idle_followers(&self, q: usize) -> usize {
        self.nodes[q].follow.iter().filter(|l| self.nodes[l.to].kind == Kind::Server && matches!(self.srv[l.to], Srv::Idle) && !self.reserved[l.to] && self.nodes[l.to].in_rule != InRule::Assembly).count()
    }
    fn has_space(&self, to: usize) -> bool {
        match self.nodes[to].kind {
            Kind::Queue => {
                let occ = self.occupancy(to);
                match self.nodes[to].cap {
                    None => true,
                    Some(c) => occ < c || occ < c + self.idle_followers(to),
                }
            }
            Kind::Server => matches!(self.srv[to], Srv::Idle) && !self.reserved[to],
            Kind::Garbage => true,
            Kind::Customer => false,
        }
    }

    /// Elige destinos para un cliente que sale de `from`. None = bloqueado (o perdido si es una población).
    fn choose(&mut self, from: usize) -> Option<Vec<usize>> {
        let n = &self.nodes[from];
        let k = n.follow.len();
        let ok: Vec<usize> = (0..k).filter(|&j| self.has_space(n.follow[j].to)).collect();
        if n.out_rule == OutRule::Disassembly {
            return (ok.len() == k).then(|| (0..k).collect());
        }
        if ok.is_empty() {
            return None;
        }
        let len = |s: &Self, j: usize| s.occupancy(s.nodes[from].follow[j].to) as f64;
        let cap = |s: &Self, j: usize| s.nodes[s.nodes[from].follow[j].to].cap.map(|c| c as f64).unwrap_or(f64::INFINITY);
        let pick = match n.out_rule {
            OutRule::Random => ok[self.rng.random_range(0..ok.len())],
            OutRule::Probability => {
                let w: Vec<f64> = ok.iter().map(|&j| n.follow[j].prob.unwrap_or(1.0 / k as f64)).collect();
                let tot: f64 = w.iter().sum();
                if tot <= 0.0 {
                    ok[self.rng.random_range(0..ok.len())]
                } else {
                    let mut r = self.rng.random::<f64>() * tot;
                    let mut pick = *ok.last().unwrap();
                    for (i, &j) in ok.iter().enumerate() {
                        if r < w[i] {
                            pick = j;
                            break;
                        }
                        r -= w[i];
                    }
                    pick
                }
            }
            OutRule::RoundRobin => {
                let start = self.rr[from];
                let j = (0..k).map(|d| (start + d) % k).find(|j| ok.contains(j)).unwrap();
                self.rr[from] = j + 1;
                j
            }
            OutRule::LongestQueue => *ok.iter().max_by(|&&a, &&b| len(self, a).partial_cmp(&len(self, b)).unwrap().then(b.cmp(&a))).unwrap(),
            OutRule::ShortestQueue => *ok.iter().min_by(|&&a, &&b| len(self, a).partial_cmp(&len(self, b)).unwrap().then(a.cmp(&b))).unwrap(),
            OutRule::MaxQueueCapacity => *ok.iter().max_by(|&&a, &&b| cap(self, a).partial_cmp(&cap(self, b)).unwrap().then(b.cmp(&a))).unwrap(),
            OutRule::MinQueueCapacity => *ok.iter().min_by(|&&a, &&b| cap(self, a).partial_cmp(&cap(self, b)).unwrap().then(a.cmp(&b))).unwrap(),
            OutRule::Disassembly => unreachable!(),
        };
        Some(vec![pick])
    }

    /// Manda el cliente al destino (con traslado si corresponde).
    fn send(&mut self, from: usize, j: usize, c: Cust) {
        let l = self.nodes[from].follow[j].clone();
        if l.transfer > 0.0 {
            match self.nodes[l.to].kind {
                Kind::Queue => self.transit[l.to] += 1,
                Kind::Server => self.reserved[l.to] = true,
                _ => {}
            }
            self.in_transit.push((c.id, c));
            self.push(self.now + l.transfer, 2, l.to, Some(CustKey(c.id)));
        } else {
            self.arrive(l.to, c);
        }
    }

    /// Un cliente llega a un componente.
    fn arrive(&mut self, to: usize, mut c: Cust) {
        match self.nodes[to].kind {
            Kind::Queue => {
                c.q_in = self.now;
                self.queues[to].push(c);
                if self.rec() {
                    let s = &mut self.stats[to];
                    s.count += 1;
                }
                let lv = self.queues[to].len();
                self.stats[to].max_level = self.stats[to].max_level.max(lv);
                self.log("entra a la cola", to, &c);
            }
            Kind::Server => self.start(to, c),
            Kind::Garbage => {
                let t = self.now - c.born;
                self.log("va al recolector", to, &c);
                self.cust[c.ctype].in_sys -= 1;
                if self.rec() {
                    let s = &mut self.stats[to];
                    s.count += 1;
                    s.sum_t += t;
                    s.max_t = s.max_t.max(t);
                    s.min_t = if s.n_t == 0 { t } else { s.min_t.min(t) };
                    s.n_t += 1;
                    self.cust[c.ctype].garbage += 1;
                }
            }
            Kind::Customer => {}
        }
    }

    fn start(&mut self, s: usize, c: Cust) {
        let ctype = self.nodes[c.ctype].name.clone();
        let t = self.nodes[s].law_for(&ctype).map(|l| l.sample(&mut self.rng)).unwrap_or(0.0).max(0.0);
        self.srv[s] = Srv::Busy(c, t);
        self.log("inicia servicio", s, &c);
        self.push(self.now + t, 1, s, None);
    }

    /// Sale un cliente de un servidor: lo manda a sus seguidores o sale del sistema.
    fn depart(&mut self, s: usize, c: Cust) -> bool {
        if self.nodes[s].follow.is_empty() {
            let t = self.now - c.born;
            self.log("sale del sistema", s, &c);
            self.cust[c.ctype].in_sys -= 1;
            if self.rec() {
                let cs = &mut self.cust[c.ctype];
                cs.sum_sys += t;
                cs.max_sys = cs.max_sys.max(t);
                cs.min_sys = if cs.finished == 0 { t } else { cs.min_sys.min(t) };
                cs.finished += 1;
                cs.sum_wait += c.wait;
                cs.sum_work += c.work;
                self.done += 1;
            }
            return true;
        }
        match self.choose(s) {
            Some(js) => {
                let multi = js.len() > 1;
                for (k, j) in js.into_iter().enumerate() {
                    let mut c2 = c;
                    if multi && k > 0 {
                        // desensamble: copias del cliente
                        self.next_id += 1;
                        c2.id = self.next_id;
                        self.cust[c.ctype].in_sys += 1;
                    }
                    self.send(s, j, c2);
                }
                true
            }
            None => false,
        }
    }

    /// Elige el cliente de una cola según su disciplina (para el servidor `s`).
    fn take(&mut self, q: usize, s: usize) -> Cust {
        let v = &self.queues[q];
        let mean_t = |c: &Cust| self.nodes[s].law_for(&self.nodes[c.ctype].name).map(|l| l.mean()).unwrap_or(0.0);
        let idx = match self.nodes[q].disc {
            Discipline::Fifo => 0,
            Discipline::Lifo => v.len() - 1,
            Discipline::Random => self.rng.random_range(0..v.len()),
            Discipline::PriorityIndex => argbest(v, |c| c.attr, true),
            Discipline::Spt => argbest(v, mean_t, false),
            Discipline::Lpt => argbest(v, mean_t, true),
            Discipline::MaxWorkDone => argbest(v, |c| c.work, true),
            Discipline::MinWorkDone => argbest(v, |c| c.work, false),
            Discipline::MostWait => {
                let now = self.now;
                argbest(v, |c| c.wait + (now - c.q_in), true)
            }
            Discipline::EarliestEntering => argbest(v, |c| c.born, false),
        };
        let mut c = self.queues[q].remove(idx);
        let w = self.now - c.q_in;
        c.wait += w;
        if self.rec() {
            let st = &mut self.stats[q];
            st.sum_t += w;
            st.max_t = st.max_t.max(w);
            st.min_t = if st.n_t == 0 { w } else { st.min_t.min(w) };
            st.n_t += 1;
        }
        c
    }

    /// Un servidor libre intenta tomar clientes de sus colas.
    fn pull(&mut self, s: usize) -> bool {
        let preds: Vec<usize> = self.nodes[s].preds.iter().copied().filter(|&p| self.nodes[p].kind == Kind::Queue).collect();
        if preds.is_empty() {
            return false;
        }
        let rule = self.nodes[s].in_rule;
        if rule == InRule::Assembly {
            if preds.iter().any(|&q| self.queues[q].is_empty()) {
                return false;
            }
            let parts: Vec<Cust> = preds.iter().map(|&q| self.take(q, s)).collect();
            let mut main = parts[0];
            for p in &parts[1..] {
                main.born = main.born.min(p.born);
                main.work = main.work.max(p.work);
                main.wait = main.wait.max(p.wait);
                // las partes ensambladas dejan de contar como clientes en el sistema
                self.cust[p.ctype].in_sys -= 1;
            }
            self.start(s, main);
            return true;
        }
        let ok: Vec<usize> = preds.iter().copied().filter(|&q| !self.queues[q].is_empty()).collect();
        if ok.is_empty() {
            return false;
        }
        let len = |s: &Self, q: usize| s.queues[q].len();
        let cap = |s: &Self, q: usize| s.nodes[q].cap.map(|c| c as f64).unwrap_or(f64::INFINITY);
        let q = match rule {
            InRule::Random => ok[self.rng.random_range(0..ok.len())],
            InRule::Probability => {
                let w: Vec<f64> = ok.iter().map(|&q| self.nodes[q].follow.iter().find(|l| l.to == s).and_then(|l| l.prob).unwrap_or(1.0)).collect();
                let tot: f64 = w.iter().sum();
                let mut r = self.rng.random::<f64>() * tot;
                let mut pick = *ok.last().unwrap();
                for (i, &q) in ok.iter().enumerate() {
                    if r < w[i] {
                        pick = q;
                        break;
                    }
                    r -= w[i];
                }
                pick
            }
            InRule::RoundRobin => {
                let k = preds.len();
                let start = self.rr[s];
                let i = (0..k).map(|d| (start + d) % k).find(|i| ok.contains(&preds[*i])).unwrap();
                self.rr[s] = i + 1;
                preds[i]
            }
            InRule::LongestQueue => *ok.iter().max_by(|&&a, &&b| len(self, a).cmp(&len(self, b)).then(b.cmp(&a))).unwrap(),
            InRule::ShortestQueue => *ok.iter().min_by(|&&a, &&b| len(self, a).cmp(&len(self, b)).then(a.cmp(&b))).unwrap(),
            InRule::MaxQueueCapacity => *ok.iter().max_by(|&&a, &&b| cap(self, a).partial_cmp(&cap(self, b)).unwrap().then(b.cmp(&a))).unwrap(),
            InRule::MinQueueCapacity => *ok.iter().min_by(|&&a, &&b| cap(self, a).partial_cmp(&cap(self, b)).unwrap().then(a.cmp(&b))).unwrap(),
            InRule::Assembly => unreachable!(),
        };
        let c = self.take(q, s);
        self.start(s, c);
        true
    }

    /// Mueve todo lo que se pueda mover (servidores bloqueados y libres) hasta que nada cambie.
    fn propagate(&mut self) {
        let servers: Vec<usize> = (0..self.nodes.len()).filter(|&i| self.nodes[i].kind == Kind::Server).collect();
        if servers.is_empty() {
            return;
        }
        let mut guard = 0;
        loop {
            guard += 1;
            let mut changed = false;
            let off = self.rng.random_range(0..servers.len());
            // primero los servidores libres toman clientes (así una cola nunca pasa de su capacidad),
            // después los bloqueados intentan pasar su cliente
            for k in 0..servers.len() {
                let s = servers[(k + off) % servers.len()];
                if matches!(self.srv[s], Srv::Idle) && !self.reserved[s] && self.pull(s) {
                    changed = true;
                }
            }
            for k in 0..servers.len() {
                let s = servers[(k + off) % servers.len()];
                if let Srv::Blocked(c, since) = self.srv[s] {
                    self.srv[s] = Srv::Idle;
                    if self.depart(s, c) {
                        changed = true;
                        self.log("se desbloquea", s, &c);
                    } else {
                        self.srv[s] = Srv::Blocked(c, since);
                    }
                }
                if matches!(self.srv[s], Srv::Idle) && !self.reserved[s] && self.pull(s) {
                    changed = true;
                }
            }
            if !changed || guard > 10_000 {
                break;
            }
        }
    }

    fn new_customer(&mut self, src: usize) -> Cust {
        self.next_id += 1;
        Cust { id: self.next_id, ctype: src, attr: self.nodes[src].attr, born: self.now, work: 0.0, wait: 0.0, q_in: self.now }
    }
}

fn argbest<F: Fn(&Cust) -> f64>(v: &[Cust], f: F, max: bool) -> usize {
    let mut best = 0;
    for i in 1..v.len() {
        let (a, b) = (f(&v[i]), f(&v[best]));
        if (max && a > b + 1e-12) || (!max && a < b - 1e-12) {
            best = i;
        }
    }
    best
}

pub fn simulate(nodes: &[Node], horizon: f64, warmup: f64, max_obs: Option<u64>, seed: u64, trace_n: usize) -> SimOut {
    let n = nodes.len();
    let mut sim = Sim {
        nodes,
        rng: ChaCha8Rng::seed_from_u64(seed),
        now: 0.0,
        warmup,
        seq: 0,
        next_id: 0,
        ev: BinaryHeap::new(),
        queues: vec![vec![]; n],
        transit: vec![0; n],
        in_transit: vec![],
        srv: vec![Srv::Idle; n],
        reserved: vec![false; n],
        rr: vec![0; n],
        stats: vec![Stat::default(); n],
        cust: vec![CustStat::default(); n],
        last: 0.0,
        trace: vec![],
        trace_n,
        done: 0,
    };
    for i in 0..n {
        if let Some(l) = &nodes[i].ia {
            let t = l.sample(&mut sim.rng).max(0.0);
            sim.push(t, 0, i, None);
        }
    }
    let mut end = horizon;
    while let Some(e) = sim.ev.pop() {
        if e.t > horizon {
            break;
        }
        sim.advance(e.t);
        sim.now = e.t;
        match e.kind {
            0 => {
                let src = e.id;
                let k = nodes[src].batch.as_ref().map(|b| b.sample(&mut sim.rng).round().max(0.0) as usize).unwrap_or(1);
                for _ in 0..k {
                    let c = sim.new_customer(src);
                    sim.cust[src].in_sys += 1;
                    if sim.rec() {
                        sim.cust[src].arrived += 1;
                        sim.stats[src].count += 1;
                    }
                    sim.log("llega", src, &c);
                    match sim.choose(src) {
                        Some(js) => {
                            let multi = js.len() > 1;
                            for (kk, j) in js.into_iter().enumerate() {
                                let mut c2 = c;
                                if multi && kk > 0 {
                                    sim.next_id += 1;
                                    c2.id = sim.next_id;
                                    sim.cust[src].in_sys += 1;
                                }
                                sim.send(src, j, c2);
                            }
                        }
                        None => {
                            sim.cust[src].in_sys -= 1;
                            sim.log("no entra (colas llenas)", src, &c);
                            if sim.rec() {
                                sim.cust[src].balked += 1;
                                sim.stats[src].lost += 1;
                            }
                        }
                    }
                    sim.propagate();
                }
                let dt = nodes[src].ia.as_ref().unwrap().sample(&mut sim.rng).max(0.0);
                sim.push(sim.now + dt.max(1e-12), 0, src, None);
            }
            1 => {
                let s = e.id;
                if let Srv::Busy(mut c, t) = sim.srv[s] {
                    c.work += t;
                    if sim.rec() {
                        let st = &mut sim.stats[s];
                        st.count += 1;
                        st.sum_t += t;
                        st.max_t = st.max_t.max(t);
                        st.min_t = if st.n_t == 0 { t } else { st.min_t.min(t) };
                        st.n_t += 1;
                    }
                    sim.log("termina servicio", s, &c);
                    sim.srv[s] = Srv::Idle;
                    if !sim.depart(s, c) {
                        sim.srv[s] = Srv::Blocked(c, sim.now);
                        sim.log("queda bloqueado", s, &c);
                    }
                }
                sim.propagate();
            }
            _ => {
                let to = e.id;
                let key = e.cust.unwrap().0;
                if let Some(p) = sim.in_transit.iter().position(|(id, _)| *id == key) {
                    let (_, c) = sim.in_transit.swap_remove(p);
                    match nodes[to].kind {
                        Kind::Queue => sim.transit[to] -= 1,
                        Kind::Server => sim.reserved[to] = false,
                        _ => {}
                    }
                    sim.arrive(to, c);
                }
                sim.propagate();
            }
        }
        if let Some(m) = max_obs {
            if sim.done >= m {
                end = sim.now;
                break;
            }
        }
    }
    sim.advance(end);
    SimOut { stats: sim.stats, cust: sim.cust, trace: sim.trace, end }
}
