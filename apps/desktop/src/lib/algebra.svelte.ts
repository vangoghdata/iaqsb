// Modelo algebraico del módulo LP-ILP (lenguaje con conjuntos e índices, al
// estilo de AMPL y LINGO). Guarda el texto, la última revisión del núcleo, el
// archivo y si hay cambios, para que nada se pierda al cambiar de pestaña.

import { sesion } from './sesion.svelte';
import { app } from './state.svelte';
import { call, inTauri } from './backend';
import { motor } from './motor.svelte';
import { t } from './i18n.svelte';
import { EJEMPLOS_ALGEBRA, PLANTILLA_ALGEBRA } from './algebra-ejemplos';
import type { GraphicData, LpModel, LpReport } from './types';
import type { TableBlock } from './gtypes';

/** Tamaño y forma del modelo armado (check_model y solve → algebra). */
export interface AlgStats {
  variables: number;
  integer: number;
  constraints: number;
  nonzeros: number;
  aux: number;
  sets: { name: string; size: number }[];
  families: { name: string; indices: string[]; count: number }[];
  reformulations: string[];
  quadratic: boolean;
  objectives: number;
}

export interface AlgError {
  line: number;
  col: number;
  message: string;
}

export type AlgCheck = { ok: true; stats: AlgStats; errors?: AlgError[] } | { ok: false; errors: AlgError[]; stats?: undefined };

/** Informe de LP-ILP de un modelo algebraico: trae además la solución por índices. */
export type AlgReport = LpReport & { tables?: TableBlock[]; algebra?: AlgStats };

/** A partir de cuántas variables la hoja se vuelve lenta (se pide confirmar). */
export const HOJA_GRANDE = 2000;

/** Confirmación pendiente (reemplazar el texto con cambios, pasar un modelo grande a la hoja…). */
export interface Pendiente {
  mensaje: string;
  boton: string;
  accion: () => void | Promise<void>;
}

function base(nombre: string): string {
  return (nombre || t('modelo')).replace(/[^\p{L}\p{N} _-]/gu, '').trim().slice(0, 80) || t('modelo');
}

/**
 * Agrupa las notas de reformulación repetidas por índice
 * («orden[1,2]: disyunción…», «orden[1,3]: disyunción…» → una sola línea con la cuenta).
 */
export function agruparNotas(notas: string[]): { texto: string; veces: number; familia: string | null }[] {
  const out: { texto: string; veces: number; familia: string | null }[] = [];
  const idx = new Map<string, number>();
  for (const n of notas) {
    const m = /^([\p{L}_][\p{L}\p{N}_']*)\[[^\]]*\]:\s*(.*)$/su.exec(n);
    const clave = m ? `${m[1]}\u0000${m[2]}` : n;
    const k = idx.get(clave);
    if (k !== undefined) out[k].veces++;
    else {
      idx.set(clave, out.length);
      out.push(m ? { texto: m[2], veces: 1, familia: m[1] } : { texto: n, veces: 1, familia: null });
    }
  }
  return out;
}

class AlgebraState {
  /** Modo del módulo LP-ILP: hoja (false) o modelo algebraico (true). */
  on = $state(false);
  text = $state('');
  name = $state('');
  file = $state<string | null>(null);
  dirty = $state(false);
  check = $state<AlgCheck | null>(null);
  /** Tamaño de la última revisión sin errores (sigue a la vista mientras se corrige un error). */
  stats = $state<AlgStats | null>(null);
  checking = $state(false);
  /** Ejemplo cargado (si no se ha editado). */
  example = $state<string | null>(null);
  /** Panel de ayuda del lenguaje abierto. */
  ayuda = $state(false);
  /** Pedido de mover el cursor (lo atiende el editor; n cambia en cada pedido). */
  salto = $state<{ line: number; col: number; n: number } | null>(null);
  pendiente = $state<Pendiente | null>(null);
  /** Formato de «Guardar»: el texto (.iaqsb) o el modelo expandido en MPS, LP o WinQSB. */
  formato = $state<'iaqsb' | 'mps' | 'lp' | 'lpp'>('iaqsb');
  extension = $derived(this.formato === 'iaqsb' ? '.iaqsb' : this.formato === 'lpp' ? '.LPP' : '.' + this.formato.toUpperCase());
  /** Desplazamiento y cursor del editor (se conservan al cambiar de pestaña; no es reactivo). */
  memoria = { top: 0, left: 0, a: 0, b: 0 };
  ocupado = $state(false);

  private timer: ReturnType<typeof setTimeout> | null = null;
  private seq = 0;

  /** El informe que se ve salió de un modelo algebraico. */
  get propio(): boolean {
    return !!(app.report as AlgReport | null)?.algebra;
  }

  /** Cambia entre la hoja y el modelo algebraico. */
  activar(on: boolean) {
    if (on === this.on) return;
    this.on = on;
    this.pendiente = null;
    if (on && !this.text.trim()) this.cargarEjemplo(EJEMPLOS_ALGEBRA[0].id);
    else if (on && !this.check) this.revisar();
    // la vista «Modelo» muestra el editor o la hoja; las demás siguen con el último informe
    if (app.tab === 'practica') app.tab = 'modelo';
  }

  /** El texto cambió en el editor. */
  editar(s: string) {
    if (s === this.text) return;
    this.text = s;
    this.dirty = true;
    this.example = null;
    if (this.propio) app.stale = true;
    this.programar();
    sesion.programar();
  }

  /** Revisión en vivo: espera 300 ms sin teclear. */
  programar(ms = 300) {
    if (this.timer) clearTimeout(this.timer);
    this.timer = setTimeout(() => {
      this.timer = null;
      this.revisar();
    }, ms);
  }

  async revisar() {
    const n = ++this.seq;
    const texto = this.text;
    this.checking = true;
    try {
      const r = await call<AlgCheck>('check_model', { text: texto });
      if (n === this.seq) {
        this.check = r;
        if (r.ok) this.stats = r.stats;
      }
    } catch (e) {
      if (n === this.seq) this.check = { ok: false, errors: [{ line: 0, col: 0, message: e instanceof Error ? e.message : String(e) }] };
    } finally {
      if (n === this.seq) this.checking = false;
    }
  }

  /** Mueve el cursor del editor a una posición (p. ej. la del error). */
  ir(line: number, col: number) {
    this.salto = { line, col, n: (this.salto?.n ?? 0) + 1 };
  }

  /** Pide confirmar antes de reemplazar un texto con cambios sin guardar. */
  private confirmarReemplazo(accion: () => void | Promise<void>) {
    if (this.dirty && this.text.trim()) {
      this.pendiente = { mensaje: t('El texto actual tiene cambios sin guardar y se va a reemplazar.'), boton: t('Reemplazar'), accion };
    } else {
      this.pendiente = null;
      accion();
    }
  }

  private limpiarInforme() {
    app.report = null;
    app.graph = null;
    app.stale = false;
    app.error = null;
    app.tab = 'modelo';
  }

  /** Pone un texto nuevo en el editor (archivo, ejemplo, plantilla o la hoja). */
  cargar(texto: string, nombre: string, file: string | null, dirty = false) {
    this.text = texto;
    this.name = nombre;
    this.file = file;
    this.dirty = dirty;
    this.example = null;
    this.check = null;
    this.stats = null;
    this.pendiente = null;
    this.on = true;
    this.limpiarInforme();
    this.programar(0);
    this.ir(1, 1);
  }

  cargarEjemplo(id: string) {
    const ex = EJEMPLOS_ALGEBRA.find((e) => e.id === id);
    if (!ex) return;
    this.confirmarReemplazo(() => {
      this.cargar(ex.texto, t(ex.titulo), null);
      this.example = id;
    });
  }

  nuevo() {
    this.confirmarReemplazo(() => this.cargar(PLANTILLA_ALGEBRA, t('Modelo nuevo'), null));
  }

  /** El modelo tal como lo recibe el núcleo. */
  payload() {
    return { format: 'iaqsb', text: this.text, name: this.name };
  }

  async solve() {
    if (!this.text.trim() || app.busy) return;
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
      this.revisar();
    }
    app.busy = true;
    app.error = null;
    try {
      const options = { integer_tolerance: app.intTolerance, ...motor.params() };
      const rep = await call<AlgReport>('solve', { module: 'lp', model: this.payload(), options });
      app.report = rep;
      app.stale = false;
      app.graph = null;
      // método gráfico si quedan dos variables continuas (p. ej. el modo rápido)
      const s = rep.algebra;
      if (s && s.variables === 2 && s.integer === 0 && !s.quadratic) {
        try {
          const { model } = await call<{ model: LpModel }>('convert', { model: this.payload(), to: 'json' });
          app.graph = await call<GraphicData>('graphic', { model });
        } catch {
          app.graph = null;
        }
      }
      app.tab = 'solucion';
    } catch (e) {
      // un error del texto se muestra en su línea (no hace falta repetirlo arriba)
      await this.revisar();
      if (this.check?.ok !== false) app.error = e instanceof Error ? e.message : String(e);
      else {
        app.tab = 'modelo';
        const er = this.check.errors[0];
        if (er && er.line > 0) this.ir(er.line, er.col);
      }
    } finally {
      app.busy = false;
    }
  }

  /** Expande el modelo en la hoja de LP-ILP (una columna por variable indexada). */
  async aHoja(forzar = false) {
    const s = this.check?.ok ? this.check.stats : null;
    if (!forzar && s && s.variables > HOJA_GRANDE) {
      this.pendiente = {
        mensaje: t('El modelo tiene {0} variables y {1} restricciones: la hoja será lenta de editar. Para resolverlo no hace falta pasarlo a la hoja.', s.variables.toLocaleString(), s.constraints.toLocaleString()),
        boton: t('Pasar de todos modos'),
        accion: () => this.aHoja(true)
      };
      return;
    }
    this.pendiente = null;
    this.ocupado = true;
    try {
      const { model } = await call<{ model: LpModel }>('convert', { model: this.payload(), to: 'json' });
      model.name = this.name || model.name;
      app.load(model);
      app.dirty = true;
      this.on = false;
      app.flash(t('Modelo pasado a la hoja: {0} variables, {1} restricciones', model.variables.length, model.constraints.length));
    } catch (e) {
      app.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.ocupado = false;
    }
  }

  /** Escribe la hoja actual en el lenguaje (sin índices: una variable por columna). */
  async deHoja() {
    this.confirmarReemplazo(async () => {
      this.ocupado = true;
      try {
        const model = app.large ? { name: app.model.name, format: app.large.format, text: app.large.text } : $state.snapshot(app.model);
        const { content } = await call<{ content: string }>('convert', { model, to: 'iaqsb' });
        this.cargar(content, app.model.name, null, true);
        app.flash(t('La hoja se escribió como modelo algebraico'));
      } catch (e) {
        app.error = e instanceof Error ? e.message : String(e);
      } finally {
        this.ocupado = false;
      }
    });
  }

  /** Abre un modelo algebraico (.iaqsb, .txt). */
  async abrir() {
    try {
      if (inTauri) {
        const { open } = await import('@tauri-apps/plugin-dialog');
        const path = await open({
          multiple: false,
          filters: [
            { name: t('Modelos algebraicos (.iaqsb)'), extensions: ['iaqsb', 'IAQSB', 'ml'] },
            { name: t('Todos los archivos'), extensions: ['*'] }
          ]
        });
        if (!path || Array.isArray(path)) return;
        const { invoke } = await import('@tauri-apps/api/core');
        let r;
        try {
          r = await invoke<import('./backend').Imported>('read_file', { path });
        } catch (e) {
          throw new Error(`${e}. ${t('Si es un modelo algebraico, guárdalo con la extensión .iaqsb.')}`);
        }
        const { shell } = await import('./shell.svelte');
        shell.openImported({ ...r, path });
        return;
      }
      // navegador: se lee el texto tal cual
      const f = await new Promise<File | null>((resolve) => {
        const input = document.createElement('input');
        input.type = 'file';
        input.accept = '.iaqsb,.ml,.txt';
        input.onchange = () => resolve(input.files?.[0] ?? null);
        input.click();
      });
      if (!f) return;
      const texto = await f.text();
      this.cargar(texto, f.name.replace(/\.[^.]*$/, ''), f.name);
      app.flash(t('Abierto {0}', f.name));
    } catch (e) {
      app.error = e instanceof Error ? e.message : String(e);
    }
  }

  /** Guarda el texto como .iaqsb (o exporta el modelo expandido en el formato elegido). */
  async guardar() {
    if (this.formato !== 'iaqsb') return this.exportar(this.formato);
    const nombre = `${base(this.name)}.iaqsb`;
    try {
      if (inTauri) {
        const { save } = await import('@tauri-apps/plugin-dialog');
        const propio = this.file && /\.(iaqsb|mod|ml|txt)$/i.test(this.file);
        const path = await save({
          defaultPath: propio ? this.file! : nombre,
          filters: [{ name: t('Modelo algebraico de IAQSB'), extensions: ['iaqsb', 'txt'] }]
        });
        if (!path) return;
        const { invoke } = await import('@tauri-apps/api/core');
        // el núcleo devuelve el texto tal cual ({content}) y save_export lo escribe
        await invoke('save_export', { path, op: 'echo_text', args: { content: this.text } });
        this.file = path;
      } else {
        const blob = new Blob([this.text], { type: 'text/plain;charset=utf-8' });
        const a = document.createElement('a');
        a.href = URL.createObjectURL(blob);
        a.download = nombre;
        a.click();
        URL.revokeObjectURL(a.href);
        this.file = nombre;
      }
      this.dirty = false;
      app.flash(t('Guardado {0}', (this.file ?? nombre).split(/[\\/]/).pop()));
    } catch (e) {
      app.error = e instanceof Error ? e.message : String(e);
    }
  }

  /** Exporta el modelo armado (sin índices) en MPS, LP o el formato de WinQSB; el .iaqsb no cambia. */
  async exportar(to: 'mps' | 'lp' | 'lpp') {
    const ext = to === 'lpp' ? 'LPP' : to;
    const nombre = `${base(this.name)}.${ext}`;
    const filtro =
      to === 'mps'
        ? { name: t('Modelo MPS (cualquier motor)'), extensions: ['mps'] }
        : to === 'lp'
          ? { name: t('Modelo LP (formato de CPLEX)'), extensions: ['lp'] }
          : { name: t('Problema de LP-ILP (WinQSB)'), extensions: ['LPP'] };
    try {
      if (inTauri) {
        const { save } = await import('@tauri-apps/plugin-dialog');
        const path = await save({ defaultPath: nombre, filters: [filtro] });
        if (!path) return;
        const { invoke } = await import('@tauri-apps/api/core');
        let hecho = false;
        if (to === 'lpp') {
          // WinQSB lee ANSI: se escribe con write_file desde la hoja expandida (si cabe en la hoja)
          try {
            const { model } = await call<{ model: LpModel }>('convert', { model: this.payload(), to: 'json' });
            await invoke('write_file', { path, module: 'lp', model, layout: 'normal' });
            hecho = true;
          } catch {
            hecho = false;
          }
        }
        if (!hecho) {
          await invoke('save_export', { path, op: 'convert', args: { model: this.payload(), to } });
        }
        app.flash(t('Guardado {0}', path.split(/[\\/]/).pop()));
        return;
      }
      const { content } = await call<{ content: string }>('convert', { model: this.payload(), to });
      // WinQSB lee ANSI (latin-1); MPS y LP van en UTF-8
      const bytes = to === 'lpp' ? Uint8Array.from(content, (c) => (c.charCodeAt(0) < 256 ? c.charCodeAt(0) : 63)) : new TextEncoder().encode(content);
      const a = document.createElement('a');
      a.href = URL.createObjectURL(new Blob([bytes as BlobPart], { type: 'application/octet-stream' }));
      a.download = nombre;
      a.click();
      URL.revokeObjectURL(a.href);
      app.flash(t('Guardado {0}', nombre));
    } catch (e) {
      app.error = e instanceof Error ? e.message : String(e);
    }
  }
}

export const algebra = new AlgebraState();
