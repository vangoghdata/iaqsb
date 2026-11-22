//! Simulación por eventos discretos de una estación de servicio G/G/s con
//! capacidad y población finitas o infinitas (disciplina FIFO), llegadas en
//! lotes, llegadas desalentadas y servicio presionado (como QA de WinQSB).

use crate::pert::dist::Law;
use rand::{RngExt, SeedableRng};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};

#[derive(Debug, Clone, Default)]
pub struct StationOut {
    pub l: f64,
    pub lq: f64,
    pub w: f64,
    pub wq: f64,
    pub util: f64,
    /// Fracción de los clientes que llegan y no entran (sistema lleno o desalentados).
    pub balk: f64,
    pub served: u64,
    pub max_wq: f64,
}

/// Datos de la simulación.
pub struct SimCfg<'a> {
    pub ia: &'a Law,
    pub sv: &'a Law,
    /// Tamaño del lote de llegada (None = siempre 1).
    pub batch: Option<&'a Law>,
    pub s: usize,
    pub qcap: Option<usize>,
    pub pop: Option<usize>,
    /// Coeficiente de presión del servicio (a) y de desaliento de las llegadas (b).
    pub pressure: f64,
    pub discourage: f64,
    pub horizon: f64,
    pub warmup: f64,
    pub seed: u64,
}

#[derive(PartialEq)]
struct Ev {
    t: f64,
    /// 0 = llegada de la fuente `id`; 1 = fin de servicio en el servidor `id`.
    kind: u8,
    id: usize,
    gen: u64,
}
impl Eq for Ev {}
impl PartialOrd for Ev {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Ev {
    fn cmp(&self, o: &Self) -> Ordering {
        // montículo de mínimos por tiempo; a igual tiempo, primero las salidas
        o.t.partial_cmp(&self.t).unwrap_or(Ordering::Equal).then(self.kind.cmp(&o.kind))
    }
}

/// Cliente: (fuente, instante de llegada).
type Cust = (usize, f64);

/// Un servidor ocupado: cliente, trabajo que le falta (en tiempo a velocidad 1) desde `t_ref`.
#[derive(Clone, Copy)]
struct Busy {
    c: Cust,
    work: f64,
    t_ref: f64,
    speed: f64,
}

pub fn simulate_station(cfg: &SimCfg) -> StationOut {
    let SimCfg { ia, sv, batch, s, qcap, pop, pressure, discourage, horizon, warmup, seed } = *cfg;
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
    let mut ev = BinaryHeap::new();
    // con población finita cada cliente genera sus propias llegadas; si no, una sola fuente
    let sources = pop.unwrap_or(1);
    let mut src_gen = vec![0u64; sources];
    let mut in_sys = vec![false; sources];
    for id in 0..sources {
        ev.push(Ev { t: ia.sample(&mut rng), kind: 0, id, gen: 0 });
    }
    let cap = qcap.map(|q| q + s);
    let speed_of = |n: usize| if pressure != 0.0 && n > s { (n as f64 / s as f64).powf(pressure) } else { 1.0 };
    let mut queue: VecDeque<Cust> = VecDeque::new();
    let mut serving: Vec<Option<Busy>> = vec![None; s];
    let mut slot_gen = vec![0u64; s];
    let mut busy = 0usize;
    let (mut a_l, mut a_lq, mut a_b) = (0.0, 0.0, 0.0);
    let mut last = 0.0f64;
    let (mut offered, mut balked, mut served) = (0u64, 0u64, 0u64);
    let (mut sum_w, mut sum_wq, mut n_wq, mut max_wq) = (0.0, 0.0, 0u64, 0.0f64);
    let mut cur_speed = 1.0f64;

    // reprograma las salidas si cambia la velocidad (servicio presionado)
    fn respeed(now: f64, sp: f64, serving: &mut [Option<Busy>], slot_gen: &mut [u64], ev: &mut BinaryHeap<Ev>) {
        for (k, b) in serving.iter_mut().enumerate() {
            if let Some(b) = b {
                b.work = (b.work - (now - b.t_ref) * b.speed).max(0.0);
                b.t_ref = now;
                b.speed = sp;
                slot_gen[k] += 1;
                ev.push(Ev { t: now + b.work / sp, kind: 1, id: k, gen: slot_gen[k] });
            }
        }
    }

    while let Some(e) = ev.pop() {
        // eventos anulados
        if (e.kind == 0 && e.gen != src_gen[e.id]) || (e.kind == 1 && e.gen != slot_gen[e.id]) {
            continue;
        }
        let t = e.t.min(horizon);
        if t > warmup {
            let from = last.max(warmup);
            let dt = t - from;
            if dt > 0.0 {
                a_l += dt * (busy + queue.len()) as f64;
                a_lq += dt * queue.len() as f64;
                a_b += dt * busy as f64;
            }
        }
        last = t;
        if e.t > horizon {
            break;
        }
        let now = e.t;
        let rec = now >= warmup;
        if e.kind == 0 {
            let src = e.id;
            let n = busy + queue.len();
            let k = batch.map(|b| b.sample(&mut rng).round().max(0.0) as usize).unwrap_or(1);
            // con población finita el lote se completa con otros clientes que estén fuera
            let mut members = vec![src];
            if pop.is_some() {
                in_sys[src] = true;
                for j in 0..sources {
                    if members.len() >= k {
                        break;
                    }
                    if !in_sys[j] {
                        in_sys[j] = true;
                        src_gen[j] += 1; // su próxima llegada queda anulada
                        members.push(j);
                    }
                }
                if k == 0 {
                    members.clear();
                    in_sys[src] = false;
                    ev.push(Ev { t: now + ia.sample(&mut rng), kind: 0, id: src, gen: src_gen[src] });
                }
            } else {
                members = vec![0; k];
                ev.push(Ev { t: now + ia.sample(&mut rng), kind: 0, id: src, gen: src_gen[src] });
            }
            if rec {
                offered += members.len() as u64;
            }
            // llegadas desalentadas: con todos los servidores ocupados el lote entra con probabilidad (s/(n+1))^b
            let discouraged = discourage != 0.0 && n >= s && !members.is_empty() && rng.random::<f64>() > (s as f64 / (n as f64 + 1.0)).powf(discourage);
            for &m in members.iter() {
                let full = cap.map(|c| busy + queue.len() >= c).unwrap_or(false);
                if discouraged || full {
                    if rec {
                        balked += 1;
                    }
                    if pop.is_some() {
                        in_sys[m] = false;
                        ev.push(Ev { t: now + ia.sample(&mut rng), kind: 0, id: m, gen: src_gen[m] });
                    }
                } else if busy < s {
                    let slot = serving.iter().position(|x| x.is_none()).unwrap();
                    busy += 1;
                    if rec {
                        n_wq += 1;
                    }
                    let work = sv.sample(&mut rng);
                    serving[slot] = Some(Busy { c: (m, now), work, t_ref: now, speed: cur_speed });
                    slot_gen[slot] += 1;
                    ev.push(Ev { t: now + work / cur_speed, kind: 1, id: slot, gen: slot_gen[slot] });
                } else {
                    queue.push_back((m, now));
                }
            }
        } else {
            let slot = e.id;
            let b = serving[slot].take().unwrap();
            let (src, t0) = b.c;
            busy -= 1;
            if t0 >= warmup {
                served += 1;
                sum_w += now - t0;
            }
            if pop.is_some() {
                in_sys[src] = false;
                ev.push(Ev { t: now + ia.sample(&mut rng), kind: 0, id: src, gen: src_gen[src] });
            }
            if let Some(c) = queue.pop_front() {
                busy += 1;
                if c.1 >= warmup {
                    let wq = now - c.1;
                    sum_wq += wq;
                    n_wq += 1;
                    max_wq = max_wq.max(wq);
                }
                let work = sv.sample(&mut rng);
                serving[slot] = Some(Busy { c, work, t_ref: now, speed: cur_speed });
                slot_gen[slot] += 1;
                ev.push(Ev { t: now + work / cur_speed, kind: 1, id: slot, gen: slot_gen[slot] });
            }
        }
        let sp = speed_of(busy + queue.len());
        if (sp - cur_speed).abs() > 1e-15 {
            cur_speed = sp;
            respeed(now, sp, &mut serving, &mut slot_gen, &mut ev);
        }
    }
    let span = (horizon - warmup).max(1e-12);
    StationOut {
        l: a_l / span,
        lq: a_lq / span,
        w: if served > 0 { sum_w / served as f64 } else { 0.0 },
        wq: if n_wq > 0 { sum_wq / n_wq as f64 } else { 0.0 },
        util: a_b / span / s as f64,
        balk: if offered > 0 { balked as f64 / offered as f64 } else { 0.0 },
        served,
        max_wq,
    }
}
