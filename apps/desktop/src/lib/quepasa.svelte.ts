// ¿Qué pasa si…? Análisis de escenarios en vivo.
//
// LP-ILP: deslizadores para cada costo c(j) y cada lado derecho b(i); cada cambio
// vuelve a resolver una copia del modelo (el original no se toca hasta «Aplicar al
// modelo») y la curva «Z según el parámetro» sale del análisis paramétrico exacto
// del núcleo (op `parametric`) o, en modelos enteros, de un barrido (op `sweep`).
// Demás módulos: cualquier dato numérico del modelo (sacado de las hojas de la
// especificación) se barre en un rango y se grafica la métrica elegida.

import { call } from './backend';
import { t } from './i18n.svelte';
import { motor } from './motor.svelte';
import { shown, type Block, type Field, type ProblemType, type Report } from './gtypes';
import type { Bound, LpModel, LpReport } from './types';

// ------------------------------------------------------------------ utilidades

/** Un tramo de un puntero JSON (RFC 6901). */
export function seg(s: string | number): string {
  return String(s).replace(/~/g, '~0').replace(/\//g, '~1');
}

export function ptr(...parts: (string | number | undefined | null)[]): string {
  return '/' + parts.filter((p): p is string | number => p !== undefined && p !== null && p !== '').map(seg).join('/');
}

function unseg(s: string): string {
  return s.replace(/~1/g, '/').replace(/~0/g, '~');
}

/** Lee el dato al que apunta `p` (o undefined). */
export function getAt(obj: any, p: string): any {
  let cur = obj;
  for (const k of p.split('/').slice(1).map(unseg)) {
    if (cur === null || cur === undefined) return undefined;
    cur = cur[k];
  }
  return cur;
}

/** Escribe `v` en el dato al que apunta `p` (el objeto debe existir). */
export function setAt(obj: any, p: string, v: unknown): boolean {
  const ks = p.split('/').slice(1).map(unseg);
  let cur = obj;
  for (const k of ks.slice(0, -1)) {
    if (cur === null || cur === undefined) return false;
    cur = cur[k];
  }
  if (cur === null || cur === undefined) return false;
  cur[ks[ks.length - 1]] = v;
  return true;
}

export function finite(b: Bound | null | undefined): number | null {
  return typeof b === 'number' && Number.isFinite(b) ? b : null;
}

/** Paso «redondo» para un deslizador que recorre `span` en unas 200 posiciones. */
export function niceStep(span: number): number {
  if (!(span > 0) || !Number.isFinite(span)) return 1;
  const raw = span / 200;
  const mag = 10 ** Math.floor(Math.log10(raw));
  return [1, 2, 5, 10].map((s) => s * mag).find((s) => s >= raw * 0.999) ?? raw;
}

/** Quita el ruido de coma flotante de un valor del deslizador. */
export function snap(v: number, step: number): number {
  const d = Math.max(0, Math.min(12, -Math.floor(Math.log10(step)) + 1));
  return Number(v.toFixed(d));
}

export interface Rango {
  min: number;
  max: number;
  step: number;
}

/**
 * Rango del deslizador: el rango de sensibilidad [lo, hi] (si se conoce) más un
 * margen a cada lado para poder salir de él; si el dato es no negativo, no baja de 0.
 */
export function rangoDeslizador(base: number, lo: number | null, hi: number | null): Rango {
  const scale = Math.max(Math.abs(base), 1);
  let a = lo ?? base - scale;
  let b = hi ?? base + scale;
  // rangos enormes: no se deja que la banda ocupe todo el recorrido
  a = Math.max(a, base - 10 * scale);
  b = Math.min(b, base + 10 * scale);
  const w = Math.max(b - a, scale * 0.5);
  let min = Math.min(a - w * 0.5, base);
  let max = Math.max(b + w * 0.5, base);
  if (base >= 0 && min < 0) min = 0;
  const step = niceStep(max - min);
  min = snap(Math.floor(min / step) * step, step);
  max = snap(Math.ceil(max / step) * step, step);
  return { min, max, step };
}

/** Diferencia con signo: «+2», «−1,5», «0». */
export function signo(x: number | null | undefined, fmt: (v: number) => string): string {
  if (x === null || x === undefined || !Number.isFinite(x)) return '–';
  if (Math.abs(x) < 1e-9) return '0';
  return (x > 0 ? '+' : '') + fmt(x);
}

/**
 * Ejecuta la última petición pedida, con espera (debounce), y descarta las
 * respuestas que llegan tarde: solo se aplica la de la petición más reciente.
 */
export class Ultima {
  private seq = 0;
  private timer: ReturnType<typeof setTimeout> | undefined;

  pedir<T>(fn: () => Promise<T>, aplicar: (r: T) => void, fallo: (e: unknown) => void, espera = 120, fin?: () => void) {
    const n = ++this.seq;
    clearTimeout(this.timer);
    this.timer = setTimeout(async () => {
      try {
        const r = await fn();
        if (n === this.seq) aplicar(r);
      } catch (e) {
        if (n === this.seq) fallo(e);
      } finally {
        if (n === this.seq) fin?.();
      }
    }, espera);
  }

  cancelar() {
    this.seq++;
    clearTimeout(this.timer);
  }

  /** ¿Hay una petición en curso o en espera? */
  get id() {
    return this.seq;
  }
}

export function mensaje(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

// ------------------------------------------------------------------ LP-ILP

export type Clase = 'cost' | 'rhs';

/** Un coeficiente del modelo que se puede mover. */
export interface ParamLp {
  key: string;
  kind: Clase;
  index: number;
  name: string;
  /** «c(Puertas)» o «b(Planta3)». */
  label: string;
  base: number;
  /** Rango de sensibilidad de la solución base (null = sin límite o desconocido). */
  lo: number | null;
  hi: number | null;
  /** ¿Se conoce el rango? (no en modelos enteros) */
  ranged: boolean;
  infLo: boolean;
  infHi: boolean;
  rango: Rango;
  path: string;
}

export interface Tramo {
  from: number;
  to: number;
  status: string;
  slope: number | null;
  objective_from: number | null;
  objective_to: number | null;
  solution?: Record<string, number>;
}

export interface Parametrico {
  kind: Clase;
  index: number;
  name: string;
  from: number;
  to: number;
  segments: Tramo[];
  breakpoints: number[];
}

export interface PuntoBarrido {
  value: number;
  status: string;
  metric: number | null;
  metric_label?: string;
  headline?: string | null;
  solution?: Record<string, number>;
  error?: string;
}

export interface Barrido {
  path: string;
  base_value: unknown;
  points: PuntoBarrido[];
  solution_changes: { between: [number, number]; before: string[]; after: string[] }[];
}

/** Curva Z(θ) lista para graficar: exacta (paramétrico) o muestreada (barrido). */
export interface Curva {
  exacta: boolean;
  key: string;
  from: number;
  to: number;
  pts: [number, number][];
  quiebres: number[];
  tramos: Tramo[];
  barrido: Barrido | null;
}

export function lpOptions(intTolerance: number): Record<string, unknown> {
  return { trace: false, bnb_trace: false, diagnose: false, integer_tolerance: intTolerance, ...motor.params() };
}

export function keyOf(kind: Clase, index: number): string {
  return `${kind}:${index}`;
}

export function pathOf(kind: Clase, index: number): string {
  return kind === 'cost' ? ptr('variables', index, 'cost') : ptr('constraints', index, 'rhs');
}

/** Los coeficientes del modelo, con los rangos de sensibilidad del informe base. */
export function paramsLp(model: LpModel, rep: LpReport | null): ParamLp[] {
  const ranged = !!rep?.sensitivity && rep.status === 'optimal';
  const out: ParamLp[] = [];
  model.variables.forEach((v, j) => {
    const r = rep?.variables[j];
    const lo = ranged ? finite(r?.cost_min) : null;
    const hi = ranged ? finite(r?.cost_max) : null;
    out.push({
      key: keyOf('cost', j),
      kind: 'cost',
      index: j,
      name: v.name,
      label: `c(${v.name})`,
      base: Number(v.cost) || 0,
      lo,
      hi,
      ranged,
      infLo: ranged && r?.cost_min === '-M',
      infHi: ranged && r?.cost_max === 'M',
      rango: rangoDeslizador(Number(v.cost) || 0, lo, hi),
      path: pathOf('cost', j)
    });
  });
  model.constraints.forEach((c, i) => {
    const r = rep?.constraints[i];
    const lo = ranged ? finite(r?.rhs_min) : null;
    const hi = ranged ? finite(r?.rhs_max) : null;
    out.push({
      key: keyOf('rhs', i),
      kind: 'rhs',
      index: i,
      name: c.name,
      label: `b(${c.name})`,
      base: Number(c.rhs) || 0,
      lo,
      hi,
      ranged,
      infLo: ranged && r?.rhs_min === '-M',
      infHi: ranged && r?.rhs_max === 'M',
      rango: rangoDeslizador(Number(c.rhs) || 0, lo, hi),
      path: pathOf('rhs', i)
    });
  });
  return out;
}

/** El modelo con los cambios aplicados (copia; el original no se toca). */
export function conCambios(model: LpModel, cambios: Record<string, number>, salvo?: string): LpModel {
  const m = structuredClone(model);
  for (const [k, v] of Object.entries(cambios)) {
    if (k === salvo) continue;
    const [kind, i] = k.split(':');
    const idx = Number(i);
    if (kind === 'cost' && m.variables[idx]) m.variables[idx].cost = v;
    else if (kind === 'rhs' && m.constraints[idx]) m.constraints[idx].rhs = v;
  }
  return m;
}

/** ¿Dentro del rango de sensibilidad? (null si no se conoce) */
export function dentro(p: ParamLp, v: number): boolean | null {
  if (!p.ranged) return null;
  const tol = 1e-9 * Math.max(1, Math.abs(v));
  if (p.lo !== null && v < p.lo - tol) return false;
  if (p.hi !== null && v > p.hi + tol) return false;
  if (p.lo === null && !p.infLo) return null;
  if (p.hi === null && !p.infHi) return null;
  return true;
}

/** Variables con valor distinto de cero (la «estructura» de la solución). */
export function soporte(rep: LpReport | null): Set<string> {
  return new Set((rep?.variables ?? []).filter((v) => Math.abs(v.value) > 1e-9).map((v) => v.name));
}

export function basicas(rep: LpReport | null): Set<string> {
  return new Set((rep?.variables ?? []).filter((v) => v.basis_status === 'basic').map((v) => v.name));
}

/** Z en θ según la curva (interpolación en el tramo que lo contiene). */
export function zEn(c: Curva, x: number): number | null {
  if (c.exacta) {
    for (const s of c.tramos) {
      if (s.status !== 'optimal' || s.slope === null || s.objective_from === null) continue;
      const tol = 1e-7 * Math.max(1, Math.abs(x));
      if (x >= s.from - tol && x <= s.to + tol) return s.objective_from + s.slope * (x - s.from);
    }
    return null;
  }
  const ps = c.pts.filter((p) => Number.isFinite(p[1]));
  for (let k = 0; k + 1 < ps.length; k++) {
    const [a, b] = [ps[k], ps[k + 1]];
    if (x >= a[0] && x <= b[0]) return b[0] === a[0] ? a[1] : a[1] + ((b[1] - a[1]) * (x - a[0])) / (b[0] - a[0]);
  }
  return null;
}

/**
 * Pasa el resultado paramétrico a una curva: los extremos de los tramos se
 * ajustan (el núcleo avanza ε para entrar en cada tramo nuevo) y los tramos sin
 * solución óptima cortan la línea.
 */
export function curvaExacta(p: Parametrico, key: string): Curva {
  const span = Math.max(p.to - p.from, 1e-9);
  const eps = span * 1e-5;
  const tramos: Tramo[] = [];
  for (const s of p.segments) {
    const prev = tramos[tramos.length - 1];
    let from = s.from;
    if (!prev && Math.abs(from - p.from) < eps) from = p.from;
    if (prev && Math.abs(from - prev.to) < eps) from = prev.to;
    let to = s.to;
    if (Math.abs(to - p.to) < eps) to = p.to;
    const z0 = s.objective_from !== null && s.slope !== null ? s.objective_from + s.slope * (from - s.from) : s.objective_from;
    const z1 = s.objective_from !== null && s.slope !== null ? s.objective_from + s.slope * (to - s.from) : s.objective_to;
    tramos.push({ ...s, from, to, objective_from: z0, objective_to: z1 });
  }
  const pts: [number, number][] = [];
  for (const s of tramos) {
    if (s.status === 'optimal' && s.objective_from !== null && s.objective_to !== null) {
      pts.push([s.from, s.objective_from], [s.to, s.objective_to]);
    } else if (pts.length && Number.isFinite(pts[pts.length - 1][1])) {
      pts.push([NaN, NaN]);
    }
  }
  const quiebres = tramos.slice(0, -1).map((s) => s.to);
  return { exacta: true, key, from: p.from, to: p.to, pts, quiebres, tramos, barrido: null };
}

export function curvaMuestreada(b: Barrido, key: string, from: number, to: number): Curva {
  const pts: [number, number][] = b.points.map((p) => [p.value, p.metric ?? NaN] as [number, number]).map((p) => (Number.isFinite(p[1]) ? p : [NaN, NaN]));
  // un cambio de solución entre dos puntos se marca en el medio
  const quiebres = b.solution_changes.map((c) => (c.between[0] + c.between[1]) / 2);
  return { exacta: false, key, from, to, pts, quiebres, tramos: [], barrido: b };
}

/** Texto corto de una solución: «Puertas = 2, Ventanas = 6» (solo las distintas de cero). */
export function textoSolucion(sol: Record<string, number> | undefined, fmt: (v: number) => string, max = 6): string {
  if (!sol) return '–';
  const nz = Object.entries(sol).filter(([, v]) => Math.abs(v) > 1e-6);
  if (!nz.length) return t('todas en 0');
  const s = nz.slice(0, max).map(([k, v]) => `${k} = ${fmt(v)}`).join(', ');
  return nz.length > max ? `${s}, … (+${nz.length - max})` : s;
}

class LpQuePasa {
  /** Huella del modelo con el que se calcularon la base y los cambios. */
  firma = $state('');
  base = $state<LpReport | null>(null);
  actual = $state<LpReport | null>(null);
  cambios = $state<Record<string, number>>({});
  elegido = $state<string>('');
  cargando = $state(false);
  resolviendo = $state(false);
  error = $state<string | null>(null);
  curva = $state<Curva | null>(null);
  curvaCargando = $state(false);
  curvaError = $state<string | null>(null);
  /** Rango de la curva elegido por el usuario (por coeficiente). */
  rangos = $state<Record<string, { from: number; to: number }>>({});
  filtro = $state('');
}

export const lpq = new LpQuePasa();

// ------------------------------------------------------------------ demás módulos

/** Un dato numérico del modelo de un módulo genérico. */
export interface Dato {
  path: string;
  label: string;
  grupo: string;
  valor: number;
  entero: boolean;
}

const numerico = (f: Field) => f.kind.type === 'number' || f.kind.type === 'integer' || f.kind.type === 'bound';

/** Nombre de una fila de una tabla de registros. */
function nombreFila(row: any, cols: Field[], i: number, quien: string): string {
  const txt = cols.find((c) => c.kind.type === 'text' && typeof row?.[c.key] === 'string' && row[c.key].trim());
  if (txt) return row[txt.key];
  for (const k of ['name', 'label', 'id']) if (typeof row?.[k] === 'string' && row[k]) return row[k];
  return `${quien} ${i + 1}`;
}

/** Todos los datos numéricos del modelo según las hojas del tipo de problema. */
export function datosDelModelo(pt: ProblemType | null, model: any): Dato[] {
  if (!pt || !model) return [];
  const out: Dato[] = [];
  const vistos = new Set<string>();
  const add = (d: Dato) => {
    if (vistos.has(d.path)) return;
    vistos.add(d.path);
    out.push(d);
  };
  for (const w of pt.layout) {
    if (w.type === 'params') {
      const obj = w.path ? model[w.path] : model;
      if (!obj) continue;
      for (const f of w.fields) {
        if (!numerico(f) || !shown(f, (k) => obj?.[k] ?? model[k])) continue;
        const v = obj[f.key];
        if (typeof v !== 'number' || !Number.isFinite(v)) continue;
        add({ path: ptr(w.path, f.key), label: f.label, grupo: w.title ?? t('Parámetros'), valor: v, entero: f.kind.type === 'integer' });
      }
    } else if (w.type === 'records') {
      const list = (model[w.path] ?? []) as any[];
      list.forEach((row, i) => {
        const quien = nombreFila(row, w.columns, i, t('Fila'));
        for (const f of w.columns) {
          if (!numerico(f) || !shown(f, (k) => row?.[k] ?? model[k])) continue;
          const v = row?.[f.key];
          if (typeof v !== 'number' || !Number.isFinite(v)) continue;
          add({ path: ptr(w.path, i, f.key), label: `${quien} · ${f.label}`, grupo: w.title ?? f.label, valor: v, entero: f.kind.type === 'integer' });
        }
      });
    } else if (w.type === 'matrix') {
      const rows = (model[w.rows.path] ?? []) as any[];
      const cols = (model[w.cols.path] ?? []) as any[];
      const mat = (model[w.path] ?? []) as any[][];
      const rn = (i: number) => String(rows[i]?.[w.rows.label_key] ?? `${w.rows.label} ${i + 1}`);
      const cn = (j: number) => String(cols[j]?.[w.cols.label_key] ?? `${w.cols.label} ${j + 1}`);
      const grupo = w.title ?? w.cell.label;
      mat.forEach((r, i) =>
        (r ?? []).forEach((v, j) => {
          if (typeof v !== 'number' || !Number.isFinite(v)) return;
          add({ path: ptr(w.path, i, j), label: `${w.cell.label}: ${rn(i)} → ${cn(j)}`, grupo, valor: v, entero: w.cell.kind.type === 'integer' });
        })
      );
      // datos de las filas y columnas (oferta, demanda, capacidades…)
      for (const [ax, list, name] of [
        [w.rows, rows, rn],
        [w.cols, cols, cn]
      ] as const) {
        for (const f of ax.extra) {
          if (!numerico(f)) continue;
          list.forEach((it, i) => {
            const v = it?.[f.key];
            if (typeof v !== 'number' || !Number.isFinite(v)) return;
            add({ path: ptr(ax.path, i, f.key), label: `${f.label}: ${name(i)}`, grupo: f.label, valor: v, entero: f.kind.type === 'integer' });
          });
        }
      }
    }
  }
  return out;
}

/** Etiquetas de las métricas numéricas del informe (para elegir qué graficar). */
export function metricasDe(rep: Report | null): string[] {
  const out: string[] = [];
  for (const s of rep?.sections ?? []) {
    for (const b of s.blocks) {
      if (b.type !== 'metrics') continue;
      for (const it of b.items) {
        if (out.includes(it.label)) continue;
        if (numDe(it.value) !== null) out.push(it.label);
      }
    }
  }
  return out;
}

export function numDe(v: unknown): number | null {
  if (typeof v === 'number') return Number.isFinite(v) ? v : null;
  if (typeof v === 'string') {
    const x = Number(v.replace('−', '-').trim().replace(/%$/, '').trim());
    return Number.isFinite(x) && v.trim() !== '' ? x : null;
  }
  return null;
}

/** El valor de una métrica del informe, por su etiqueta exacta (o la primera). */
export function metricaDe(rep: Report | null, label: string | null): { label: string; value: number | null; unit?: string } | null {
  for (const s of rep?.sections ?? []) {
    for (const b of s.blocks) {
      if (b.type !== 'metrics') continue;
      for (const it of b.items) {
        if (label === null || it.label === label) {
          const v = numDe(it.value);
          if (label === null && v === null) continue;
          return { label: it.label, value: v, unit: it.unit };
        }
      }
    }
  }
  return null;
}

/**
 * Cómo pedirle la métrica al núcleo: por su etiqueta, o por su puntero en el
 * informe si otra etiqueta anterior la contiene (el núcleo busca por «contiene»).
 */
export function metricaArg(rep: Report | null, label: string | null): string | null {
  if (!label || !rep) return label;
  const l = label.toLowerCase();
  // las métricas en el orden en que las recorre el núcleo, con su puntero
  const todas: { label: string; ok: boolean; p: string }[] = [];
  rep.sections.forEach((s, si) =>
    s.blocks.forEach((b, bi) => {
      if (b.type === 'metrics')
        b.items.forEach((it, k) => todas.push({ label: it.label, ok: numDe(it.value) !== null, p: ptr('sections', si, 'blocks', bi, 'items', k, 'value') }));
    })
  );
  const exacta = todas.find((x) => x.label === label);
  const primera = todas.find((x) => x.ok && (x.label.toLowerCase() === l || x.label.toLowerCase().includes(l)));
  return primera && exacta && primera !== exacta ? exacta.p : label;
}

/** El primer bloque de métricas del informe (para mostrar el resultado en vivo). */
export function primerasMetricas(rep: Report | null): Block | null {
  for (const s of rep?.sections ?? []) for (const b of s.blocks) if (b.type === 'metrics') return b;
  return null;
}

/** Rango por omisión de un barrido: ±50 % del valor actual. */
export function rangoPorOmision(v: number, entero: boolean): { from: number; to: number } {
  if (Math.abs(v) < 1e-12) return { from: 0, to: entero ? 10 : 1 };
  let a = v * 0.5;
  let b = v * 1.5;
  if (a > b) [a, b] = [b, a];
  if (entero) {
    a = Math.floor(a);
    b = Math.ceil(b);
  }
  const st = niceStep(b - a);
  return { from: snap(a, st / 10), to: snap(b, st / 10) };
}

/** Los valores del barrido; en datos enteros, solo enteros (sin repetir). */
export function valoresBarrido(from: number, to: number, pasos: number, entero: boolean): number[] | { from: number; to: number; steps: number } {
  const steps = Math.max(1, Math.min(400, Math.round(pasos)));
  if (!entero) return { from, to, steps };
  const out = new Set<number>();
  for (let k = 0; k <= steps; k++) out.add(Math.round(from + ((to - from) * k) / steps));
  return [...out].sort((a, b) => a - b);
}

export class GenQuePasa {
  abierto = $state(false);
  path = $state('');
  from = $state(0);
  to = $state(1);
  pasos = $state(20);
  metrica = $state<string | null>(null);
  filtro = $state('');
  auto = $state(true);
  barrido = $state<Barrido | null>(null);
  cargando = $state(false);
  error = $state<string | null>(null);
  /** Valor del deslizador en vivo (null = el del modelo). */
  vivo = $state<number | null>(null);
  vivoRep = $state<Report | null>(null);
  vivoCargando = $state(false);
  vivoError = $state<string | null>(null);
  /** Lo que tardó la última resolución (ms). */
  ms = $state(0);
  /** Informe del modelo actual (si el de la app quedó desactualizado). */
  base = $state<Report | null>(null);
  firmaBase = $state('');
}

const gens = new Map<string, GenQuePasa>();
export function genq(modulo: string): GenQuePasa {
  let g = gens.get(modulo);
  if (!g) {
    g = new GenQuePasa();
    gens.set(modulo, g);
  }
  return g;
}

export async function barrer(args: { module: string; model: unknown; options: unknown; path: string; values: unknown; metric?: string | null }): Promise<Barrido> {
  const a: Record<string, unknown> = { ...args };
  if (!args.metric) delete a.metric;
  return call<Barrido>('sweep', a);
}
