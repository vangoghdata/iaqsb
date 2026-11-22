//! Programa maestro: los ejemplos resueltos del curso (Arango Marín, 2020),
//! el disponible para promesa, las solicitudes de pedidos y el PMP óptimo
//! comparado con la enumeración de todos los planes posibles.

use iaqsb_core::Module;
use iaqsb_modules::pmp::Pmp;
use serde_json::{json, Value};

fn ex(name: &str) -> Value {
    Module::examples(&Pmp).into_iter().find(|(n, _)| n == name).unwrap().1
}

fn col(r: &iaqsb_core::Report, key: &str) -> Vec<Option<f64>> {
    r.data["periods"].as_array().unwrap().iter().map(|p| p[key].as_f64()).collect()
}

fn nums(r: &iaqsb_core::Report, key: &str) -> Vec<f64> {
    col(r, key).into_iter().map(|x| x.unwrap()).collect()
}

#[test]
fn basico_de_la_diapositiva() {
    let r = Module::solve(&Pmp, &ex("basico_arango"), &Value::Null).unwrap();
    assert_eq!(nums(&r, "mps"), [0., 80., 0., 80., 80., 0., 80., 0., 80., 80.]);
    assert_eq!(nums(&r, "projected_available"), [30., 60., 15., 45., 75., 25., 55., 5., 35., 65.]);
    assert_eq!(r.data["lot_periods"], json!(["2", "4", "5", "7", "9", "10"]));
}

#[test]
fn ato_igual_a_la_hoja_resuelta() {
    let r = Module::solve(&Pmp, &ex("ato_arango"), &Value::Null).unwrap();
    // «Ejemplo p mae ato resuelto.xlsx», columnas D, E y F
    assert_eq!(nums(&r, "projected_available"), [32., 9., 49., 27., 9., 51., 19., 49., 21., 53., 24., 1.]);
    assert_eq!(nums(&r, "mps"), [0., 0., 60., 0., 0., 60., 0., 60., 0., 60., 0., 0.]);
    let dpp: Vec<f64> = col(&r, "atp").into_iter().flatten().collect();
    assert_eq!(dpp, [9., 6., 29., 32., 20.]);
    // dentro de la barrera de demanda solo cuentan los pedidos
    assert_eq!(nums(&r, "demand")[..3], [24., 23., 20.]);
}

#[test]
fn producto_a_y_solicitudes() {
    let r = Module::solve(&Pmp, &ex("ato_solicitudes"), &Value::Null).unwrap();
    let dpp: Vec<f64> = col(&r, "atp").into_iter().flatten().collect();
    assert_eq!(dpp, [27., 7., 40., 65., 122., 143., 150.]);
    let s = r.data["requests"].as_array().unwrap();
    assert_eq!(s.len(), 3);
    for (q, max) in s.iter().zip([34.0, 74.0, 139.0]) {
        assert_eq!(q["accepted"], json!(true), "{q}");
        assert_eq!(q["available_to_promise"].as_f64().unwrap(), max);
    }
    // 120 unidades en la semana 7: la demanda de esa semana sube a 154 y la regla adelanta el lote
    assert_eq!(s[2]["mps_changes"], json!(true));
    assert_eq!(s[2]["mps"][6].as_f64().unwrap(), 150.0);
}

#[test]
fn zona_congelada_y_dpp_prestado() {
    // una solicitud grande dentro de la barrera de demanda no se puede aceptar
    let mut m = ex("ato_arango");
    m["requests"] = json!([{"period": "2", "quantity": 40}]);
    let r = Module::solve(&Pmp, &m, &Value::Null).unwrap();
    let q = &r.data["requests"][0];
    assert_eq!(q["decision"], json!("No"));
    assert_eq!(q["available_to_promise"].as_f64().unwrap(), 9.0);
    // pedidos que superan un lote: el DPP negativo se cubre con el del lote anterior
    let mut m = ex("ato_arango");
    m["periods"][6]["orders"] = json!(80);
    let r = Module::solve(&Pmp, &m, &Value::Null).unwrap();
    let dpp = col(&r, "atp");
    assert!(dpp.iter().flatten().all(|x| *x >= 0.0), "{dpp:?}");
    let acum = col(&r, "cumulative_atp");
    let total_lotes: f64 = 56.0 + nums(&r, "mps").iter().sum::<f64>();
    let total_pedidos: f64 = m["periods"].as_array().unwrap().iter().map(|p| p["orders"].as_f64().unwrap()).sum();
    assert_eq!(acum.last().unwrap().unwrap(), total_lotes - total_pedidos);
}

/// Costo de un plan: preparación por periodo con lote + inventario, o None si no es factible.
fn costo(m: &Value, lotes: &[u32], fijo: &[f64]) -> Option<f64> {
    let lot = m["lot_size"].as_f64().unwrap();
    let ss = m["safety_stock"].as_f64().unwrap();
    let h = m["hours_per_unit"].as_f64().unwrap();
    let dtf = m["demand_fence"].as_u64().unwrap() as usize;
    let per = m["periods"].as_array().unwrap();
    let mut inv = m["on_hand"].as_f64().unwrap();
    let mut c = 0.0;
    for (t, p) in per.iter().enumerate() {
        let q = if t < dtf { fijo[t] } else { lotes[t - dtf] as f64 * lot };
        if q > 0.0 {
            c += m["setup_cost"].as_f64().unwrap();
            if let Some(cap) = p["capacity"].as_f64() {
                if q * h > cap + 1e-9 {
                    return None;
                }
            }
        }
        let f = p["forecast"].as_f64().unwrap();
        let o = p["orders"].as_f64().unwrap();
        let d = if t < dtf { o } else { f.max(o) };
        inv += q - d;
        if inv < ss - 1e-9 {
            return None;
        }
        c += inv * m["holding_cost"].as_f64().unwrap();
    }
    Some(c)
}

/// El mejor plan por enumeración (hasta `max` lotes por periodo libre).
fn mejor(m: &Value, fijo: &[f64], max: u32) -> Option<f64> {
    let n = m["periods"].as_array().unwrap().len() - m["demand_fence"].as_u64().unwrap() as usize;
    let mut best: Option<f64> = None;
    let mut x = vec![0u32; n];
    loop {
        if let Some(c) = costo(m, &x, fijo) {
            best = Some(best.map_or(c, |b: f64| b.min(c)));
        }
        let mut k = 0;
        while k < n && x[k] == max {
            x[k] = 0;
            k += 1;
        }
        if k == n {
            return best;
        }
        x[k] += 1;
    }
}

#[test]
fn optimo_igual_a_la_enumeracion() {
    let m = ex("ato_capacidad");
    let regla = Module::solve(&Pmp, &m, &Value::Null).unwrap();
    let r = Module::solve(&Pmp, &m, &json!({"method": "optimo"})).unwrap();
    let fijo = nums(&regla, "mps");
    let b = mejor(&m, &fijo, 3).unwrap();
    assert!((r.data["cost"].as_f64().unwrap() - b).abs() < 1e-6, "{} frente a {b}", r.data["cost"]);
    assert!(r.data["cost"].as_f64().unwrap() < r.data["rule_cost"].as_f64().unwrap());
    assert_eq!(r.data["overloaded_periods"], json!([]));
    // problemas al azar (generador congruencial: siempre los mismos)
    let mut s: u64 = 20261122;
    let mut rnd = |a: u64, b: u64| {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        a + (s >> 33) % (b - a + 1)
    };
    let mut probados = 0;
    for _ in 0..60 {
        let n = 6;
        let lot = rnd(20, 40) as f64;
        let periods: Vec<Value> = (0..n).map(|_| json!({"name": "", "forecast": rnd(5, 25), "orders": rnd(0, 20), "capacity": if rnd(0, 3) == 0 { json!(rnd(20, 90)) } else { Value::Null }})).collect();
        let m = json!({
            "name": "azar", "problem_type": "ato", "on_hand": rnd(0, 40), "lot_size": lot, "safety_stock": rnd(0, 10),
            "demand_fence": 1, "planning_fence": 0, "hours_per_unit": 1.0, "setup_cost": rnd(10, 200), "holding_cost": rnd(1, 5),
            "periods": periods, "requests": []
        });
        let regla = Module::solve(&Pmp, &m, &Value::Null).unwrap();
        let fijo = nums(&regla, "mps");
        let r = Module::solve(&Pmp, &m, &json!({"method": "optimo"})).unwrap();
        match mejor(&m, &fijo, 5) {
            Some(b) => {
                assert_eq!(r.status, iaqsb_core::Status::Optimal, "{m}");
                assert!((r.data["cost"].as_f64().unwrap() - b).abs() < 1e-6, "{m}: {} frente a {b}", r.data["cost"]);
                probados += 1;
            }
            None => assert_eq!(r.status, iaqsb_core::Status::Infeasible, "{m}"),
        }
    }
    assert!(probados > 30, "solo {probados} problemas factibles");
}

#[test]
fn archivo_ida_y_vuelta() {
    let m = ex("ato_solicitudes");
    let b = Module::write_file(&Pmp, &m).unwrap();
    assert!(Module::recognizes(&Pmp, std::str::from_utf8(&b).unwrap()));
    let back = Module::read_file(&Pmp, &b).unwrap();
    assert_eq!(back, m);
}
