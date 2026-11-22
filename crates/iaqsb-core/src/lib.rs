//! IAQSB · núcleo común.
//!
//! - `report`: bloques de informe que dibuja la aplicación.
//! - `spec`: descripción de las entradas de cada módulo (hojas y parámetros).
//! - `expr`: expresiones matemáticas al estilo de WinQSB.
//! - `lp`: programación lineal y entera con HiGHS.
//! - `module`: el contrato que cumple cada módulo.
//! - `biblio`: bibliografía (APA 7), citas de los informes y origen de los ejemplos.
//! - `winqsb`: utilidades para leer y escribir los archivos de WinQSB.

pub mod biblio;
pub mod expr;
pub mod i18n;
pub mod layout;
pub mod linear;
pub mod lp;
pub mod module;
pub mod report;
pub mod solver;
pub mod xlsx;
pub mod zip;
pub mod spec;
pub mod winqsb;

pub use module::*;
pub use report::*;
pub use spec::*;
