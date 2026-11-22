//! GP-IGP: respuesta de libro (Render), archivos .GP en sus dos formas y errores.

use iaqsb_core::Module;
use iaqsb_modules::gp::Gp;
use serde_json::{json, Value};

fn harrison() -> Value {
    Module::examples(&Gp).into_iter().next().unwrap().1
}

fn value(r: &iaqsb_core::Report, name: &str) -> f64 {
    r.data["variables"].as_array().unwrap().iter().find(|v| v["name"] == name).unwrap()["value"].as_f64().unwrap()
}

#[test]
fn harrison_render() {
    let r = Module::solve(&Gp, &harrison(), &Value::Null).unwrap();
    // Render: X1 = 0, X2 = 6, d1+ = 6, d2+ = 6, d4- = 1
    assert_eq!(value(&r, "Candelabros"), 0.0);
    assert_eq!(value(&r, "Ventiladores"), 6.0);
    assert_eq!(value(&r, "d1p"), 6.0);
    assert_eq!(value(&r, "d2p"), 6.0);
    assert_eq!(value(&r, "d4m"), 1.0);
    let g: Vec<f64> = r.data["goal_values"].as_array().unwrap().iter().map(|g| g["value"].as_f64().unwrap()).collect();
    assert_eq!(g, vec![0.0, 0.0, 0.0, 1.0]);
    // el símplex de metas aparece en el paso a paso
    let pasos = r.sections.iter().find(|s| s.id == "pasos").unwrap();
    let txt = serde_json::to_string(&pasos.blocks).unwrap();
    assert!(txt.contains("Símplex de metas") && txt.contains("Tabla final"));
}

#[test]
fn archivos_gp_matriz_y_normal() {
    let m = harrison();
    for normal in [false, true] {
        let mut mm = m.clone();
        mm["normal_form"] = json!(normal);
        let out = Module::write_file(&Gp, &mm).unwrap();
        let text = iaqsb_core::winqsb::decode(&out);
        assert!(text.starts_with(if normal { "GP\tNormalModel" } else { "GP\tMatrixFormat" }), "{text}");
        let back = Module::read_file(&Gp, &out).unwrap();
        let a = Module::solve(&Gp, &back, &Value::Null).unwrap();
        assert_eq!(a.data["goal_values"], Module::solve(&Gp, &m, &Value::Null).unwrap().data["goal_values"]);
        assert_eq!(back["goal_coefs"], m["goal_coefs"]);
        assert_eq!(back["coefs"], m["coefs"]);
    }
}

#[test]
fn modelo_normal_escrito_a_mano_y_lp() {
    let txt = "GP\tNormalModel\tMezcla\t2\t3\t1\r\nGoal/Constraint/VariableType/Bound\t\r\n\
               Min:Faltante\tD1\r\nMax:Ganancia\t3X+2Y\r\nC1\tX+Y+D1>=10\r\n\
               Integer:\tX\r\nBinary:\t\r\nUnrestricted:\t\r\nX\t>=0, <=4\r\nY\t>=0, <=5\r\nD1\t>=0, <=M\r\n";
    let m = Module::read_file(&Gp, txt.as_bytes()).unwrap();
    assert_eq!(m["variables"][0]["type"], "integer");
    assert_eq!(m["variables"][0]["upper"], 4.0);
    let r = Module::solve(&Gp, &m, &Value::Null).unwrap();
    // X + Y ≤ 9 → falta 1; después ganancia máxima 3·4 + 2·5 = 22
    assert_eq!(r.data["goal_values"][0]["value"], 1.0);
    assert_eq!(r.data["goal_values"][1]["value"], 22.0);
    // WinQSB abre también un archivo de LP-ILP como una sola meta
    let lp = "LP\tMatrixFormat\tWyndor\t2\t3\r\nVariable -->\tX1\tX2\tDirection\tR. H. S.\r\nMaximize\t3\t5\t\t\r\n\
              C1\t1\t0\t<=\t4\r\nC2\t0\t2\t<=\t12\r\nC3\t3\t2\t<=\t18\r\nLowerBound\t0\t0\t\t\r\nUpperBound\tM\tM\t\t\r\nVariableType\tContinuous\tContinuous\t\t\r\n";
    let m = Module::read_file(&Gp, lp.as_bytes()).unwrap();
    let r = Module::solve(&Gp, &m, &Value::Null).unwrap();
    assert_eq!(r.data["goal_values"][0]["value"], 36.0);
}

#[test]
fn no_factible_y_no_acotado() {
    let inf = json!({"name": "x", "variables": [{"name": "X"}], "goals": [{"name": "G", "objective": "max"}],
        "goal_coefs": [[1]], "constraints": [{"name": "C", "direction": ">=", "rhs": 5}, {"name": "D", "direction": "<=", "rhs": 3}],
        "coefs": [[1], [1]]});
    assert_eq!(Module::solve(&Gp, &inf, &Value::Null).unwrap().status, iaqsb_core::Status::Infeasible);
    let unb = json!({"name": "x", "variables": [{"name": "X"}], "goals": [{"name": "G", "objective": "max"}],
        "goal_coefs": [[1]], "constraints": [], "coefs": []});
    assert_eq!(Module::solve(&Gp, &unb, &Value::Null).unwrap().status, iaqsb_core::Status::Unbounded);
}
