// Puente con el núcleo en Rust. Dentro de Tauri usa `invoke`; en el navegador
// (modo de desarrollo) usa el servidor de tools/dev_server.py.

import type { LpModel } from './types';
import { i18n, t } from './i18n.svelte';

export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

async function invoke<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

export async function call<T = unknown>(op: string, args: Record<string, unknown> = {}): Promise<T> {
  // el núcleo devuelve informes, ejemplos y fuentes en el idioma de la interfaz
  args = { ...args, lang: i18n.lang };
  if (inTauri) {
    try {
      return await invoke<T>('api_call', { op, args });
    } catch (e) {
      throw new Error(String(e));
    }
  }
  const r = await fetch('/api/call', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ op, args })
  });
  const j = await r.json();
  if (!j.ok) throw new Error(j.error?.message ?? t('Error desconocido'));
  return j.result as T;
}

export interface Imported {
  module: string;
  layout: 'matrix' | 'normal';
  model: any;
  path?: string;
  /** Modelo MPS/LP demasiado grande para la hoja (model = {name, format, text}). */
  large?: boolean;
  size?: { cols: number; rows: number; nonzeros: number; integer: number };
  /** Modelo en el lenguaje algebraico de IAQSB (model = {format: 'iaqsb', text, name}). */
  algebra?: boolean;
}

/** Módulo y tipo de problema abiertos (para leer un libro de Excel que no hizo IAQSB). */
export interface XlsxCtx {
  module: string;
  problemType?: string | null;
}

/** Bytes a base64 (por trozos, para libros grandes). */
export function bytesToBase64(bytes: Uint8Array): string {
  let s = '';
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s);
}

const SIN_MODULO = /indica el m[oó]dulo|\(module\)/i;
const XLSX_AJENO = () => t('Este libro de Excel no lo hizo IAQSB y no se pudo leer como un problema de este módulo. Abre el módulo que corresponde o copia las tablas en Excel y pégalas en el editor (Ctrl+V).');

/** Lee un libro .xlsx: el que hizo IAQSB trae su módulo; uno ajeno se lee con el módulo abierto. */
async function importXlsx(bytes: Uint8Array, ctx?: XlsxCtx): Promise<Imported> {
  const base64 = bytesToBase64(bytes);
  try {
    return await call<Imported>('import_xlsx', { base64 });
  } catch (e) {
    if (!ctx || !SIN_MODULO.test(String(e instanceof Error ? e.message : e))) throw e;
  }
  try {
    return await call<Imported>('import_xlsx', { base64, module: ctx.module, ...(ctx.problemType ? { problem_type: ctx.problemType } : {}) });
  } catch (e) {
    throw new Error(`${XLSX_AJENO()} (${e instanceof Error ? e.message : e})`);
  }
}

/** En Tauri, read_file lee los .xlsx de IAQSB; para uno ajeno se le pasa el módulo abierto. */
async function readFileTauri(path: string, ctx?: XlsxCtx): Promise<Imported> {
  try {
    return await invoke<Imported>('read_file', { path });
  } catch (e) {
    const msg = String(e);
    if (!ctx || !/\.xlsx$/i.test(path) || !SIN_MODULO.test(msg)) throw e;
    try {
      // read_file(path, module, problem_type): el núcleo lo acepta cuando el comando recibe el módulo
      return await invoke<Imported>('read_file', { path, module: ctx.module, problemType: ctx.problemType ?? null });
    } catch {
      throw new Error(XLSX_AJENO());
    }
  }
}

/** Abre un archivo de WinQSB (de cualquier módulo), un modelo .json de IAQSB o un libro de Excel. */
export async function openFile(exts: string[] = ['LPP'], ctx?: XlsxCtx): Promise<Imported | null> {
  if (inTauri) {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const all = [...new Set(exts.flatMap((e) => [e.toLowerCase(), e.toUpperCase()]))];
    const path = await open({
      multiple: false,
      filters: [
        { name: t('Archivos de WinQSB e IAQSB'), extensions: [...all, 'json', 'dat', 'DAT', 'txt', 'TXT', 'mps', 'MPS', 'lp', 'iaqsb', 'IAQSB', 'xlsx', 'XLSX'] },
        { name: t('Libros de Excel (.xlsx)'), extensions: ['xlsx', 'XLSX'] },
        { name: t('Modelos matemáticos (MPS, LP)'), extensions: ['mps', 'MPS', 'lp', 'LP'] },
        { name: t('Modelos algebraicos (.iaqsb)'), extensions: ['iaqsb', 'IAQSB', 'ml'] },
        { name: t('Todos los archivos'), extensions: ['*'] }
      ]
    });
    if (!path || Array.isArray(path)) return null;
    const res = await readFileTauri(path, ctx);
    return { ...res, path };
  }
  return new Promise((resolve, reject) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = [...exts.map((e) => '.' + e), '.json', '.txt', '.dat', '.mps', '.lp', '.iaqsb', '.ml', '.xlsx'].join(',');
    input.onchange = async () => {
      const f = input.files?.[0];
      if (!f) return resolve(null);
      try {
        const buf = new Uint8Array(await f.arrayBuffer());
        if (/\.xlsx$/i.test(f.name)) return resolve({ ...(await importXlsx(buf, ctx)), path: f.name });
        let text: string;
        try {
          text = new TextDecoder('utf-8', { fatal: true }).decode(buf);
        } catch {
          text = new TextDecoder('windows-1252').decode(buf);
        }
        if (/\.(iaqsb|mod|ml)$/i.test(f.name)) {
          // modelo en el lenguaje algebraico: el texto va tal cual al editor
          const name = f.name.replace(/\.[^.]*$/, '');
          resolve({ module: 'lp', layout: 'matrix', algebra: true, model: { format: 'iaqsb', text, name }, path: f.name });
        } else if (f.name.toLowerCase().endsWith('.json')) {
          const v = JSON.parse(text);
          resolve({ module: v.module ?? 'lp', layout: 'matrix', model: v.model ?? v, path: f.name });
        } else {
          const r = await call<Imported>('import_text', { content: text });
          resolve({ ...r, path: f.name });
        }
      } catch (e) {
        reject(e);
      }
    };
    input.click();
  });
}

function latin1(text: string): Uint8Array {
  const out = new Uint8Array(text.length);
  for (let i = 0; i < text.length; i++) {
    const c = text.charCodeAt(i);
    out[i] = c < 256 ? c : 63;
  }
  return out;
}

/** Guarda el modelo en formato WinQSB (.LPP), MPS o LP (según `layout`). Devuelve la ruta o el nombre usado. */
export async function saveLpp(model: LpModel, layout: 'matrix' | 'normal' | 'mps' | 'lp' | 'xlsx', current?: string): Promise<string | null> {
  if (layout === 'xlsx') return saveXlsx('lp', model, current);
  const base = (model.name || t('problema')).replace(/[^\p{L}\p{N} _-]/gu, '').trim() || t('problema');
  const ext = layout === 'mps' ? 'mps' : layout === 'lp' ? 'lp' : 'LPP';
  const filter =
    layout === 'mps'
      ? { name: t('Modelo MPS (cualquier motor)'), extensions: ['mps'] }
      : layout === 'lp'
        ? { name: t('Modelo LP (formato de CPLEX)'), extensions: ['lp'] }
        : { name: t('Problema de LP-ILP (WinQSB)'), extensions: ['LPP'] };
  if (inTauri) {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const keep = current && current.toLowerCase().endsWith('.' + ext.toLowerCase());
    const path = await save({
      defaultPath: keep ? current : `${base}.${ext}`,
      filters: [filter]
    });
    if (!path) return null;
    await invoke('write_file', { path, module: 'lp', model, layout });
    return path;
  }
  const { content } = await call<{ content: string }>('export_text', { module: 'lp', model, layout });
  const bytes = ext === 'LPP' ? latin1(content) : new TextEncoder().encode(content);
  const blob = new Blob([bytes as BlobPart], { type: 'application/octet-stream' });
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = `${base}.${ext}`;
  a.click();
  URL.revokeObjectURL(a.href);
  return a.download;
}

/** Guarda cualquier modelo en el formato de WinQSB del módulo (o .json). */
export async function saveModel(module: string, model: any, ext: string, current?: string): Promise<string | null> {
  const base = (model.name || t('problema')).replace(/[^\p{L}\p{N} _-]/gu, '').trim() || t('problema');
  if (inTauri) {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const path = await save({
      defaultPath: current && !current.toLowerCase().endsWith('.json') ? current : `${base}.${ext}`,
      filters: [
        { name: t('Archivo de WinQSB (.{0})', ext), extensions: [ext] },
        { name: t('Libro de Excel (.xlsx)'), extensions: ['xlsx'] },
        { name: t('Modelo de IAQSB (.json)'), extensions: ['json'] }
      ]
    });
    if (!path) return null;
    await invoke('write_file', { path, module, model, layout: null, lang: i18n.lang });
    return path;
  }
  if (current?.toLowerCase().endsWith('.xlsx')) return saveXlsx(module, model, current);
  const { content } = await call<{ content: string }>('export_text', { module, model });
  const blob = new Blob([latin1(content) as BlobPart], { type: 'application/octet-stream' });
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = `${base}.${ext}`;
  a.click();
  URL.revokeObjectURL(a.href);
  return a.download;
}

/** Descarga un archivo en el modo navegador (sin Tauri). */
export function download(blob: Blob, name: string): string {
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
  return name;
}

/** Guarda el modelo de cualquier módulo como libro de Excel (una hoja por tabla del editor). */
export async function saveXlsx(module: string, model: any, current?: string): Promise<string | null> {
  const base = (model.name || t('problema')).replace(/[^\p{L}\p{N} _-]/gu, '').trim() || t('problema');
  if (inTauri) {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const path = await save({
      defaultPath: current?.toLowerCase().endsWith('.xlsx') ? current : `${base}.xlsx`,
      filters: [{ name: t('Libro de Excel (.xlsx)'), extensions: ['xlsx'] }]
    });
    if (!path) return null;
    await invoke('write_file', { path, module, model, layout: null, lang: i18n.lang });
    return path;
  }
  const r = await call<{ base64: string; filename?: string }>('export_xlsx', { module, model });
  return download(b64Blob(r.base64, 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'), `${base}.xlsx`);
}

export interface McpInfo {
  server_path: string;
  cli_path: string;
  config: string;
}

export async function mcpInfo(): Promise<McpInfo> {
  if (inTauri) return invoke<McpInfo>('mcp_info', {});
  const server_path = 'C:\\Program Files\\IAQSB\\iaqsb-mcp.exe';
  return {
    server_path,
    cli_path: 'C:\\Program Files\\IAQSB\\iaqsb.exe',
    config: JSON.stringify({ mcpServers: { iaqsb: { command: server_path } } }, null, 2)
  };
}

/** Abre un enlace (DOI, URL) en el navegador del sistema. */
export async function openExternal(url: string): Promise<void> {
  if (inTauri) {
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    await openUrl(url);
  } else {
    window.open(url, '_blank', 'noopener');
  }
}

export function b64Blob(b64: string, type: string): Blob {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return new Blob([out as BlobPart], { type });
}

function fileBase(name: string): string {
  return (name || t('grafica')).replace(/[^\p{L}\p{N} _-]/gu, '').trim().slice(0, 60) || t('grafica');
}

/** Guarda una gráfica del informe en PNG (alta resolución) o SVG. */
export async function saveGraphic(block: unknown, name: string, format: 'png' | 'svg'): Promise<string | null> {
  const base = fileBase(name);
  if (inTauri) {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const path = await save({
      defaultPath: `${base}.${format}`,
      filters: [{ name: format === 'png' ? t('Imagen PNG') : t('Imagen vectorial SVG'), extensions: [format] }]
    });
    if (!path) return null;
    await invoke('save_render', { path, block, lang: i18n.lang });
    return path;
  }
  const r = await call<{ png?: string; svg?: string }>('render', { block, format, scale: 2 });
  const blob = format === 'png' ? b64Blob(r.png ?? '', 'image/png') : new Blob([r.svg ?? ''], { type: 'image/svg+xml' });
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = `${base}.${format}`;
  a.click();
  URL.revokeObjectURL(a.href);
  return a.download;
}

/** Copia la gráfica al portapapeles como imagen PNG (para pegarla en Word o en un chat). */
export async function copyGraphic(block: unknown): Promise<void> {
  const r = await call<{ png: string }>('render', { block, format: 'png', scale: 2 });
  const blob = b64Blob(r.png, 'image/png');
  await navigator.clipboard.write([new ClipboardItem({ 'image/png': blob })]);
}
