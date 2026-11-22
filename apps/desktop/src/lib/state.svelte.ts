// Estado de la aplicación (runas de Svelte 5).

import { ui } from './ui.svelte';
import { ajustes } from './ajustes.svelte';
import { call, saveLpp } from './backend';
import { i18n, t } from './i18n.svelte';
import { motor } from './motor.svelte';
import { algebra } from './algebra.svelte';
import { Historial } from './historial.svelte';
import { recientes } from './recientes.svelte';
import { sesion } from './sesion.svelte';
import type { GraphicData, LpModel, LpReport, VarType } from './types';

export type Tab = 'modelo' | 'solucion' | 'pasos' | 'nodos' | 'grafico' | 'practica' | 'quepasa';
export type LpLayout = 'matrix' | 'normal' | 'mps' | 'lp' | 'xlsx';

/** Modelo demasiado grande para la hoja: se guarda como texto MPS/LP y lo resuelve el motor. */
export interface LargeModel {
  name: string;
  format: 'mps' | 'lp';
  text: string;
  size: { cols: number; rows: number; nonzeros: number; integer: number };
}

function blank(nv = 2, nc = 2): LpModel {
  return {
    name: t('Problema nuevo'),
    objective: 'max',
    variables: Array.from({ length: nv }, (_, j) => ({ name: `X${j + 1}`, cost: 0, lower: 0, upper: 'M', type: 'continuous' as VarType })),
    constraints: Array.from({ length: nc }, (_, i) => ({ name: `C${i + 1}`, coefs: Array(nv).fill(0), direction: '<=' as const, rhs: 0 }))
  };
}

class AppState {
  model = $state<LpModel>(blank());
  layout = $state<LpLayout>('matrix');
  large = $state<LargeModel | null>(null);
  file = $state<string | null>(null);
  dirty = $state(false);
  report = $state<LpReport | null>(null);
  graph = $state<GraphicData | null>(null);
  stale = $state(false);
  busy = $state(false);
  error = $state<string | null>(null);
  notice = $state<string | null>(null);
  tab = $state<Tab>('modelo');
  showSteps = $state(ajustes.pasos);
  intTolerance = $state(0.01);
  connectOpen = $state(false);

  /** Deshacer y rehacer de la hoja. */
  historial = new Historial();

  private foto = () => JSON.stringify($state.snapshot(this.model));

  constructor() {
    this.historial.reiniciar(this.foto());
  }

  /** Vuelve al estado anterior (Ctrl+Z). */
  deshacer() {
    const s = this.historial.deshacer();
    if (s) this.restaurar(s);
  }

  /** Rehace lo deshecho (Ctrl+Y o Ctrl+Mayús+Z). */
  rehacer() {
    const s = this.historial.rehacer();
    if (s) this.restaurar(s);
  }

  private restaurar(s: string) {
    try {
      this.model = JSON.parse(s);
      this.dirty = true;
      if (this.report) this.stale = true;
      sesion.programar();
    } catch {
      /* foto dañada: se ignora */
    }
  }

  isInteger = $derived(this.large ? this.large.size.integer > 0 : this.model.variables.some((v) => v.type === 'integer' || v.type === 'binary' || v.type === 'semiinteger'));
  canGraph = $derived(!this.large && this.model.variables.length === 2);

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

  newProblem(nv = 2, nc = 2) {
    this.model = blank(nv, nc);
    this.large = null;
    this.reset();
    this.file = null;
    this.example = null;
    this.historial.reiniciar(this.foto());
    sesion.programar();
  }

  private reset() {
    this.report = null;
    this.graph = null;
    this.stale = false;
    this.dirty = false;
    this.error = null;
    this.tab = 'modelo';
  }

  load(model: LpModel, file: string | null = null, layout: LpLayout = 'matrix') {
    this.large = null;
    algebra.on = false; // un modelo de hoja se ve en la hoja
    // completa los campos opcionales para que el editor los muestre
    model.variables = model.variables.map((v) => ({
      ...v,
      lower: v.type === 'unrestricted' ? '-M' : (v.lower ?? 0),
      upper: v.type === 'unrestricted' ? 'M' : v.type === 'binary' ? (v.upper ?? 1) : (v.upper ?? 'M'),
      type: v.type ?? 'continuous'
    }));
    this.model = model;
    this.layout = layout;
    this.reset();
    this.file = file;
    this.example = null;
    this.historial.reiniciar(this.foto());
    recientes.anotar(file, 'lp', model.name);
    sesion.programar();
  }

  /** Abre un modelo MPS/LP demasiado grande para la hoja. */
  loadLarge(m: LargeModel, file: string | null) {
    this.large = m;
    algebra.on = false;
    this.model = { ...blank(0, 0), name: m.name || file?.split(/[\\/]/).pop()?.replace(/\.[^.]*$/, '') || t('Modelo grande') };
    this.layout = m.format;
    this.reset();
    this.file = file;
    this.example = null;
    this.historial.reiniciar(this.foto());
    recientes.anotar(file, 'lp', m.name);
  }

  /** El modelo tal como lo recibe el núcleo. */
  payload(): unknown {
    if (this.large) return { name: this.model.name, format: this.large.format, text: this.large.text };
    return $state.snapshot(this.model);
  }

  /** Ejemplo cargado (para traerlo de nuevo en el otro idioma si no se ha editado). */
  example = $state<string | null>(null);

  async loadExample(name: string) {
    const lang = i18n.lang;
    const all = await call<{ name: string; model: LpModel }[]>('examples');
    // si mientras llegaba se cambió de idioma, lo carga la llamada del idioma nuevo
    if (i18n.lang !== lang) return;
    const ex = all.find((e) => e.name === name);
    if (ex) {
      this.load(structuredClone(ex.model));
      this.example = name;
    }
  }

  /** Tras cambiar el idioma: trae el ejemplo traducido y vuelve a resolver. */
  async refreshLanguage() {
    const had = !!this.report && !this.stale;
    // modo algebraico: se revisa el texto otra vez (errores en el nuevo idioma) y se resuelve de nuevo
    if (algebra.on) {
      algebra.revisar();
      if (had && algebra.propio) {
        const tab = this.tab;
        await algebra.solve();
        this.tab = tab;
      }
      return;
    }
    if (this.example && !this.dirty && !this.file) {
      const tab = this.tab;
      await this.loadExample(this.example);
      if (had) {
        await this.solve();
        this.tab = tab;
      }
    } else if (had) {
      const tab = this.tab;
      await this.solve();
      this.tab = tab;
    }
  }

  addVariable() {
    const n = this.model.variables.length + 1;
    let name = `X${n}`;
    while (this.model.variables.some((v) => v.name.toUpperCase() === name)) name += "'";
    this.model.variables.push({ name, cost: 0, lower: 0, upper: 'M', type: 'continuous' });
    for (const c of this.model.constraints) c.coefs.push(0);
    this.changed();
  }

  removeVariable(j: number) {
    if (this.model.variables.length <= 1) return;
    this.model.variables.splice(j, 1);
    for (const c of this.model.constraints) c.coefs.splice(j, 1);
    this.changed();
  }

  addConstraint() {
    const n = this.model.constraints.length + 1;
    this.model.constraints.push({ name: `C${n}`, coefs: Array(this.model.variables.length).fill(0), direction: '<=', rhs: 0 });
    this.changed();
  }

  removeConstraint(i: number) {
    this.model.constraints.splice(i, 1);
    this.changed();
  }

  /** Número de la última resolución pedida (una respuesta vieja no pisa a una nueva). */
  private pedido = 0;

  async solve() {
    const n = ++this.pedido;
    this.busy = true;
    this.error = null;
    try {
      const options = { trace: this.showSteps, bnb_trace: true, integer_tolerance: this.intTolerance, ...motor.params() };
      const enviado = this.foto();
      const rep = await call<LpReport>('solve', { module: 'lp', model: this.payload(), options });
      if (n !== this.pedido) return;
      this.report = rep;
      // si se editó mientras se resolvía, el informe ya no corresponde a la hoja
      this.stale = this.foto() !== enviado;
      this.graph = null;
      if (this.canGraph) {
        try {
          this.graph = await call<GraphicData>('graphic', { model: $state.snapshot(this.model) });
        } catch {
          this.graph = null;
        }
      }
      this.tab = 'solucion';
    } catch (e) {
      if (n === this.pedido) this.error = e instanceof Error ? e.message : String(e);
    } finally {
      if (n === this.pedido) this.busy = false;
    }
  }

  async open() {
    const { shell } = await import('./shell.svelte');
    await shell.open((msg) => (this.error = msg));
  }

  async save() {
    try {
      const path = await saveLpp(this.payload() as LpModel, this.layout, this.file ?? undefined);
      if (!path) return;
      this.file = path;
      this.dirty = false;
      recientes.anotar(path, 'lp', this.model.name);
      sesion.programar();
      this.flash(t('Guardado {0}', path.split(/[\\/]/).pop()));
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    }
  }
}

export const app = new AppState();
