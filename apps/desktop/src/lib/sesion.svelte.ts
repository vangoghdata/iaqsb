// Autoguardado: el trabajo abierto (la hoja de LP-ILP, el modelo algebraico y los
// datos de cada módulo) se guarda en este equipo unos segundos después de cada
// cambio y se recupera al volver a abrir IAQSB, aunque no se haya guardado.

import { ajustes } from './ajustes.svelte';
import { inTauri, type Imported } from './backend';

const CLAVE = 'iaqsb.sesion';
/** Más que esto no se guarda (el almacenamiento del navegador es limitado). */
const MAX = 3_000_000;

interface Sesion {
  v: 1;
  ts: number;
  module: string;
  lp?: { model: unknown; layout: string; file: string | null; dirty: boolean };
  algebra?: { on: boolean; text: string; name: string; file: string | null; dirty: boolean };
  mods: Record<string, { model: unknown; options: unknown; file: string | null; dirty: boolean }>;
}

class SesionStore {
  private t: ReturnType<typeof setTimeout> | undefined;
  /** Hora del último autoguardado (para mostrarla en Ajustes). */
  ultimo = $state<number | null>(null);
  /** Mientras se recupera la sesión no se vuelve a guardar. */
  private recuperando = false;

  /** Guarda dentro de un momento (agrupa los cambios seguidos). */
  programar(ms = 1500) {
    if (this.recuperando || typeof window === 'undefined') return;
    clearTimeout(this.t);
    this.t = setTimeout(() => void this.guardar(), ms);
  }

  async guardar() {
    if (!ajustes.autoguardado) return;
    const { app } = await import('./state.svelte');
    const { shell } = await import('./shell.svelte');
    const { algebra } = await import('./algebra.svelte');
    const mods: Sesion['mods'] = {};
    for (const m of shell.modules) {
      const st = shell.peek(m.id);
      if (st?.model) mods[m.id] = { model: $state.snapshot(st.model), options: $state.snapshot(st.options), file: st.file, dirty: st.dirty };
    }
    const s: Sesion = {
      v: 1,
      ts: Date.now(),
      module: shell.module,
      lp: app.large ? undefined : { model: $state.snapshot(app.model), layout: app.layout, file: app.file, dirty: app.dirty },
      algebra: algebra.text ? { on: algebra.on, text: algebra.text, name: algebra.name, file: algebra.file, dirty: algebra.dirty } : undefined,
      mods
    };
    let txt = JSON.stringify(s);
    if (txt.length > MAX) {
      // sin los módulos que no se están usando
      s.mods = Object.fromEntries(Object.entries(mods).filter(([k]) => k === shell.module));
      txt = JSON.stringify(s);
      if (txt.length > MAX) return this.fallo();
    }
    try {
      localStorage.setItem(CLAVE, txt);
      this.ultimo = s.ts;
      this.avisado = false;
    } catch {
      this.fallo();
    }
  }

  private avisado = false;

  /** No se pudo autoguardar: se borra lo anterior (para no recuperar después algo viejo) y se avisa una vez. */
  private async fallo() {
    try {
      localStorage.removeItem(CLAVE);
    } catch {
      /* nada */
    }
    this.ultimo = null;
    if (this.avisado) return;
    this.avisado = true;
    const { ui } = await import('./ui.svelte');
    const { t } = await import('./i18n.svelte');
    ui.avisar(t('El trabajo es demasiado grande para el autoguardado: guárdalo con Ctrl+S.'), 'error');
  }

  private leer(): Sesion | null {
    try {
      const v = JSON.parse(localStorage.getItem(CLAVE) ?? 'null');
      return v && v.v === 1 ? v : null;
    } catch {
      return null;
    }
  }

  /** Recupera el trabajo de la sesión anterior. Devuelve si había algo. */
  async recuperar(): Promise<boolean> {
    if (!ajustes.autoguardado) return false;
    const s = this.leer();
    if (!s) return false;
    const { app } = await import('./state.svelte');
    const { shell } = await import('./shell.svelte');
    const { algebra } = await import('./algebra.svelte');
    this.recuperando = true;
    let algo = false;
    try {
      // un archivo sin cambios se vuelve a leer del disco (pudo cambiar fuera de IAQSB)
      const releer = async (path: string, module: string): Promise<boolean> => {
        if (!inTauri) return false;
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          if (!(await invoke<boolean>('file_exists', { path }))) return false;
          const res = await invoke<Imported>('read_file', { path, module, problemType: null });
          shell.openImported({ ...res, path });
          return true;
        } catch {
          return false;
        }
      };
      for (const [id, m] of Object.entries(s.mods ?? {})) {
        if (!shell.modules.some((x) => x.id === id && x.available)) continue;
        if (m.file && !m.dirty) {
          await releer(m.file, id);
          continue;
        }
        const st = shell.get(id);
        if (m.options && typeof m.options === 'object') st.options = m.options as Record<string, unknown>;
        st.load(m.model, m.file);
        st.dirty = m.dirty;
        algo = algo || m.dirty;
      }
      if (s.lp?.file && !s.lp.dirty && !s.algebra?.on) {
        await releer(s.lp.file, 'lp');
      } else if (s.lp?.model) {
        app.load(s.lp.model as never, s.lp.file, (s.lp.layout as never) ?? 'matrix');
        app.dirty = s.lp.dirty;
        algo = algo || s.lp.dirty;
      }
      if (s.algebra?.text) {
        algebra.cargar(s.algebra.text, s.algebra.name, s.algebra.file, s.algebra.dirty);
        if (!s.algebra.on) algebra.on = false;
        algo = algo || s.algebra.dirty;
      }
      if (s.module && (s.module === 'lp' || shell.modules.some((x) => x.id === s.module && x.available))) shell.module = s.module;
    } catch {
      /* sesión de otra versión: se ignora */
    } finally {
      this.recuperando = false;
    }
    return algo;
  }

  borrar() {
    try {
      localStorage.removeItem(CLAVE);
    } catch {
      /* nada */
    }
    this.ultimo = null;
  }
}

export const sesion = new SesionStore();
