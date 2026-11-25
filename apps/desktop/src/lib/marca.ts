// Nombre del programa que ve el usuario (provisional). Cuando sea definitivo se cambia aquí y en
// MARCA de crates/iaqsb-core/src/i18n.rs. Los nombres internos (comando `iaqsb`, servidor MCP,
// formatos de archivo como «.iaqsb» o «IAQSB-ST», carpeta de datos) no cambian.
export const MARCA: string = 'Nativo';

/** Cambia «IAQSB» por el nombre del programa (no en «IAQSB-ST», «.IAQSB» ni dentro de palabras). */
export function marca(s: string): string {
  return MARCA === 'IAQSB' ? s : s.replace(/(?<![\w.\-])IAQSB(?![\w\-])/g, MARCA);
}
