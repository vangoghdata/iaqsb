//! QCC: límites de Montgomery (gráficos C y P), X̄-R contra el cálculo directo,
//! reglas, capacidad, Pareto con columna de total y archivos de WinQSB.

use iaqsb_core::Module;
use iaqsb_modules::qcc::Qcc;
use serde_json::{json, Value};

fn ex(name: &str) -> Value {
    Module::examples(&Qcc).into_iter().find(|(n, _)| n == name).unwrap().1
}
fn f(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

#[test]
fn grafico_c_montgomery() {
    // 26 muestras de 100 tarjetas: c̄ = 19.85, LCS = 33.22, LCI = 6.48; las muestras 6 y 20 salen de los límites
    let r = Module::solve(&Qcc, &ex("tarjetas_de_circuito_c"), &Value::Null).unwrap();
    assert_eq!(r.data["chart"], "c");
    assert!((f(&r.data["cl"]) - 19.846).abs() < 1e-3 && (f(&r.data["ucl"]) - 33.22).abs() < 1e-2 && (f(&r.data["lcl"]) - 6.48).abs() < 1e-2);
    let out: Vec<&str> = r.data["out_of_control"].as_array().unwrap().iter().map(|x| x.as_str().unwrap()).collect();
    assert!(out.contains(&"6") && out.contains(&"20"));
    // solo con la regla 1 y 2 (fuera de los límites)
    let mut m = ex("tarjetas_de_circuito_c");
    m["rules"] = json!([m["rules"][0], m["rules"][1]]);
    let r = Module::solve(&Qcc, &m, &Value::Null).unwrap();
    assert_eq!(r.data["out_of_control"], json!(["6", "20"]));
}

#[test]
fn grafico_p_montgomery() {
    // latas de jugo: p̄ = 0.2313, LCS = 0.4102, LCI = 0.0524; las muestras 15 y 23 pasan el LCS
    let r = Module::solve(&Qcc, &ex("latas_de_jugo_p"), &Value::Null).unwrap();
    assert_eq!(r.data["chart"], "p");
    assert!((f(&r.data["cl"]) - 0.2313).abs() < 1e-4 && (f(&r.data["ucl"]) - 0.4102).abs() < 1e-4 && (f(&r.data["lcl"]) - 0.0524).abs() < 1e-4);
    let mut m = ex("latas_de_jugo_p");
    m["rules"] = json!([m["rules"][0]]);
    let r = Module::solve(&Qcc, &m, &Value::Null).unwrap();
    assert_eq!(r.data["out_of_control"], json!(["15", "23"]));
    // nP con los mismos datos: LC = n·p̄
    let r = Module::solve(&Qcc, &ex("latas_de_jugo_p"), &json!({"chart": "np"})).unwrap();
    assert!((f(&r.data["cl"]) - 50.0 * 0.23133).abs() < 1e-2);
}

#[test]
fn xbar_r_y_capacidad() {
    let m = ex("anillos_de_piston");
    let r = Module::solve(&Qcc, &m, &json!({"chart": "xbar"})).unwrap();
    // cálculo directo: σ = R̄/d2 con d2(5) = 2.326
    let rows = m["rows"].as_array().unwrap();
    let data = m["data"].as_array().unwrap();
    let mut groups: Vec<Vec<f64>> = vec![];
    for (i, row) in rows.iter().enumerate() {
        let g = f(&row["subgroup"]) as usize;
        if groups.len() < g {
            groups.resize(g, vec![]);
        }
        groups[g - 1].push(f(&data[i][0]));
    }
    let rbar: f64 = groups.iter().map(|g| g.iter().cloned().fold(f64::MIN, f64::max) - g.iter().cloned().fold(f64::MAX, f64::min)).sum::<f64>() / groups.len() as f64;
    let xbb: f64 = groups.iter().map(|g| g.iter().sum::<f64>() / g.len() as f64).sum::<f64>() / groups.len() as f64;
    assert!((f(&r.data["sigma"]) - rbar / 2.326).abs() < 2e-5);
    assert!((f(&r.data["ucl"]) - (xbb + 0.577 * rbar)).abs() < 1e-4, "A2 = 0.577");
    // capacidad con las especificaciones del archivo
    assert!(r.data["capability"]["pcr"].is_number());
}

#[test]
fn pareto_sin_la_columna_total() {
    let r = Module::solve(&Qcc, &ex("defectos_pareto"), &Value::Null).unwrap();
    let sec = r.sections.iter().find(|s| s.id == "pareto").expect("sección de Pareto");
    let txt = serde_json::to_string(&sec.blocks).unwrap();
    assert!(txt.contains("se deja fuera") && txt.contains("Patas dobladas"));
}

#[test]
fn todos_los_graficos() {
    let mut var = ex("anillos_de_piston");
    var["characteristics"][0]["lsl"] = json!(73.95);
    var["characteristics"][0]["usl"] = json!(74.05);
    let att = ex("defectos_pareto");
    for (i, (_, id, _)) in iaqsb_modules::qcc::CHARTS.iter().enumerate() {
        let m = if i < 21 { &var } else { &att };
        let r = Module::solve(&Qcc, m, &json!({"chart": id}));
        assert!(r.is_ok(), "{id}: {:?}", r.err());
    }
}

#[test]
fn archivos_qcc() {
    for (name, _) in Module::examples(&Qcc) {
        let m = ex(&name);
        let out = Module::write_file(&Qcc, &m).unwrap();
        let back = Module::read_file(&Qcc, &out).unwrap();
        assert_eq!(back, m, "{name}");
    }
    // formato horizontal: se lee como vertical y se vuelve a escribir horizontal
    let h = "QCC\th\t0\t1\r\n1\t2\t3\t0\t0\t0\r\n3\t11\r\nSubgroup\tCharacteristics\tDate\tTime\tObservation 1\tObservation 2\tObservation 3\tDisabled\tCause Code\tAction Code\tComment\r\n1\tPeso\t\t\t10\t11\t12\t\t\t\t\r\n2\tPeso\t\t\t11\t13\t9\t\t\t\t\r\n1\tPeso\r\n0\t0\t0\t0\r\n";
    let m = Module::read_file(&Qcc, h.as_bytes()).unwrap();
    assert_eq!(m["rows"].as_array().unwrap().len(), 6);
    assert_eq!(m["data"][4][0], 13.0);
    let back = Module::read_file(&Qcc, &Module::write_file(&Qcc, &m).unwrap()).unwrap();
    assert_eq!(back, m);
}
