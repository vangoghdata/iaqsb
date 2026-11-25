// Estado general: módulo activo y estado de cada módulo genérico.

import { ui } from './ui.svelte';
import { call, openFile, saveModel, type Imported } from './backend';
import { app } from './state.svelte';
import { algebra } from './algebra.svelte';
import { i18n, t, type Lang } from './i18n.svelte';
import { motor } from './motor.svelte';
import { Historial } from './historial.svelte';
import { recientes } from './recientes.svelte';
import { sesion } from './sesion.svelte';
import type { DataSource, Field, ModuleSpec, Report } from './gtypes';
import { basesDeTemas } from './grafo';
import type { ModuleInfo } from './types';

export function defaults(fields: Field[]): Record<string, unknown> {
  const o: Record<string, unknown> = {};
  for (const f of fields) o[f.key] = f.default ?? null;
  return o;
}

export class GenState {
  id: string;
  spec = $state<ModuleSpec | null>(null);
  model = $state<any>(null);
  report = $state<Report | null>(null);
  options = $state<Record<string, unknown>>({});
  file = $state<string | null>(null);
  dirty = $state(false);
  stale = $state(false);
  busy = $state(false);
  error = $state<string | null>(null);
  notice = $state<string | null>(null);
  tab = $state('datos');
  newOpen = $state(false);
  examples = $state<{ name: string; model: any; source?: DataSource | null }[]>([]);
  ready = $state(false);
  /** Ejemplo cargado (para traerlo en el otro idioma si no se ha editado). */
  example = $state<string | null>(null);

  ptype = $derived(
    this.spec?.problem_types.find((p) => p.id === this.model?.problem_type) ??
      (this.spec?.problem_types.length === 1 ? this.spec.problem_types[0] : null)
  );

  /** Deshacer y rehacer de los datos del módulo. */
  historial = new Historial();
  private foto = () => JSON.stringify({ model: $state.snapshot(this.model), options: $state.snapshot(this.options) });

  constructor(id: string) {
    this.id = id;
  }

  deshacer() {
    const s = this.historial.deshacer();
    if (s) this.restaurar(s);
  }

  rehacer() {
    const s = this.historial.rehacer();
    if (s) this.restaurar(s);
  }

  private restaurar(s: string) {
    try {
      const v = JSON.parse(s);
      this.model = v.model;
      if (v.options && Object.keys(v.options).length) this.options = v.options;
      this.dirty = true;
      if (this.report) this.stale = true;
      sesion.programar();
    } catch {
      /* foto dañada */
    }
  }

  async init() {
    if (this.ready) return;
    try {
      this.spec = await call<ModuleSpec>('spec', { module: this.id });
      // las opciones recuperadas de la sesión anterior se conservan
      this.options = { ...defaults(this.spec.options), ...this.options };
      this.examples = await call<{ name: string; model: any; source?: DataSource | null }[]>('examples', { module: this.id });
      if (!this.model) {
        if (this.examples.length) this.loadExample(this.examples[0].name);
        else await this.newProblem(this.spec.problem_types[0].id, {});
      } else {
        // el punto de partida de «deshacer» ya con las opciones completas
        this.historial.reiniciar(this.foto());
      }
      this.ready = true;
    } catch (e) {
      this.error = String(e instanceof Error ? e.message : e);
    }
  }

  changed() {
    this.dirty = true;
    if (this.report) this.stale = true;
    this.historial.registrar(this.foto);
    sesion.programar();
  }

  flash(msg: string) {
    ui.avisar(msg);
    this.notice = msg;
    setTimeout(() => {
      if (this.notice === msg) this.notice = null;
    }, 3500);
  }

  loadExample(name: string) {
    const ex = this.examples.find((e) => e.name === name);
    if (!ex) return;
    this.load($state.snapshot(ex.model), null);
    this.example = name;
  }

  /** Tras cambiar el idioma: trae de nuevo la especificación, los ejemplos y el informe. */
  async refreshLanguage() {
    if (!this.ready) return;
    const lang = i18n.lang;
    const foto = this.foto();
    // si mientras tanto se cambió otra vez el idioma o se editó el modelo, esto ya no vale
    const vigente = () => i18n.lang === lang && this.foto() === foto;
    try {
      const spec = await call<ModuleSpec>('spec', { module: this.id });
      const examples = await call<{ name: string; model: any; source?: DataSource | null }[]>('examples', { module: this.id });
      if (!vigente()) return;
      this.spec = spec;
      this.examples = examples;
      const had = !!this.report && !this.stale;
      const tab = this.tab;
      if (this.example && !this.dirty && !this.file) this.loadExample(this.example);
      if (had) {
        await this.solve();
        if (this.report?.sections.some((x) => x.id === tab)) this.tab = tab;
      }
    } catch (e) {
      this.error = String(e instanceof Error ? e.message : e);
    }
  }

  load(model: any, file: string | null) {
    this.example = null;
    this.model = model;
    this.report = null;
    this.stale = false;
    this.dirty = false;
    this.error = null;
    this.file = file;
    this.tab = 'datos';
    this.historial.reiniciar(this.foto());
    recientes.anotar(file, this.id, model?.name ?? '');
    sesion.programar();
  }

  async newProblem(problemType: string, dims: Record<string, unknown>) {
    const m = await call<any>('template', { module: this.id, problem_type: problemType, dims });
    this.load(m, null);
  }

  /** Opciones del módulo más los parámetros del motor (si se cambiaron). */
  private solveOptions() {
    const o: Record<string, unknown> = { ...$state.snapshot(this.options) };
    const p = motor.params();
    if (Object.keys(p).length) o.engine = p;
    return o;
  }

  /** Número de la última resolución pedida (una respuesta vieja no pisa a una nueva). */
  private pedido = 0;

  async solve() {
    const n = ++this.pedido;
    this.busy = true;
    this.error = null;
    try {
      const enviado = this.foto();
      const rep = await call<Report>('solve', {
        module: this.id,
        model: $state.snapshot(this.model),
        options: this.solveOptions()
      });
      if (n !== this.pedido) return;
      this.report = rep;
      this.stale = this.foto() !== enviado;
      this.tab = this.report.sections[0]?.id ?? 'datos';
    } catch (e) {
      if (n === this.pedido) this.error = e instanceof Error ? e.message : String(e);
    } finally {
      if (n === this.pedido) this.busy = false;
    }
  }

  /** Resuelve con cada método de la opción comparable y muestra la comparación. */
  async compare() {
    this.busy = true;
    this.error = null;
    try {
      this.report = await call<Report>('compare', {
        module: this.id,
        model: $state.snapshot(this.model),
        options: this.solveOptions()
      });
      this.stale = false;
      this.tab = this.report.sections[0]?.id ?? 'datos';
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.busy = false;
    }
  }

  async save() {
    try {
      const path = await saveModel(this.id, $state.snapshot(this.model), this.spec?.file_ext || 'DAT', this.file ?? undefined);
      if (!path) return;
      this.file = path;
      this.dirty = false;
      recientes.anotar(path, this.id, this.model?.name ?? '');
      sesion.programar();
      this.flash(t('Guardado {0}', path.split(/[\\/]/).pop()));
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    }
  }
}

class Shell {
  module = $state('lp');
  modules = $state<ModuleInfo[]>([]);
  private states = new Map<string, GenState>();

  async init() {
    try {
      const r = await call<{ modules: ModuleInfo[] }>('list_modules');
      this.modules = r.modules;
      basesDeTemas.clear();
      for (const m of r.modules) if (m.base) basesDeTemas.set(m.id, m.base);
    } catch (e) {
      app.error = t('No se pudo conectar con el núcleo de cálculo: {0}', e);
    }
  }

  /** Cambia el idioma de toda la app y trae de nuevo lo que viene del núcleo. */
  /** Se están trayendo del núcleo los textos en el idioma nuevo. */
  cambiando = $state(false);
  private idiomaN = 0;

  /**
   * Cambia el idioma de toda la app: la interfaz cambia al instante y lo que
   * viene del núcleo (módulos, ejemplos, informes) se trae en segundo plano,
   * sin bloquear la ventana.
   */
  async cambiarIdioma(l: Lang) {
    if (l === i18n.lang) return;
    i18n.set(l);
    const n = ++this.idiomaN;
    this.cambiando = true;
    try {
      await Promise.all([this.init(), app.refreshLanguage(), ...[...this.states.values()].map((s) => s.refreshLanguage())]);
    } finally {
      // si se cambió otra vez mientras tanto, el indicador lo apaga el último cambio
      if (n === this.idiomaN) this.cambiando = false;
    }
  }

  /** El estado de un módulo, si ya existe (sin crearlo). */
  peek(id: string): GenState | undefined {
    return this.states.get(id);
  }

  get(id: string): GenState {
    let s = this.states.get(id);
    if (!s) {
      s = new GenState(id);
      this.states.set(id, s);
    }
    return s;
  }

  extensions(): string[] {
    return this.modules.map((m) => m.file_ext).filter(Boolean);
  }

  openImported(r: Imported) {
    if (r.module === 'lp') {
      this.module = 'lp';
      const ext = r.path?.split('.').pop()?.toLowerCase();
      const base = r.path?.split(/[\\/]/).pop()?.replace(/\.[^.]*$/, '');
      if (r.algebra) {
        // modelo en el lenguaje algebraico (.iaqsb): va al editor de texto
        algebra.cargar(r.model.text ?? '', r.model.name || base || t('Modelo algebraico'), r.path ?? null);
      } else if (r.large) {
        algebra.on = false;
        app.loadLarge({ ...r.model, size: r.size }, r.path ?? null);
      } else {
        if (!r.model.name && base) r.model.name = base;
        algebra.on = false;
        app.load(r.model, r.path ?? null, ext === 'mps' || ext === 'lp' || ext === 'xlsx' ? ext : r.layout);
      }
      app.flash(t('Abierto {0}', r.path?.split(/[\\/]/).pop() ?? t('el archivo')));
      return;
    }
    const st = this.get(r.module);
    st.load(r.model, r.path ?? null);
    st.init().then(() => st.flash(t('Abierto {0}', r.path?.split(/[\\/]/).pop() ?? t('el archivo'))));
    this.module = r.module;
  }

  /** Abre cualquier archivo de WinQSB o de IAQSB y cambia al módulo que corresponda. */
  async open(onError: (msg: string) => void) {
    try {
      // un libro de Excel que no hizo IAQSB se lee como problema del módulo abierto
      const ctx = { module: this.module, problemType: this.module === 'lp' ? null : (this.states.get(this.module)?.model?.problem_type ?? null) };
      const r = await openFile(this.extensions(), ctx);
      if (!r) return;
      if (!this.modules.some((m) => m.id === r.module && m.available)) {
        onError(t('Ese archivo es de un módulo que IAQSB todavía no incluye.'));
        return;
      }
      this.openImported(r);
    } catch (e) {
      onError(e instanceof Error ? e.message : String(e));
    }
  }
}

export const shell = new Shell();
