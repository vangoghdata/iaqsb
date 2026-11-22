// Evita la ventana de consola extra en Windows en las compilaciones de publicación.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    iaqsb_desktop_lib::run()
}
