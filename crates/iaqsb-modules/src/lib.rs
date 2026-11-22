//! Módulos de IAQSB (todos menos LP-ILP, que vive en su propio crate).

pub mod alm;
pub mod ap;
pub mod asa;
pub mod da;
pub mod dp;
pub mod fc;
pub mod fin;
pub mod fll;
pub mod gp;
pub mod inv;
pub mod its;
pub mod job;
pub mod kpi;
pub mod mkp;
pub mod mrp;
pub mod mrp2;
pub mod net;
pub mod nlp;
pub mod nlpsolve;
pub mod pert;
pub mod pmp;
pub mod qa;
pub mod plot2d;
pub mod qcc;
pub mod qp;
pub mod qpsolve;
pub mod qss;
pub mod st;
pub mod stat;
pub mod modo;
pub mod vrp;

use iaqsb_core::Module;

/// Todos los módulos genéricos, en el orden del menú.
pub fn all() -> Vec<Box<dyn Module>> {
    vec![Box::new(net::Net), Box::new(pert::Pert), Box::new(gp::Gp), Box::new(qp::QpMod), Box::new(nlp::NlpMod), Box::new(dp::Dp), Box::new(ap::Ap), Box::new(pmp::Pmp), Box::new(da::Da), Box::new(mkp::Mkp), Box::new(its::Its), Box::new(inv::Inv), Box::new(fin::Fin), Box::new(alm::Alm), Box::new(kpi::Kpi), Box::new(fc::Fc), Box::new(st::St), Box::new(qa::Qa), Box::new(qss::Qss), Box::new(job::JobMod), Box::new(mrp::Mrp), Box::new(mrp2::Mrp2), Box::new(fll::Fll), Box::new(asa::Asa), Box::new(qcc::Qcc), Box::new(vrp::Vrp), Box::new(modo::Modo)]
}
