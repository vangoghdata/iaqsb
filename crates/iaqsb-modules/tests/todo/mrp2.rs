//! MRP II por matrices (Gozinto): el ejemplo de los estantes del curso, comparado
//! con el mismo cálculo hecho aparte con numpy, y la lista de materiales directa.

use iaqsb_core::Module;
use iaqsb_modules::mrp2::Mrp2;
use serde_json::{json, Value};

fn ex() -> Value {
    Module::examples(&Mrp2).into_iter().find(|(n, _)| n == "estantes_arango").unwrap().1
}

fn mat(v: &Value) -> Vec<Vec<f64>> {
    v.as_array().unwrap().iter().map(|r| r.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect()).collect()
}

fn cerca(a: &[Vec<f64>], b: &[Vec<f64>]) {
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b) {
        for (u, v) in x.iter().zip(y) {
            assert!((u - v).abs() < 1e-6, "{a:?}\n≠\n{b:?}");
        }
    }
}

#[test]
fn estantes_igual_a_numpy() {
    let r = Module::solve(&Mrp2, &ex(), &Value::Null).unwrap();
    // RB = LM × Ds (numpy)
    cerca(&mat(&r.data["gross"])[2..4], &[vec![3900., 4200., 4500.], vec![5200., 5600., 6000.]]);
    // LM × Dneta antes de repartir: anaquel −6800, 3900, 8200; lámina −70400, 18100, 49800
    let np = mat(&r.data["to_schedule"]);
    cerca(
        &np,
        &[
            vec![0., 400., 800.],
            vec![200., 450., 800.],
            vec![0., 0., 5300.],
            vec![0., 5200., 11100.],
            vec![0., 4400., 14800.],
            vec![0., 0., 19200.],
            vec![0., 0., 4000.],
            vec![0., 0., 0.],
        ],
    );
    assert_eq!(r.data["surplus"][7].as_f64().unwrap(), 2500.0);
    // Tmq del departamento 2 en marzo = (0.75 × 5300 + 1 × 11100) / 60 = 251.25 h; Tres = 140 − 251.25
    let tres = mat(&r.data["capacity"]["residual_time"]);
    assert!((tres[1][2] + 111.25).abs() < 1e-9);
    assert_eq!(r.data["overloads"].as_array().unwrap().len(), 1);
    // RA = Fcal × NP
    let ra = mat(&r.data["quality_adjusted"]);
    assert!((ra[4][2] - 14800.0 * 1.11).abs() < 1e-6);
    assert_eq!(r.data["below_reorder_point"].as_array().unwrap().len(), 2);
}

#[test]
fn estructura_directa_da_lo_mismo() {
    let m = ex();
    let total = Module::solve(&Mrp2, &m, &Value::Null).unwrap();
    // la estructura directa de los estantes: la lámina va en el anaquel (3) y en la pata (2)
    let n = 8;
    let mut dir = vec![vec![0.0; n]; n];
    let lm = mat(&m["bom"]);
    for i in 0..n {
        for j in 0..n {
            if i != j {
                dir[i][j] = lm[i][j];
            }
        }
    }
    dir[7][0] = 0.0;
    dir[7][1] = 0.0;
    let mut m2 = m.clone();
    m2["bom"] = json!(dir);
    m2["bom_kind"] = json!("directa");
    let r = Module::solve(&Mrp2, &m2, &Value::Null).unwrap();
    cerca(&mat(&r.data["total_requirements"]), &lm);
    cerca(&mat(&r.data["to_schedule"]), &mat(&total.data["to_schedule"]));
    // un ciclo en la estructura directa es un error claro
    let mut m3 = m2.clone();
    m3["bom"][0][2] = json!(1.0);
    let e = Module::solve(&Mrp2, &m3, &Value::Null).unwrap_err().to_string();
    assert!(e.contains("ciclo"), "{e}");
}

#[test]
fn sin_repartir_y_archivo() {
    let r = Module::solve(&Mrp2, &ex(), &json!({"spread_negatives": false})).unwrap();
    assert_eq!(r.data["to_schedule"][2][0].as_f64().unwrap(), -6800.0);
    let b = Module::write_file(&Mrp2, &ex()).unwrap();
    assert_eq!(Module::read_file(&Mrp2, &b).unwrap(), ex());
}
