//! Series de tiempo: la exploración y el motor nativo comparados con R 4.6.1 y
//! el paquete forecast 9.0.2 (valores de referencia sacados de R: las pruebas
//! corren también en equipos sin R). Si R está instalado, también se prueba el puente.

use iaqsb_core::Module;
use iaqsb_modules::st::{analisis, motores, St};
use serde_json::{json, Value};

fn ex(name: &str) -> Value {
    Module::examples(&St).into_iter().find(|(n, _)| n == name).unwrap().1
}

fn cerca(a: f64, b: f64, tol: f64, que: &str) {
    assert!((a - b).abs() <= tol, "{que}: {a} frente a {b}");
}

fn nums(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap_or(f64::NAN)).collect()
}

#[test]
fn exploracion_igual_a_r() {
    // Acf(u), Pacf(u), Box.test(u, lag = 8, type = "Ljung-Box"), decompose(u)$figure y BoxCox.lambda(u)
    let r = Module::solve(&St, &ex("utilidades_arango"), &Value::Null).unwrap();
    let acf = nums(&r.data["acf"]);
    for (a, b) in acf.iter().zip([0.8334, 0.6261, 0.5817, 0.6176, 0.4759, 0.2841]) {
        cerca(*a, b, 5e-5, "ACF");
    }
    assert_eq!(acf.len(), 16, "16 retardos, como Acf() de R");
    for (a, b) in nums(&r.data["pacf"]).iter().zip([0.8334, -0.2239, 0.4468, 0.0829]) {
        cerca(*a, b, 5e-5, "PACF");
    }
    cerca(r.data["ljung_box"]["q"].as_f64().unwrap(), 101.7476, 5e-5, "Q de Ljung-Box");
    for (a, b) in nums(&r.data["seasonal_indices"]).iter().zip([-4.8442, -29.0483, 14.2186, 19.6739]) {
        cerca(*a, b, 5e-5, "índices estacionales");
    }
    cerca(r.data["lambda"].as_f64().unwrap(), -0.2625, 2e-4, "λ de Guerrero");
}

#[test]
fn box_cox_y_errores() {
    for l in [0.0, 0.5, -0.26, 1.0] {
        let y = 123.4;
        cerca(analisis::box_cox_inv(analisis::box_cox(y, l), l), y, 1e-9, "Box-Cox ida y vuelta");
    }
    // accuracy(): con y = 1..6 y ajustados y − 1, ME = MAE = RMSE = 1 y MASE = 1 (el ingenuo tiene error 1)
    let y: Vec<f64> = (1..=6).map(|x| x as f64).collect();
    let f: Vec<f64> = y.iter().map(|v| v - 1.0).collect();
    let e = analisis::errores(&y, &f, &y, 1);
    cerca(e.me, 1.0, 1e-12, "ME");
    cerca(e.rmse, 1.0, 1e-12, "RMSE");
    cerca(e.mase, 1.0, 1e-12, "MASE");
    // sin valores para comparar, nada de ceros engañosos
    assert!(analisis::errores(&y, &[f64::NAN; 6], &y, 1).rmse.is_nan());
}

#[test]
fn motor_nativo_da_los_coeficientes_de_r() {
    // auto.arima(usnetelec): ARIMA(2,1,2) con deriva; coef ar1 −1.3032, ar2 −0.4332, ma1 1.5284, ma2 0.834
    let r = Module::solve(&St, &ex("usnetelec_arima"), &json!({"forecast_engine": "nativo"})).unwrap();
    assert_eq!(r.data["engine"], "nativo");
    assert!(r.data["model"].as_str().unwrap().starts_with("ARIMA(2,1,2)"), "{}", r.data["model"]);
    let c: Vec<f64> = r.data["coefficients"].as_array().unwrap().iter().map(|x| x["value"].as_f64().unwrap()).collect();
    for (a, b) in c.iter().zip([-1.3032, -0.4332, 1.5284, 0.834]) {
        cerca(*a, b, 1e-3, "coeficientes de usnetelec");
    }
    assert!(r.data["accuracy"]["rmse"].as_f64().unwrap() < 60.0, "RMSE {}", r.data["accuracy"]["rmse"]);
    // auto.arima(h02, lambda = 0): ARIMA(2,1,1)(0,1,2)[12]
    let r = Module::solve(&St, &ex("h02_arima_log"), &json!({"forecast_engine": "nativo"})).unwrap();
    assert_eq!(r.data["model"], "ARIMA(2,1,1)(0,1,2)[12]");
    let c: Vec<f64> = r.data["coefficients"].as_array().unwrap().iter().map(|x| x["value"].as_f64().unwrap()).collect();
    for (a, b) in c.iter().zip([-1.1358, -0.5753, 0.3683, -0.5318, -0.1817]) {
        cerca(*a, b, 2e-3, "coeficientes de h02");
    }
    let f = nums(&r.data["forecast"]);
    cerca(f[0], 1.0918, 0.01, "primer pronóstico de h02 (R: 1.0918)");
}

#[test]
fn comparacion_elige_por_mase() {
    let r = Module::solve(&St, &ex("qcement_comparar"), &json!({"forecast_engine": "nativo"})).unwrap();
    let c = &r.data["comparison"];
    assert_eq!(c["test_size"], 25);
    let modelos = c["models"].as_array().unwrap();
    assert!(modelos.len() >= 4);
    let mejor = modelos.iter().min_by(|a, b| a["mase"].as_f64().unwrap().partial_cmp(&b["mase"].as_f64().unwrap()).unwrap()).unwrap();
    assert_eq!(c["best"], mejor["model"]);
    assert_eq!(nums(&r.data["forecast"]).len(), 8);
}

#[test]
fn con_r_si_esta_instalado() {
    if motores::r_info().is_none() {
        eprintln!("R con forecast no está instalado: se omite");
        return;
    }
    let r = Module::solve(&St, &ex("usnetelec_arima"), &json!({"forecast_engine": "r"})).unwrap();
    assert_eq!(r.data["model"], "ARIMA(2,1,2) with drift");
    let c = r.data["coefficients"].as_array().unwrap();
    let drift = c.iter().find(|x| x["name"] == "drift").unwrap()["value"].as_f64().unwrap();
    cerca(drift, 66.1585, 1e-3, "deriva");
    cerca(r.data["aicc"].as_f64().unwrap(), 580.461, 1e-2, "AICc");
}

#[test]
fn errores_claros_y_archivo() {
    let mut m = ex("usnetelec_arima");
    m["arima_order"] = json!("1,1");
    assert!(Module::solve(&St, &m, &Value::Null).unwrap_err().to_string().contains("orden ARIMA"));
    let mut m = ex("usnetelec_arima");
    m["data"][3]["value"] = Value::Null;
    assert!(Module::solve(&St, &m, &Value::Null).unwrap_err().to_string().contains("huecos"));
    let b = Module::write_file(&St, &ex("h02_arima_log")).unwrap();
    assert_eq!(Module::read_file(&St, &b).unwrap(), ex("h02_arima_log"));
}

#[test]
fn con_python_si_esta_instalado() {
    if motores::py_info().is_none() {
        eprintln!("statsforecast no está instalado: se omite");
        return;
    }
    let r = Module::solve(&St, &ex("usnetelec_arima"), &json!({"forecast_engine": "python"})).unwrap();
    assert_eq!(r.data["engine"], "python");
    let m = r.data["model"].as_str().unwrap();
    assert!(m.starts_with("ARIMA(") && !m.ends_with(' '), "{m}");
    assert_eq!(nums(&r.data["forecast"]).len(), 10);
    // TBATS con el periodo entero (statsforecast no acepta 12.0)
    let r = Module::solve(&St, &ex("gas_tbats"), &json!({"forecast_engine": "python"})).unwrap();
    assert_eq!(r.data["model"], "TBATS");
}
