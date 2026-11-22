// Integración con Excel, Google Sheets y LibreOffice (lado de la interfaz):
//  · pegar bloques de celdas en las hojas del editor (LP-ILP, matrices y registros),
//  · copiar las tablas de resultados como TSV/HTML para pegarlas en una hoja de cálculo,
//  · exportar el informe (Word, PDF, HTML, Excel o Markdown) y el modelo a .xlsx.
// La lógica pura del pegado está en pegar.ts.

import { mount, tick, unmount } from 'svelte';
import { ajustes } from './ajustes.svelte';
import { b64Blob, call, download, inTauri } from './backend';
import { i18n, t } from './i18n.svelte';
import { ui } from './ui.svelte';
import { app } from './state.svelte';
import type { Field } from './gtypes';
import {
  analizarLP,
  analizarMatriz,
  analizarRegistros,
  detectarComa,
  leerTabla,
  pegarLP,
  pegarMatriz,
  pegarRegistros,
  type Celda,
  type Eleccion,
  type Falta,
  type MatrixW,
  type RecordsW,
  type Resultado,
  type Tabla
} from './pegar';
import PegarOferta from '../components/PegarOferta.svelte';

// ------------------------------------------------------------- separador decimal

/** ¿La configuración regional del equipo usa coma decimal? (Excel copia y pega con ella.) */
export function comaDelSistema(): boolean {
  try {
    return new Intl.NumberFormat(navigator.language).formatToParts(1.5).find((p) => p.type === 'decimal')?.value === ',';
  } catch {
    return false;
  }
}

/** Coma decimal para lo que va y viene del portapapeles. */
export function comaPortapapeles(): boolean {
  if (ajustes.separador === 'coma') return true;
  if (ajustes.separador === 'punto') return false;
  return comaDelSistema();
}

/** Coma decimal en los informes: la de los Ajustes; en automático, la del español y el portugués. */
export function comaInformes(): boolean {
  if (ajustes.separador === 'coma') return true;
  if (ajustes.separador === 'punto') return false;
  return i18n.lang === 'es' || i18n.lang === 'pt';
}

// ------------------------------------------------------------- diálogo «el bloque no cabe»

export interface Opcion {
  id: string;
  etiqueta: string;
  detalle?: string;
}

/** Pregunta qué hacer y devuelve la opción elegida (null si se cancela). */
export function elegir(titulo: string, texto: string, opciones: Opcion[]): Promise<string | null> {
  return new Promise((resolve) => {
    const caja = mount(PegarOferta, {
      target: document.body,
      props: {
        titulo,
        texto,
        opciones,
        cerrar: (id: string | null) => {
          void unmount(caja);
          resolve(id);
        }
      }
    });
  });
}

function cuantas(n: number, uno: string, varios: string): string {
  return n === 1 ? t(uno) : t(varios, n);
}

function juntar(partes: string[]): string {
  return partes.length > 1 ? t('{0} y {1}', partes.slice(0, -1).join(', '), partes[partes.length - 1]) : (partes[0] ?? '');
}

/** Ofrece crecer la hoja si el bloque no cabe. Devuelve null si se cancela. */
async function ofrecer(tabla: Tabla, falta: Falta, partes: (f: Falta) => string[], ld?: () => string[]): Promise<Eleccion | null> {
  if (!falta.filas && !falta.columnas) return { crecer: false };
  const txt = juntar(partes(falta));
  const opciones: Opcion[] = [{ id: 'crecer', etiqueta: t('Añadir {0} y pegar todo', txt), detalle: t('La hoja crece para que quepa el bloque completo.') }];
  if (falta.ultimaLD && ld) {
    const resto = ld();
    opciones.push({
      id: 'ld',
      etiqueta: t('La última columna es el lado derecho'),
      detalle: resto.length ? t('Se pega como lado derecho y se añade {0}.', juntar(resto)) : t('Se pega como lado derecho de las restricciones.')
    });
  }
  opciones.push({ id: 'cabe', etiqueta: t('Pegar solo lo que cabe'), detalle: t('Lo que sobra a la derecha o abajo no se pega.') });
  const id = await elegir(
    t('El bloque es más grande que la hoja'),
    t('Copiaste {0} × {1} celdas y desde esta celda falta espacio para {2}.', tabla.length, tabla[0]?.length ?? 0, txt),
    opciones
  );
  if (!id) return null;
  return id === 'crecer' ? { crecer: true } : id === 'ld' ? { crecer: true, ultimaLD: true } : { crecer: false };
}

// ------------------------------------------------------------- pegar

/** Copia simple de un modelo (también de un proxy de $state). */
function plano<T>(x: T): T {
  return JSON.parse(JSON.stringify(x));
}

/** Texto pegado como tabla; null si no hay nada que hacer aquí (se deja el pegado normal). */
function tablaDe(e: ClipboardEvent, textoLibre: boolean): Tabla | null {
  const txt = e.clipboardData?.getData('text/plain') ?? '';
  if (!txt.trim()) return null;
  const tabla = leerTabla(txt);
  if (!tabla.length || !tabla[0].length) return null;
  if (tabla.length === 1 && tabla[0].length === 1) {
    // una sola celda: en un nombre o un texto, o con parte del texto seleccionado, se pega normal
    const el = e.target;
    if (textoLibre) return null;
    if (el instanceof HTMLInputElement && el.type !== 'checkbox') {
      const todo = el.value === '' || (el.selectionStart === 0 && el.selectionEnd === el.value.length);
      if (!todo) return null;
    }
  }
  return tabla;
}

function numAttr(el: Element, k: string): number | null {
  const v = (el as HTMLElement).dataset?.[k];
  if (v === undefined || v === '') return null;
  const n = Number(v);
  return Number.isFinite(n) ? n : null;
}

/** Marca un momento las celdas recién pegadas. */
async function resaltar(caja: HTMLElement | null, celdas: Celda[], aDom: (c: Celda) => string | null) {
  if (!caja || !celdas.length) return;
  estilos();
  await tick();
  requestAnimationFrame(() => {
    for (const c of celdas.slice(0, 4000)) {
      const sel = aDom(c);
      const el = sel ? caja.querySelector<HTMLElement>(sel) : null;
      const td = el?.closest<HTMLElement>('td, th');
      if (!td) continue;
      td.classList.remove('iaqsb-pegado');
      void td.offsetWidth;
      td.classList.add('iaqsb-pegado');
      td.addEventListener('animationend', () => td.classList.remove('iaqsb-pegado'), { once: true });
    }
  });
}

function avisarPegado(tabla: Tabla, r: Resultado<unknown>) {
  const extra: string[] = [];
  if (r.nuevasFilas || r.nuevasColumnas) extra.push(t('la hoja creció'));
  if (r.renombradas) extra.push(cuantas(r.renombradas, '1 nombre nuevo', '{0} nombres nuevos'));
  const base = t('Se pegaron {0} × {1} celdas', tabla.length, tabla[0]?.length ?? 0);
  ui.avisar(extra.length ? `${base} (${extra.join(', ')})` : base);
  const malas: string[] = [];
  if (r.invalidas) malas.push(cuantas(r.invalidas, '1 celda no se pudo leer y quedó como estaba', '{0} celdas no se pudieron leer y quedaron como estaban'));
  if (r.fuera) malas.push(cuantas(r.fuera, '1 celda quedó fuera de la hoja', '{0} celdas quedaron fuera de la hoja'));
  if (malas.length) ui.avisar(malas.join(' · '), 'info');
}

/** Hoja de LP-ILP (ModelSheet): onpaste del contenedor de la hoja. */
export async function pegarEnLP(e: ClipboardEvent) {
  const el = e.target as HTMLElement;
  const dr = numAttr(el, 'r');
  const dc = numAttr(el, 'c');
  if (dr === null || dc === null) return;
  const caja = e.currentTarget as HTMLElement;
  const n = app.model.variables.length;
  const nr = app.model.constraints.length;
  // coordenadas de la hoja → lógicas (sin la fila de «+ Restricción» ni la columna de «+ Variable»)
  const r = dr <= nr + 1 ? dr : dr - 1;
  const c = dc <= n ? dc : dc - 1;
  const tabla = tablaDe(e, r === 0 || c === 0);
  if (!tabla) return;
  e.preventDefault();
  const coma = detectarComa(tabla, comaPortapapeles());
  const a0 = analizarLP(plano(app.model), r, c, tabla, coma);
  const el0 = await ofrecer(
    tabla,
    a0.falta,
    (f) => [...(f.columnas ? [cuantas(f.columnas, '1 variable', '{0} variables')] : []), ...(f.filas ? [cuantas(f.filas, '1 restricción', '{0} restricciones')] : [])],
    () => [...(a0.falta.columnas > 1 ? [cuantas(a0.falta.columnas - 1, '1 variable', '{0} variables')] : []), ...(a0.falta.filas ? [cuantas(a0.falta.filas, '1 restricción', '{0} restricciones')] : [])]
  );
  if (!el0) return;
  // el modelo pudo cambiar mientras se elegía: se analiza otra vez
  const actual = plano(app.model);
  const res = pegarLP(actual, analizarLP(actual, r, c, tabla, coma), el0);
  // un solo cambio del modelo (se deshace de una vez)
  app.model = res.modelo;
  app.changed();
  avisarPegado(tabla, res);
  const n2 = res.modelo.variables.length;
  const nr2 = res.modelo.constraints.length;
  resaltar(caja, res.escritas, ([lr, lc]) => `[data-r="${lr <= nr2 + 1 ? lr : lr + 1}"][data-c="${lc <= n2 ? lc : lc + 1}"]`);
}

/** Matriz de un módulo genérico (MatrixGrid). */
export async function pegarEnMatriz(e: ClipboardEvent, w: MatrixW, hermanas: MatrixW[], model: any, onchange: () => void) {
  const el = e.target as HTMLElement;
  const caja = e.currentTarget as HTMLElement;
  const rows = (model[w.rows.path] ?? []) as any[];
  const cols = (model[w.cols.path] ?? []) as any[];
  const R = rows.length;
  const C = cols.length;
  let r: number | null;
  let c: number | null;
  let campo: Field | null = null;
  const hc = numAttr(el, 'hc');
  const hr = numAttr(el, 'hr');
  if (hc !== null) [r, c] = [-1, hc];
  else if (hr !== null) [r, c] = [hr, -1];
  else {
    r = numAttr(el, 'r');
    c = numAttr(el, 'c');
    if (r === null || c === null) return;
    // los datos extra de las filas van después de la columna de «+ …»
    if (c > C) c -= 1;
    campo = r >= R ? (w.cols.extra[r - R] ?? null) : c >= C ? (w.rows.extra[c - C] ?? null) : w.cell;
  }
  const textoLibre = r === -1 || c === -1 || !campo || ['text', 'expr', 'list'].includes(campo.kind.type);
  const tabla = tablaDe(e, textoLibre);
  if (!tabla) return;
  e.preventDefault();
  const coma = detectarComa(tabla, comaPortapapeles());
  const square = w.rows.path === w.cols.path;
  const puedeF = !!w.rows.add_label;
  const puedeC = !!w.cols.add_label && !square;
  const a0 = analizarMatriz(w, plano(model), r, c, tabla, coma, puedeF, puedeC);
  const el0 = await ofrecer(tabla, a0.falta, (f) =>
    square
      ? [`${cuantas(f.filas, '1 fila y columna', '{0} filas y columnas')} (${w.rows.label})`]
      : [
          ...(f.columnas ? [`${cuantas(f.columnas, '1 columna', '{0} columnas')} (${w.cols.label})`] : []),
          ...(f.filas ? [`${cuantas(f.filas, '1 fila', '{0} filas')} (${w.rows.label})`] : [])
        ]
  );
  if (!el0) return;
  const actual = plano(model);
  const res = pegarMatriz(actual, w, hermanas, analizarMatriz(w, actual, r, c, tabla, coma, puedeF, puedeC), el0, { filas: w.rows.label, cols: w.cols.label });
  for (const k of res.claves) model[k] = res.modelo[k];
  onchange();
  avisarPegado(tabla, res);
  const C2 = (res.modelo[w.cols.path] ?? []).length;
  resaltar(caja, res.escritas, ([i, j]) => (i === -1 ? `[data-hc="${j}"]` : j === -1 ? `[data-hr="${i}"]` : `[data-r="${i}"][data-c="${j < C2 ? j : j + 1}"]`));
}

/** Lista con columnas de un módulo genérico (RecordsGrid). `columnas`: las visibles, en orden. */
export async function pegarEnRegistros(e: ClipboardEvent, w: RecordsW, columnas: Field[], model: any, onchange: () => void) {
  const el = e.target as HTMLElement;
  const caja = e.currentTarget as HTMLElement;
  let r = numAttr(el, 'r');
  let c = numAttr(el, 'c');
  if (r === null || c === null) {
    // sin celda (p. ej. la lista vacía y el foco en «+ Añadir»): se pega al final
    if (!caja.contains(el)) return;
    r = (model[w.path] ?? []).length;
    c = 0;
  }
  const campo = columnas[c as number];
  const tabla = tablaDe(e, !campo || ['text', 'expr', 'list'].includes(campo.kind.type));
  if (!tabla) return;
  e.preventDefault();
  const coma = detectarComa(tabla, comaPortapapeles());
  const a0 = analizarRegistros(plano(model), w, columnas, r as number, c as number, tabla, coma, !!w.add_label);
  const el0 = await ofrecer(tabla, a0.falta, (f) => [cuantas(f.filas, '1 fila', '{0} filas')]);
  if (!el0) return;
  const actual = plano(model);
  const res = pegarRegistros(actual, w, columnas, analizarRegistros(actual, w, columnas, r as number, c as number, tabla, coma, !!w.add_label), el0, t('Elemento'));
  model[w.path] = res.modelo;
  onchange();
  avisarPegado(tabla, res);
  resaltar(caja, res.escritas, ([i, j]) => `[data-r="${i}"][data-c="${j}"]`);
}

// ------------------------------------------------------------- copiar tablas

function celdaTexto(td: HTMLTableCellElement, coma: boolean): string {
  let s = (td.innerText ?? td.textContent ?? '').replace(/\s+/g, ' ').trim();
  if (s === '–' || s === '—') return '';
  // números como los muestra la app (signo «−», punto decimal) → como los lee Excel en este equipo
  // (con el separador «coma» de los Ajustes, la app ya los muestra con coma)
  if (/^[−-]?\d+([.,]\d+)?(e[+-]?\d+)?$/i.test(s)) {
    s = s.replace('−', '-').replace(',', '.');
    if (coma) s = s.replace('.', ',');
  } else if (/^[−-]?∞$/.test(s)) s = s.replace('−', '-');
  return s;
}

function filasDe(tabla: HTMLTableElement): string[][] {
  const coma = comaPortapapeles();
  const out: string[][] = [];
  for (const tr of Array.from(tabla.rows)) {
    if (tr.closest('table') !== tabla) continue;
    const fila: string[] = [];
    for (const td of Array.from(tr.cells)) {
      fila.push(celdaTexto(td, coma));
      for (let k = 1; k < td.colSpan; k++) fila.push('');
    }
    if (fila.some((x) => x !== '')) out.push(fila);
  }
  return out;
}

function esc(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

/** Copia una tabla al portapapeles como TSV (Excel, Sheets) y HTML (Word, Excel con formato). */
export async function copiarTabla(tabla: HTMLTableElement): Promise<void> {
  const filas = filasDe(tabla);
  const tsv = filas.map((f) => f.map((c) => (/[\t\n"]/.test(c) ? `"${c.replace(/"/g, '""')}"` : c)).join('\t')).join('\r\n');
  const cab = tabla.tHead ? tabla.tHead.rows.length : 0;
  const html =
    '<meta charset="utf-8"><table border="1" style="border-collapse:collapse">' +
    filas.map((f, i) => '<tr>' + f.map((c) => (i < cab ? `<th>${esc(c)}</th>` : `<td>${esc(c)}</td>`)).join('') + '</tr>').join('') +
    '</table>';
  try {
    await navigator.clipboard.write([
      new ClipboardItem({ 'text/plain': new Blob([tsv], { type: 'text/plain' }), 'text/html': new Blob([html], { type: 'text/html' }) })
    ]);
  } catch {
    await navigator.clipboard.writeText(tsv);
  }
}

/**
 * Acción de Svelte para el contenedor de una tabla: al pasar el mouse aparece
 * «Copiar» (a la derecha de la tabla, o encima si no hay espacio).
 *   <div class="tw" use:copiable> <table>…</table> </div>
 */
export function copiable(node: HTMLElement) {
  estilos();
  const padre = node.parentElement;
  if (!padre) return {};
  if (getComputedStyle(padre).position === 'static') padre.style.position = 'relative';
  const btn = document.createElement('button');
  btn.type = 'button';
  btn.className = 'iaqsb-copiar';
  let idioma = '';
  // se vuelve a pintar solo si cambió el idioma (cambiar el contenido durante un clic lo anula)
  const pintar = () => {
    if (idioma === i18n.lang) return;
    idioma = i18n.lang;
    btn.title = t('Copiar la tabla para pegarla en Excel, Google Sheets o Word');
    btn.setAttribute('aria-label', t('Copiar tabla'));
    btn.innerHTML = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M9 9h11v11H9zM5 15H4V4h11v1"/></svg><span>${esc(t('Copiar'))}</span>`;
  };
  pintar();
  padre.appendChild(btn);
  let ocultar: ReturnType<typeof setTimeout> | undefined;
  const colocar = () => {
    const izq = node.offsetLeft + node.offsetWidth + 8;
    if (izq + btn.offsetWidth <= padre.clientWidth) {
      btn.style.left = `${izq}px`;
      btn.style.top = `${node.offsetTop}px`;
    } else {
      btn.style.left = `${Math.max(0, node.offsetLeft + node.offsetWidth - btn.offsetWidth)}px`;
      btn.style.top = `${Math.max(0, node.offsetTop - btn.offsetHeight - 4)}px`;
    }
  };
  const mostrar = () => {
    clearTimeout(ocultar);
    pintar();
    btn.classList.add('ver');
    colocar();
  };
  const esconder = () => {
    clearTimeout(ocultar);
    ocultar = setTimeout(() => btn.classList.remove('ver'), 220);
  };
  const copiar = async () => {
    const tabla = node.querySelector('table');
    if (!tabla) return;
    try {
      await copiarTabla(tabla);
      ui.avisar(t('Tabla copiada: pégala en Excel, Google Sheets o Word'));
    } catch (e) {
      ui.avisar(t('No se pudo copiar al portapapeles') + (e instanceof Error ? `: ${e.message}` : ''), 'error');
    }
  };
  node.addEventListener('mouseenter', mostrar);
  node.addEventListener('mouseleave', esconder);
  node.addEventListener('focusin', mostrar);
  btn.addEventListener('mouseenter', mostrar);
  btn.addEventListener('mouseleave', esconder);
  btn.addEventListener('focus', mostrar);
  btn.addEventListener('blur', esconder);
  btn.addEventListener('click', copiar);
  return {
    destroy() {
      clearTimeout(ocultar);
      node.removeEventListener('mouseenter', mostrar);
      node.removeEventListener('mouseleave', esconder);
      node.removeEventListener('focusin', mostrar);
      btn.remove();
    }
  };
}

let conEstilos = false;
/** Estilos globales del resaltado de lo pegado y del botón «Copiar» (con las variables del tema). */
function estilos() {
  if (conEstilos || typeof document === 'undefined') return;
  conEstilos = true;
  const st = document.createElement('style');
  st.dataset.iaqsb = 'excel';
  st.textContent = `
.iaqsb-pegado { animation: iaqsb-pegado 1.3s ease-out; }
@keyframes iaqsb-pegado {
  0%, 35% { background-color: var(--solver-tint); box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--solver) 55%, transparent); }
  100% { box-shadow: inset 0 0 0 1px transparent; }
}
.iaqsb-copiar {
  position: absolute; z-index: 4; display: inline-flex; align-items: center; gap: 5px;
  height: 26px; padding: 0 9px; border-radius: 7px; border: 0.5px solid var(--rule);
  background: color-mix(in oklab, var(--sheet) 86%, transparent);
  -webkit-backdrop-filter: var(--vidrio); backdrop-filter: var(--vidrio);
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.12); color: var(--ink-2);
  font-size: 11.5px; font-weight: 600; white-space: nowrap;
  opacity: 0; pointer-events: none; transform: translateY(-2px); transition: opacity 0.15s, transform 0.15s;
}
.iaqsb-copiar.ver, .iaqsb-copiar:focus-visible { opacity: 1; pointer-events: auto; transform: none; }
.iaqsb-copiar:hover { color: var(--ink); background: var(--sheet); }
`;
  document.head.appendChild(st);
}

// ------------------------------------------------------------- exportar

export type Formato = 'docx' | 'pdf' | 'html' | 'xlsx' | 'md';

export const FORMATOS: { id: Formato; nombre: string; ext: string; desc: string; filtro: string }[] = [
  { id: 'docx', nombre: 'Word', ext: 'docx', desc: 'Editable, con portada, tablas y figuras', filtro: 'Documento de Word' },
  { id: 'pdf', nombre: 'PDF', ext: 'pdf', desc: 'Listo para imprimir o entregar', filtro: 'Documento PDF' },
  { id: 'html', nombre: 'HTML', ext: 'html', desc: 'Se abre en cualquier navegador', filtro: 'Página web' },
  { id: 'xlsx', nombre: 'Excel', ext: 'xlsx', desc: 'Una hoja por cada tabla de resultados', filtro: 'Libro de Excel' },
  { id: 'md', nombre: 'Markdown', ext: 'md', desc: 'Texto plano para una IA o un repositorio', filtro: 'Markdown' }
];

export interface ArgsInforme {
  module: string;
  report?: unknown;
  model?: unknown;
  options?: Record<string, unknown>;
  format: Formato;
  steps: boolean;
  author: string;
  course: string;
  institution: string;
  decimal_comma: boolean;
}

/** Nombre de archivo seguro (como el del núcleo). */
export function nombreArchivo(s: string, ext: string): string {
  const base = (s || t('informe')).replace(/[\\/:*?"<>|]/g, '').replace(/\s+/g, ' ').trim().slice(0, 80) || t('informe');
  return `${base}.${ext}`;
}

/** Genera el informe y lo guarda (Tauri: diálogo de guardar; navegador: descarga). Devuelve el nombre o null. */
export async function exportarInforme(args: ArgsInforme, nombre: string): Promise<string | null> {
  const f = FORMATOS.find((x) => x.id === args.format)!;
  if (inTauri) {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const path = await save({ defaultPath: nombreArchivo(nombre, f.ext), filters: [{ name: `${t(f.filtro)} (.${f.ext})`, extensions: [f.ext] }] });
    if (!path) return null;
    const { invoke } = await import('@tauri-apps/api/core');
    // el núcleo genera el archivo y lo escribe: el base64 no pasa por la interfaz
    await invoke('save_export', { path, op: 'export_report', args: { ...args, lang: i18n.lang } });
    return path;
  }
  const r = await call<{ base64?: string; content?: string; filename?: string; mime?: string }>('export_report', { ...args });
  const mime = r.mime ?? 'application/octet-stream';
  const blob = r.base64 !== undefined ? b64Blob(r.base64, mime) : new Blob([r.content ?? ''], { type: `${mime};charset=utf-8` });
  return download(blob, r.filename ?? nombreArchivo(nombre, f.ext));
}
