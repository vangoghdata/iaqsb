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

/// Ejercicio de clase (cuestionario): mismos números que R 4.6.1 con forecast 9.0.2.
/// La suavización exponencial y el ingenuo los da igual el motor nativo; TBATS, solo R.
#[test]
fn evaluacion_como_en_clase() {
    let r = Module::solve(&St, &ex("evaluacion_clase"), &json!({"forecast_engine": "nativo"})).unwrap();
    let d = &r.data;
    assert_eq!(d["train_end"], "2023-12");
    assert_eq!(d["train_size"], 32);
    assert_eq!(d["test_size"], 8);
    assert_eq!(d["recommended_smoothing"], "ses", "el correlograma no muestra tendencia ni estacionalidad");
    let modelo = |id: &str| d["models"].as_array().unwrap().iter().find(|m| m["id"] == id).unwrap().clone();
    let ses = modelo("ses");
    // accuracy(ses(tr, h = 8), te)[, "RMSE"]
    cerca(ses["accuracy"]["training"]["rmse"].as_f64().unwrap(), 938.8517287, 1e-6, "RMSE de ses en el entrenamiento");
    cerca(ses["accuracy"]["test"]["rmse"].as_f64().unwrap(), 881.5051704, 1e-6, "RMSE de ses en la prueba");
    // checkresiduals(ses): Q* = 4.0338, df = 6, p-value = 0.6721
    cerca(ses["ljung_box"]["q"].as_f64().unwrap(), 4.0338, 5e-5, "Q* de ses");
    assert_eq!(ses["ljung_box"]["df"], 6);
    cerca(ses["ljung_box"]["p_value"].as_f64().unwrap(), 0.6721, 5e-5, "p de ses");
    let ing = modelo("ingenuo");
    cerca(ing["accuracy"]["training"]["rmse"].as_f64().unwrap(), 1269.1196603, 1e-6, "RMSE del ingenuo en el entrenamiento");
    cerca(ing["accuracy"]["test"]["rmse"].as_f64().unwrap(), 889.3556319, 1e-6, "RMSE del ingenuo en la prueba");
    assert_eq!(d["best_test_rmse"], "ses");
    // misma serie ×3.1875 (la otra versión del cuestionario): 2992.590339 y 2809.682925
    let mut m2 = ex("evaluacion_clase");
    for o in m2["data"].as_array_mut().unwrap() {
        let v = o["value"].as_f64().unwrap() * 3.1875;
        o["value"] = json!(v);
    }
    let r2 = Module::solve(&St, &m2, &json!({"forecast_engine": "nativo", "steps": false})).unwrap();
    let s2 = r2.data["models"].as_array().unwrap().iter().find(|m| m["id"] == "ses").unwrap().clone();
    cerca(s2["accuracy"]["training"]["rmse"].as_f64().unwrap(), 2992.590339, 1e-5, "RMSE de ses ×3.1875 en el entrenamiento");
    cerca(s2["accuracy"]["test"]["rmse"].as_f64().unwrap(), 2809.682925, 1e-5, "RMSE de ses ×3.1875 en la prueba");
}

#[test]
fn evaluacion_con_r_incluye_tbats() {
    if !motores::r_disponible() {
        eprintln!("R con forecast no está instalado: se omite");
        return;
    }
    let r = Module::solve(&St, &ex("evaluacion_clase"), &json!({"forecast_engine": "r"})).unwrap();
    let t = r.data["models"].as_array().unwrap().iter().find(|m| m["id"] == "tbats").unwrap().clone();
    assert_eq!(t["model"], "BATS(1, {0,0}, -, -)");
    cerca(t["accuracy"]["training"]["rmse"].as_f64().unwrap(), 954.0058404, 1e-6, "RMSE de tbats en el entrenamiento");
    cerca(t["accuracy"]["test"]["rmse"].as_f64().unwrap(), 890.3480692, 1e-6, "RMSE de tbats en la prueba");
}

/// ets(), holt() y hw() traducidos de forecast: mismos parámetros y pronósticos que R.
#[test]
fn ets_nativo_igual_a_r() {
    let solve = |ejemplo: &str, extra: Value| {
        let mut m = ex(ejemplo);
        m["problem_type"] = json!("pronostico");
        m["horizon"] = json!(4);
        for (k, v) in extra.as_object().unwrap() {
            m[k] = v.clone();
        }
        Module::solve(&St, &m, &json!({"forecast_engine": "nativo", "steps": false})).unwrap().data
    };
    let coef = |d: &Value, n: &str| d["coefficients"].as_array().unwrap().iter().find(|c| c["name"] == n).unwrap()["value"].as_f64().unwrap();
    // hw(qcement, h = 4, seasonal = "multiplicative")
    let d = solve("qcement_comparar", json!({"model": "hw", "seasonal_type": "multiplicativa"}));
    assert_eq!(d["model"], "Holt-Winters' multiplicative method");
    cerca(coef(&d, "alpha"), 0.6117689501, 1e-7, "alpha");
    cerca(coef(&d, "beta"), 0.0001111052346, 1e-9, "beta");
    cerca(coef(&d, "gamma"), 0.0001080417085, 1e-9, "gamma");
    cerca(nums(&d["forecast"])[0], 2.5285, 1e-4, "pronóstico");
    cerca(nums(&d["lower"])[0], 2.2514, 1e-4, "límite inferior del 95 %");
    // holt(utilidades, h = 4, damped = TRUE)
    let d = solve("utilidades_arango", json!({"model": "holt", "damped": true}));
    assert_eq!(d["model"], "Damped Holt's method");
    cerca(coef(&d, "alpha"), 0.3128705579, 1e-7, "alpha");
    cerca(coef(&d, "phi"), 0.9799999946, 1e-7, "phi");
    cerca(nums(&d["forecast"])[0], 364.6542, 1e-4, "pronóstico");
    cerca(nums(&d["lower"])[0], 297.6244, 1e-4, "límite inferior del 95 %");
    // ets(utilidades): ETS(M,N,M), AICc 381.4362
    let d = solve("utilidades_arango", json!({"model": "ets"}));
    assert_eq!(d["model"], "ETS(M,N,M)");
    cerca(d["aicc"].as_f64().unwrap(), 381.4362335, 1e-5, "AICc");
    cerca(nums(&d["forecast"])[0], 285.3838, 1e-4, "pronóstico");
}
