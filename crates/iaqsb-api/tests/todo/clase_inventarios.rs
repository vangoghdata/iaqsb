//! Ejercicios de clase de inventarios, finanzas del inventario, almacenes e
//! indicadores logísticos: cada respuesta se comprobó a mano y con un cálculo
//! independiente en Python.

use iaqsb_api as api;
use serde_json::{json, Value};

fn ex(module: &str, name: &str) -> Value {
    api::examples().into_iter().find(|e| e.0 == module && e.1 == name).unwrap_or_else(|| panic!("no existe {module}/{name}")).2
}
fn run(module: &str, name: &str, opts: Value) -> Value {
    api::solve(module, ex(module, name), Some(opts)).unwrap_or_else(|e| panic!("{module}/{name}: {e}"))
}
fn close(a: &Value, b: f64, tol: f64, what: &str) {
    let x = a.as_f64().unwrap_or_else(|| panic!("{what}: no es número ({a})"));
    assert!((x - b).abs() <= tol, "{what}: {x} ≠ {b}");
}

#[test]
fn eoq_y_poq_ferreteria() {
    let r = run("its", "clase_ferreteria_eoq", json!({}));
    close(&r["data"]["order_quantity"], 524.4044, 1e-3, "EOQ");
    close(&r["data"]["related_cost"], 209.7618, 1e-3, "CT");
    close(&r["data"]["reorder_point"], 120.0, 1e-9, "R");
    let r = run("its", "clase_ferreteria_poq", json!({}));
    close(&r["data"]["order_quantity"], 642.2616, 1e-3, "POQ");
    close(&r["data"]["related_cost"], 171.2698, 1e-3, "CT POQ");
    close(&r["data"]["max_inventory"], 428.1744, 1e-3, "Imax");
    let r = run("its", "clase_te_always_fresh", json!({}));
    close(&r["data"]["order_quantity"], 22.8035, 1e-3, "té");
    close(&r["data"]["related_cost"], 364.8561, 1e-3, "CT té");
}

#[test]
fn descuentos() {
    let r = run("its", "clase_descuento_limpieza", json!({}));
    close(&r["data"]["order_quantity"], 30.0, 1e-9, "Q");
    close(&r["data"]["total_cost"], 1078.3333, 1e-3, "CT");
    let r = run("its", "clase_descuento_tres_rangos", json!({}));
    close(&r["data"]["order_quantity"], 700.0, 1e-9, "Q");
    close(&r["data"]["total_cost"], 18204.5714, 1e-3, "CT");
}

#[test]
fn despejes_del_parcial_y_la_tarea() {
    let r = run("inv", "parcial_poq_entregas", json!({}));
    let v = &r["data"]["values"];
    close(&v["Q"], 1950.0, 1e-6, "Q");
    close(&v["p"], 165.0, 1e-6, "p");
    close(&v["Imax"], 1181.8182, 1e-3, "Imax");
    close(&r["data"]["total_cost"], 2543.6364, 1e-3, "CT");
    close(&r["data"]["optimal_quantity"], 538.1217, 1e-3, "Q*");
    assert_eq!(r["data"]["consistent"], true);
    let r = run("inv", "parcial_eoq_grafica", json!({}));
    close(&r["data"]["values"]["Q"], 50.0, 1e-9, "Q");
    close(&r["data"]["values"]["D"], 400.0, 1e-9, "D");
    close(&r["data"]["total_cost"], 400.0, 1e-9, "CT");
    let r = run("inv", "poq_tarea", json!({}));
    let v = &r["data"]["values"];
    close(&v["Q"], 750.0, 1e-9, "Q");
    close(&v["p"], 250.0, 1e-9, "p");
    close(&v["d"], 50.0, 1e-9, "d");
    close(&r["data"]["total_cost"], 1395.0, 1e-6, "CT");
    // datos que se contradicen: se detecta
    let mut m = ex("inv", "poq_tarea");
    m["cycle_days"] = json!(20);
    let r = api::solve("inv", m, None).unwrap();
    assert_eq!(r["data"]["consistent"], false);
}

#[test]
fn abc_quince_articulos() {
    let r = run("inv", "abc_15_articulos", json!({}));
    let g = r["data"]["groups"].as_array().unwrap();
    assert_eq!(g[0]["items"], json!(["4", "14", "3"]));
    assert_eq!(g[1]["items"], json!(["9", "1", "8", "2", "12", "5"]));
    assert_eq!(g[2]["items"], json!(["13", "6", "10", "11", "15", "7"]));
    close(&g[0]["value_pct"], 60.6, 1e-9, "A");
    close(&r["data"]["total"], 55837.0, 1e-9, "total");
    // por porcentajes 80/95: A llega hasta el artículo 8 (80.39 %)
    let r = run("inv", "abc_15_articulos", json!({"abc_method": "porcentajes"}));
    assert_eq!(r["data"]["groups"][0]["items"].as_array().unwrap().len(), 6);
}

#[test]
fn sell_rite() {
    let r = run("inv", "sell_rite", json!({}));
    close(&r["data"]["reorder_point"], 14019.7332, 1e-3, "R exacto");
    let r = run("inv", "sell_rite", json!({"z_mode": "tabla"}));
    close(&r["data"]["z"], 1.28, 1e-12, "Z tabla");
    close(&r["data"]["reorder_point"], 14017.8933, 1e-3, "R con Z = 1.28");
    close(&r["data"]["safety_stock"], 1517.8933, 1e-3, "SS");
}

#[test]
fn taller2_politica_qr() {
    let r = run("inv", "taller2_qr", json!({"search": false}));
    let sc = r["data"]["scenarios"].as_array().unwrap();
    // (Q, R, inventario promedio, perdidas o pendientes, costo total) del informe del Grupo 8
    let want = [
        (1442.0, 1344.0, 656.07, 1973.0, 14855.91),
        (1442.0, 1634.0, 773.73, 996.0, 9535.17),
        (2404.0, 1344.0, 985.80, 1476.0, 13390.68),
        (1442.0, 831.0, 679.67, 1196.0, 10302.47),
        (1150.0, 892.0, 791.67, 225.0, 4991.67),
        (500.0, 1225.0, 551.67, 465.0, 5327.67),
        (850.0, 892.0, 681.33, 370.0, 5354.13),
        (1442.0, 892.0, 898.60, 531.0, 7319.56),
        (1442.0, 1344.0, 341.40, 2534.0, 1570.44),
    ];
    assert_eq!(sc.len(), want.len());
    for (s, w) in sc.iter().zip(want) {
        let n = s["name"].as_str().unwrap();
        close(&s["quantity"], w.0, 1e-9, &format!("{n} Q"));
        close(&s["reorder_point"], w.1, 1e-9, &format!("{n} R"));
        close(&s["average_inventory"], w.2, 0.006, &format!("{n} inventario promedio"));
        close(&s["shortage_units"], w.3, 1e-9, &format!("{n} faltantes"));
        close(&s["total_cost"], w.4, 0.006, &format!("{n} costo total"));
    }
    // inventario en bodega del escenario A, día a día
    assert_eq!(sc[0]["stock"], json!([1200.0, 770.0, 385.0, 0.0, 0.0, 1442.0, 1002.0, 572.0, 92.0, 0.0, 0.0, 1442.0, 1182.0, 977.0, 777.0]));
    // con back orders la bodega queda negativa
    assert_eq!(sc[8]["stock"][3], json!(-25.0));
    assert_eq!(sc[8]["orders"], json!(4));
    // la búsqueda encuentra una política mejor que la propuesta E1
    let r = run("inv", "taller2_qr", json!({}));
    assert!(r["data"]["best_search"]["total_cost"].as_f64().unwrap() < 4991.67);
}

#[test]
fn finanzas_del_inventario() {
    let r = run("fin", "roi_escenarios", json!({}));
    close(&r["data"]["cases"][0]["roi_pct"], 11.5044, 1e-3, "ROI A");
    close(&r["data"]["cases"][1]["roi_pct"], 15.2381, 1e-3, "ROI B");
    let r = run("fin", "roi_manufacturera", json!({}));
    close(&r["data"]["cases"][0]["roi_pct"], 4.9774, 1e-3, "ROI actual");
    close(&r["data"]["cases"][1]["net_income"], 243.5, 1e-9, "UN reducida");
    close(&r["data"]["cases"][1]["roi_pct"], 8.9687, 1e-3, "ROI reducido");
    let r = run("fin", "liquidez_almacen", json!({}));
    close(&r["data"]["current_ratio"], 1.4, 1e-9, "RC");
    close(&r["data"]["acid_test"], 0.5667, 1e-4, "PA");
    close(&r["data"]["after"]["current_ratio"], 1.275, 1e-9, "RC con obsoleto");
    let r = run("fin", "capital_de_trabajo", json!({}));
    close(&r["data"]["cycles"][0]["working_capital"], 150.0, 1e-9, "CT actual");
    close(&r["data"]["cycles"][1]["working_capital"], 65.625, 1e-9, "CT nuevo");
    close(&r["data"]["cycles"][1]["profit_per_cycle"], 21.875, 1e-9, "utilidad por ciclo");
    let r = run("fin", "parcial_capital", json!({}));
    close(&r["data"]["cycles"][0]["inventory_days"], 85.0, 1e-9, "días de inventario");
    close(&r["data"]["cycles"][1]["working_capital"], 3650.0, 1e-6, "CT con 20 % menos");
}

#[test]
fn almacenes() {
    let r = run("alm", "envasadora", json!({}));
    assert_eq!(r["data"]["boxes_per_pallet"], json!(80.0));
    assert_eq!(r["data"]["pallets_per_lot"], json!(13.0));
    assert_eq!(r["data"]["spaces"], json!(8.0));
    close(&r["data"]["area"], 475.15, 1e-9, "área");
    let r = run("alm", "taller3_estiba", json!({}));
    assert_eq!(r["data"]["boxes_per_pallet"], json!(132.0));
    assert_eq!(r["data"]["binding"], json!("Peso del montacargas"));
    let r = run("alm", "compania_quimica", json!({}));
    close(&r["data"]["optimal_area"], 63720.2381, 1e-3, "óptimo");
    close(&r["data"]["optimal_cost"], 913306.07, 0.01, "costo óptimo");
    let alts: Vec<(f64, f64)> = r["data"]["alternatives"].as_array().unwrap().iter().map(|a| (a["area"].as_f64().unwrap(), a["total"].as_f64().unwrap())).collect();
    for (a, c) in [(40000.0, 934085.0), (50000.0, 925325.0), (60000.0, 916565.0), (70000.0, 919058.33), (80000.0, 943845.0)] {
        let x = alts.iter().find(|x| (x.0 - a).abs() < 1e-9).unwrap();
        close(&json!(x.1), c, 0.01, &format!("alternativa {a}"));
    }
    let r = run("alm", "taller3_dimensionado", json!({}));
    close(&r["data"]["optimal_area"], 3030.8571, 1e-3, "óptimo taller 3");
    close(&r["data"]["fixed_per_area"], 15.0417, 1e-4, "F");
}

#[test]
fn indicadores_logisticos() {
    let r = run("kpi", "taller1_distribuidor", json!({}));
    let d = &r["data"];
    close(&d["on_time_pct"], 80.0, 1e-9, "a tiempo");
    close(&d["in_full_pct"], 83.3, 1e-9, "completos");
    close(&d["doc_ok_pct"], 80.0, 1e-9, "documentación");
    close(&d["perfect_pct"], 60.0, 1e-9, "perfecto");
    close(&d["otif_pct"], 70.0, 1e-9, "OTIF");
    close(&d["urgent_pct"], 40.0, 1e-9, "urgentes");
    close(&d["cycle_avg"], 3.2333, 1e-3, "ciclo");
    close(&d["logistics_cost_pct"], 9.2, 1e-9, "costo / ventas");
    close(&d["welch"]["t"], -8.6465, 1e-3, "t de Welch");
    assert_eq!(d["welch"]["reject"], json!(true));
}
