// R y Python dentro de la app, sin instalar nada:
//
// - R: webR (https://webr.r-wasm.org), el R oficial compilado a WebAssembly, con los
//   paquetes de CRAN compilados para webR (forecast, readxl, ggplot2…). Da los mismos
//   resultados que el R de escritorio. Lo usan el módulo de series de tiempo (cuando el
//   equipo no tiene R con forecast: ver src-tauri/src/puentes.rs) y la herramienta run_r
//   del asistente.
// - Python: Pyodide (CPython compilado a WebAssembly) con numpy, pandas, scipy,
//   statsmodels, scikit-learn y matplotlib; la herramienta run_python del asistente.
//
// Se descargan la primera vez que se usan (R y forecast pesan unos 30 MB; después quedan
// en la caché) y corren aislados en la ventana: no pueden tocar los archivos del equipo.

import { inTauri } from './backend';
import { t } from './i18n.svelte';

const WEBR = '0.6.0';
const BASE_WEBR = `https://webr.r-wasm.org/v${WEBR}/`;
const PYODIDE = '0.29.5';
const BASE_PYODIDE = `https://cdn.jsdelivr.net/pyodide/v${PYODIDE}/full/`;
/** Paquetes de R que se instalan al iniciar (los demás, cuando el código los pide). */
const PAQUETES_R = ['forecast'];

export interface Archivo {
  nombre: string;
  datos: Uint8Array;
}

export interface Ejecucion {
  /** Lo que escribió el programa (salida y mensajes, en orden). */
  salida: string;
  /** Error que detuvo el programa (vacío si terminó bien). */
  error: string;
  /** Gráficas en PNG (data URL). */
  imagenes: string[];
  /** Versión del motor («R 4.6.0 (webR 0.6.0)»). */
  motor: string;
}

/** Estado visible: se muestra mientras se descarga cada motor. */
export const motoresWeb = $state({ r: '' as '' | 'cargando' | 'listo' | 'error', python: '' as '' | 'cargando' | 'listo' | 'error', detalle: '' });

// ---------------------------------------------------------------- R (webR)

let webR: Promise<any> | null = null;
let versionR = '';
const instalados = new Set<string>();

function cargarR(): Promise<any> {
  webR ??= (async () => {
    motoresWeb.r = 'cargando';
    motoresWeb.detalle = t('Descargando R (la primera vez tarda un poco)…');
    try {
      const mod = await import(/* @vite-ignore */ `${BASE_WEBR}webr.mjs`);
      const w = new mod.WebR({ baseUrl: BASE_WEBR, interactive: false });
      await w.init();
      motoresWeb.detalle = t('Instalando el paquete forecast de R…');
      await w.installPackages(PAQUETES_R, { quiet: true });
      PAQUETES_R.forEach((p) => instalados.add(p));
      versionR = String(await w.evalRString('paste(R.version$major, R.version$minor, sep = ".")'));
      motoresWeb.r = 'listo';
      motoresWeb.detalle = '';
      return w;
    } catch (e) {
      webR = null;
      motoresWeb.r = 'error';
      motoresWeb.detalle = '';
      throw new Error(t('No se pudo cargar R dentro de la app (¿hay conexión a internet?): {0}', e instanceof Error ? e.message : String(e)));
    }
  })();
  return webR;
}

/** Paquetes que usa un programa de R: library(x), require(x), x::f. */
function paquetesR(codigo: string): string[] {
  const v = new Set<string>();
  for (const m of codigo.matchAll(/\b(?:library|require|requireNamespace)\s*\(\s*["']?([A-Za-z][A-Za-z0-9.]*)["']?/g)) v.add(m[1]);
  for (const m of codigo.matchAll(/\b([A-Za-z][A-Za-z0-9.]*):::?[A-Za-z.]/g)) v.add(m[1]);
  const base = new Set(['base', 'stats', 'utils', 'graphics', 'grDevices', 'methods', 'datasets', 'tools', 'grid', 'parallel', 'splines', 'stats4', 'tcltk', 'compiler', 'webr']);
  return [...v].filter((p) => !base.has(p));
}

async function escribirArchivos(fs: { writeFile: (ruta: string, datos: Uint8Array) => Promise<unknown> | unknown }, dir: string, archivos: Archivo[]) {
  for (const a of archivos) {
    const nombre = a.nombre.replace(/[/\\]/g, '_');
    await fs.writeFile(`${dir}/${nombre}`, a.datos);
  }
}

async function aPng(img: ImageBitmap): Promise<string> {
  const c = new OffscreenCanvas(img.width, img.height);
  c.getContext('2d')!.drawImage(img, 0, 0);
  const b = await c.convertToBlob({ type: 'image/png' });
  const buf = new Uint8Array(await b.arrayBuffer());
  let s = '';
  for (let i = 0; i < buf.length; i += 0x8000) s += String.fromCharCode(...buf.subarray(i, i + 0x8000));
  return `data:image/png;base64,${btoa(s)}`;
}

/** Ejecuta un programa de R con webR. Los archivos quedan en la carpeta de trabajo. */
export async function ejecutarR(codigo: string, archivos: Archivo[] = [], graficas = true): Promise<Ejecucion> {
  const w = await cargarR();
  const faltan = paquetesR(codigo).filter((p) => !instalados.has(p));
  if (faltan.length) {
    motoresWeb.detalle = t('Instalando paquetes de R: {0}…', faltan.join(', '));
    try {
      await w.installPackages(faltan, { quiet: true });
    } finally {
      motoresWeb.detalle = '';
    }
    faltan.forEach((p) => instalados.add(p));
  }
  await escribirArchivos(w.FS, '/home/web_user', archivos);
  const sh = await new w.Shelter();
  try {
    const salida: string[] = [];
    let error = '';
    let imagenes: string[] = [];
    try {
      const r = await sh.captureR(codigo, {
        withAutoprint: true,
        captureStreams: true,
        captureConditions: false,
        ...(graficas ? { captureGraphics: { width: 720, height: 450 } } : { captureGraphics: false })
      });
      for (const o of r.output as { type: string; data: unknown }[]) salida.push(String(o.data));
      imagenes = await Promise.all(((r.images ?? []) as ImageBitmap[]).map(aPng));
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
    return { salida: salida.join('\n'), error, imagenes, motor: `R ${versionR} (webR ${WEBR})` };
  } finally {
    await sh.purge();
  }
}

/** El núcleo (módulo de series de tiempo) pide ejecutar un programa de R: ver puentes.rs. */
export async function escucharPedidosR() {
  if (!inTauri) return;
  try {
    await escuchar();
  } catch {
    /* sin eventos de Tauri: el núcleo usa su R o el motor nativo */
  }
}

async function escuchar() {
  const { listen } = await import('@tauri-apps/api/event');
  const { invoke } = await import('@tauri-apps/api/core');
  await listen<{ id: number; programa: string }>('r-ejecutar', async (ev) => {
    const { id, programa } = ev.payload;
    try {
      const r = await ejecutarR(programa, [], false);
      await invoke('r_resultado', { id, ok: true, salida: r.salida + (r.error ? `\n${r.error}` : '') });
    } catch (e) {
      await invoke('r_resultado', { id, ok: false, salida: e instanceof Error ? e.message : String(e) });
    }
  });
}

// ---------------------------------------------------------------- Python (Pyodide)

let pyodide: Promise<any> | null = null;

function cargarPython(): Promise<any> {
  pyodide ??= (async () => {
    motoresWeb.python = 'cargando';
    motoresWeb.detalle = t('Descargando Python (la primera vez tarda un poco)…');
    try {
      const mod = await import(/* @vite-ignore */ `${BASE_PYODIDE}pyodide.mjs`);
      const py = await mod.loadPyodide({ indexURL: BASE_PYODIDE });
      await py.loadPackage('micropip');
      motoresWeb.python = 'listo';
      motoresWeb.detalle = '';
      return py;
    } catch (e) {
      pyodide = null;
      motoresWeb.python = 'error';
      motoresWeb.detalle = '';
      throw new Error(t('No se pudo cargar Python dentro de la app (¿hay conexión a internet?): {0}', e instanceof Error ? e.message : String(e)));
    }
  })();
  return pyodide;
}

const GRAFICAS_PY = `
import sys as _s
_imgs = []
if 'matplotlib.pyplot' in _s.modules:
    import io as _io, base64 as _b64
    import matplotlib.pyplot as _plt
    for _n in _plt.get_fignums():
        _b = _io.BytesIO()
        _plt.figure(_n).savefig(_b, format='png', dpi=100, bbox_inches='tight')
        _imgs.append('data:image/png;base64,' + _b64.b64encode(_b.getvalue()).decode())
    _plt.close('all')
_imgs
`;

/** Ejecuta un programa de Python con Pyodide. Los archivos quedan en la carpeta de trabajo. */
export async function ejecutarPython(codigo: string, archivos: Archivo[] = []): Promise<Ejecucion> {
  const py = await cargarPython();
  const salida: string[] = [];
  // los avisos de carga de paquetes («Loading numpy…») no son parte de la salida
  py.setStdout({ batched: () => {} });
  py.setStderr({ batched: () => {} });
  await escribirArchivos(py.FS, '/home/pyodide', archivos);
  let error = '';
  let imagenes: string[] = [];
  try {
    motoresWeb.detalle = t('Preparando los paquetes de Python…');
    await py.loadPackagesFromImports(codigo);
    // lectores de Excel que pandas carga por dentro
    if (/read_excel|openpyxl/.test(codigo)) await py.pyimport('micropip').install('openpyxl');
    motoresWeb.detalle = '';
    if (/matplotlib/.test(codigo)) await py.runPythonAsync("import matplotlib\nmatplotlib.use('Agg')");
    py.setStdout({ batched: (x: string) => salida.push(x) });
    py.setStderr({ batched: (x: string) => salida.push(x) });
    const r = await py.runPythonAsync(codigo);
    if (r !== undefined && r !== null) {
      salida.push(String(r));
      r?.destroy?.();
    }
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
  } finally {
    motoresWeb.detalle = '';
  }
  try {
    const imgs = await py.runPythonAsync(GRAFICAS_PY);
    imagenes = imgs?.toJs?.() ?? [];
    imgs?.destroy?.();
  } catch {
    /* sin gráficas */
  }
  return { salida: salida.join('\n'), error, imagenes, motor: `Python ${py.runPython('import sys; sys.version.split()[0]')} (Pyodide ${PYODIDE})` };
}

// para las pruebas de punta a punta (tools/test_ia_ui.mjs)
if (typeof window !== 'undefined') (window as unknown as Record<string, unknown>).__iaqsbMotores = { ejecutarR, ejecutarPython };
