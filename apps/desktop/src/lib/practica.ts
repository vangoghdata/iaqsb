// Modo práctica del módulo LP-ILP: lógica pura (sin Svelte ni llamadas al núcleo).
//
// El estudiante hace a mano el símplex de tablas y el branch and bound; aquí se
// comprueba cada decisión. Todo es aritmética exacta con fracciones (BigInt) y
// valores con M simbólica (m·M + c), así que el camino del estudiante puede
// apartarse del de la regla de Dantzig y seguir siendo correcto.
//
// Las explicaciones se devuelven como claves en español con sus argumentos
// ({ key, args }); la interfaz las pasa por t() para traducirlas.

import type { LpModel, TableauSnapshot, Variable } from './types';

// ------------------------------------------------------------ fracciones

function gcd(a: bigint, b: bigint): bigint {
  if (a < 0n) a = -a;
  if (b < 0n) b = -b;
  while (b) [a, b] = [b, a % b];
  return a;
}

/** Fracción exacta n/d con d > 0 y sin factores comunes. */
export class Frac {
  readonly n: bigint;
  readonly d: bigint;
  constructor(n: bigint, d: bigint = 1n) {
    if (d === 0n) throw new Error('división entre cero');
    if (d < 0n) {
      n = -n;
      d = -d;
    }
    const g = gcd(n, d) || 1n;
    this.n = n / g;
    this.d = d / g;
  }
  static readonly ZERO = new Frac(0n);
  static readonly ONE = new Frac(1n);
  static int(k: number): Frac {
    return new Frac(BigInt(k));
  }
  add(o: Frac): Frac {
    return new Frac(this.n * o.d + o.n * this.d, this.d * o.d);
  }
  sub(o: Frac): Frac {
    return new Frac(this.n * o.d - o.n * this.d, this.d * o.d);
  }
  mul(o: Frac): Frac {
    return new Frac(this.n * o.n, this.d * o.d);
  }
  div(o: Frac): Frac {
    return new Frac(this.n * o.d, this.d * o.n);
  }
  neg(): Frac {
    return new Frac(-this.n, this.d);
  }
  abs(): Frac {
    return this.n < 0n ? this.neg() : this;
  }
  sign(): -1 | 0 | 1 {
    return this.n > 0n ? 1 : this.n < 0n ? -1 : 0;
  }
  isZero(): boolean {
    return this.n === 0n;
  }
  cmp(o: Frac): -1 | 0 | 1 {
    const x = this.n * o.d - o.n * this.d;
    return x > 0n ? 1 : x < 0n ? -1 : 0;
  }
  eq(o: Frac): boolean {
    return this.n === o.n && this.d === o.d;
  }
  isInt(): boolean {
    return this.d === 1n;
  }
  toNumber(): number {
    // división en coma flotante cuidando números grandes
    const q = this.n / this.d;
    const r = this.n - q * this.d;
    return Number(q) + Number(r) / Number(this.d);
  }
  /** «3/2», «−1/4», «5». */
  toString(): string {
    const s = this.d === 1n ? (this.n < 0n ? -this.n : this.n).toString() : `${(this.n < 0n ? -this.n : this.n).toString()}/${this.d.toString()}`;
    return this.n < 0n ? '−' + s : s;
  }
}

/** Número decimal exacto ("0.25", "-1.5e-3") a fracción. */
function decimalFrac(s: string): Frac | null {
  const m = /^([+-]?)(\d*)(?:\.(\d*))?(?:e([+-]?\d+))?$/i.exec(s);
  if (!m || (m[2] === '' && (m[3] ?? '') === '')) return null;
  const sign = m[1] === '-' ? -1n : 1n;
  const ent = m[2] || '0';
  const dec = m[3] ?? '';
  const exp = Number(m[4] ?? 0);
  let n = BigInt(ent + dec) * sign;
  let d = 10n ** BigInt(dec.length);
  if (exp > 0) n *= 10n ** BigInt(exp);
  else if (exp < 0) d *= 10n ** BigInt(-exp);
  return new Frac(n, d);
}

/** Aproximación por fracciones continuas con denominador ≤ maxDen, si es casi exacta. */
function niceFrac(x: number, maxDen = 1000): Frac | null {
  const sign = x < 0 ? -1 : 1;
  let y = Math.abs(x);
  let h0 = 0,
    h1 = 1,
    k0 = 1,
    k1 = 0;
  for (let i = 0; i < 40; i++) {
    const a = Math.floor(y);
    const h2 = a * h1 + h0;
    const k2 = a * k1 + k0;
    if (k2 > maxDen) break;
    h0 = h1;
    h1 = h2;
    k0 = k1;
    k1 = k2;
    if (Math.abs(Math.abs(x) - h1 / k1) <= 1e-9 * Math.max(1, Math.abs(x))) return new Frac(BigInt(sign * h1), BigInt(k1));
    const f = y - a;
    if (f < 1e-15) break;
    y = 1 / f;
  }
  return null;
}

/** Número del núcleo (ya redondeado a 9 decimales) a fracción: 0.5 → 1/2, 1.333333333 → 4/3. */
export function fracOf(x: number): Frac {
  if (!Number.isFinite(x)) throw new Error('valor no finito en la tabla');
  if (x === 0) return Frac.ZERO;
  if (Number.isInteger(x) && Math.abs(x) < Number.MAX_SAFE_INTEGER) return Frac.int(x);
  return niceFrac(x) ?? decimalFrac(String(x)) ?? Frac.ZERO;
}

/** Lo que escribe el estudiante: «1/2», «-3/4», «0,5», «−2», «1.5/3». Null si no se entiende. */
export function parseFrac(s: string): Frac | null {
  let t = s.trim().replace(/\s+/g, '').replace(/[−–]/g, '-');
  if (t === '') return null;
  t = t.replace(/,/g, '.');
  const parts = t.split('/');
  if (parts.length > 2) return null;
  const a = decimalFrac(parts[0]);
  if (!a) return null;
  if (parts.length === 1) return a;
  const b = decimalFrac(parts[1]);
  if (!b || b.isZero()) return null;
  return a.div(b);
}

/** ¿El valor escrito coincide con el exacto? Tolerancia de redondeo a 2 decimales (relativa si es grande). */
export function closeEnough(typed: Frac, exact: Frac): boolean {
  if (typed.eq(exact)) return true;
  const d = Math.abs(typed.toNumber() - exact.toNumber());
  return d <= 0.005 * Math.max(1, Math.abs(exact.toNumber())) + 1e-12;
}

// ------------------------------------------------------------ valores con M

/** m·M + c, con M «muy grande»: se comparan primero los coeficientes de M. */
export interface MV {
  m: Frac;
  c: Frac;
}

export const MV0: MV = { m: Frac.ZERO, c: Frac.ZERO };
export const mv = (m: Frac, c: Frac): MV => ({ m, c });
export const mvAdd = (a: MV, b: MV): MV => ({ m: a.m.add(b.m), c: a.c.add(b.c) });
export const mvSub = (a: MV, b: MV): MV => ({ m: a.m.sub(b.m), c: a.c.sub(b.c) });
export const mvScale = (a: MV, k: Frac): MV => ({ m: a.m.mul(k), c: a.c.mul(k) });
export const mvSign = (a: MV): -1 | 0 | 1 => (a.m.sign() !== 0 ? a.m.sign() : a.c.sign());
export const mvCmp = (a: MV, b: MV): -1 | 0 | 1 => mvSign(mvSub(a, b));
export const mvIsZero = (a: MV): boolean => a.m.isZero() && a.c.isZero();
export const mvNum = (a: MV): { m: number; c: number } => ({ m: a.m.toNumber(), c: a.c.toNumber() });

/** Formato de una fracción: exacta («3/2») o decimal («1.5»). */
export function fmtFrac(x: Frac, frac: boolean, digits = 4): string {
  if (frac || x.isInt()) return x.toString();
  let s = x.toNumber().toFixed(digits);
  if (s.includes('.')) s = s.replace(/0+$/, '').replace(/\.$/, '');
  if (s === '-0') s = '0';
  return s.replace('-', '−');
}

/** «2M + 3», «−M», «1/2M − 4». */
export function fmtMV(v: MV, frac: boolean): string {
  if (v.m.isZero()) return fmtFrac(v.c, frac);
  const am = v.m.abs();
  const mp = (v.m.sign() < 0 ? '−' : '') + (am.eq(Frac.ONE) ? 'M' : `${fmtFrac(am, frac)}M`);
  if (v.c.isZero()) return mp;
  return `${mp} ${v.c.sign() > 0 ? '+' : '−'} ${fmtFrac(v.c.abs(), frac)}`;
}

// ------------------------------------------------------------ tabla del símplex

/** Mensaje para la interfaz: clave en español y argumentos para t(). */
export interface Msg {
  key: string;
  args?: (string | number)[];
}

/**
 * Marca un texto para traducir y lo guarda con sus argumentos; la interfaz lo
 * traduce al mostrarlo con el t() de i18n.svelte.ts (así este archivo no depende
 * de Svelte y los textos siguen apareciendo como t('…') al buscarlos).
 */
function t(key: string, ...args: (string | number)[]): Msg {
  return args.length ? { key, args } : { key };
}
export { t as msg };

export type Sense = 'max' | 'min';

/** Tabla del símplex con aritmética exacta. Se trata como inmutable. */
export interface PTableau {
  sense: Sense;
  columns: string[];
  /** C(j) en el sentido original del problema (la M aparece solo en las artificiales). */
  cj: MV[];
  /** Índice de columna básica de cada fila. */
  basis: number[];
  rows: Frac[][];
  rhs: Frac[];
  /** Término constante del objetivo (por desplazamiento de cotas). */
  offset: MV;
  /** Iteraciones hechas desde la tabla inicial. */
  iteration: number;
}

/** Construye la tabla exacta a partir de la tabla inicial del núcleo (iteración 0). */
export function fromSnapshot(s: TableauSnapshot, sense: Sense): PTableau {
  const cj = s.cj.map((v) => mv(fracOf(v.m), fracOf(v.c)));
  const basis = s.basis.map((b) => {
    const j = s.columns.indexOf(b);
    if (j < 0) throw new Error(`la columna básica ${b} no está en la tabla`);
    return j;
  });
  const rows = s.rows.map((r) => r.map(fracOf));
  const rhs = s.rhs.map(fracOf);
  // offset = Z − Σ C(B)·b, para reproducir exactamente el valor del objetivo del núcleo
  let z = MV0;
  basis.forEach((j, r) => (z = mvAdd(z, mvScale(cj[j], rhs[r]))));
  const offset = mvSub(mv(fracOf(s.objective.m), fracOf(s.objective.c)), z);
  return { sense, columns: [...s.columns], cj, basis, rows, rhs, offset, iteration: 0 };
}

export const isArtificial = (t: PTableau, j: number): boolean => !t.cj[j].m.isZero();
export const isBasic = (t: PTableau, j: number): boolean => t.basis.includes(j);

/** C(B) de cada fila. */
export const cb = (t: PTableau): MV[] => t.basis.map((j) => t.cj[j]);

/** Z(j) = Σ C(B)·a(r,j). */
export function zj(t: PTableau): MV[] {
  const c = cb(t);
  return t.columns.map((_, j) => {
    let z = MV0;
    t.rows.forEach((row, r) => {
      if (!row[j].isZero()) z = mvAdd(z, mvScale(c[r], row[j]));
    });
    return z;
  });
}

/** C(j) − Z(j) (cero en las básicas). */
export function reduced(t: PTableau): MV[] {
  const z = zj(t);
  return t.cj.map((c, j) => (isBasic(t, j) ? MV0 : mvSub(c, z[j])));
}

/** Valor del objetivo de la tabla: Σ C(B)·b + constante. */
export function objective(t: PTableau): MV {
  let z = t.offset;
  t.basis.forEach((j, r) => (z = mvAdd(z, mvScale(t.cj[j], t.rhs[r]))));
  return z;
}

/** ¿Este C(j)−Z(j) mejora el objetivo? (positivo al maximizar, negativo al minimizar) */
export const improves = (sense: Sense, d: MV): boolean => (sense === 'max' ? mvSign(d) > 0 : mvSign(d) < 0);
/** ¿Mejora la parte de M? (fase de sacar artificiales) */
const improvesM = (sense: Sense, d: MV): boolean => (sense === 'max' ? d.m.sign() > 0 : d.m.sign() < 0);

/** Columnas no básicas que mejoran el objetivo. */
export function improving(t: PTableau): number[] {
  const d = reduced(t);
  return t.columns.map((_, j) => j).filter((j) => !isBasic(t, j) && improves(t.sense, d[j]));
}

/** Las columnas que elige la regla de Dantzig (el C(j)−Z(j) más favorable; varias si empatan). */
export function dantzig(t: PTableau): number[] {
  const d = reduced(t);
  const cand = improving(t);
  if (!cand.length) return [];
  const better = (a: MV, b: MV) => (t.sense === 'max' ? mvCmp(a, b) > 0 : mvCmp(a, b) < 0);
  let best = cand[0];
  for (const j of cand) if (better(d[j], d[best])) best = j;
  return cand.filter((j) => mvCmp(d[j], d[best]) === 0);
}

/** Filas con artificiales básicas en nivel positivo. */
export function positiveArtificials(t: PTableau): number[] {
  return t.basis.map((j, r) => (isArtificial(t, j) && t.rhs[r].sign() > 0 ? r : -1)).filter((r) => r >= 0);
}

/** ¿Ya no se puede reducir la parte M y queda una artificial positiva? → no factible. */
export function provablyInfeasible(t: PTableau): boolean {
  if (!positiveArtificials(t).length) return false;
  const d = reduced(t);
  return !t.columns.some((_, j) => !isBasic(t, j) && improvesM(t.sense, d[j]));
}

/** ¿La columna no tiene ningún elemento positivo? */
export const noPositive = (t: PTableau, e: number): boolean => t.rows.every((row) => row[e].sign() <= 0);

/** Cociente b(r)/a(r,e) de cada fila (null si a(r,e) ≤ 0). */
export function ratios(t: PTableau, e: number): (Frac | null)[] {
  return t.rows.map((row, r) => (row[e].sign() > 0 ? t.rhs[r].div(row[e]) : null));
}

/** Filas con el cociente mínimo (varias si hay empate). */
export function minRatioRows(t: PTableau, e: number): number[] {
  const q = ratios(t, e);
  let best: Frac | null = null;
  for (const x of q) if (x && (!best || x.cmp(best) < 0)) best = x;
  if (!best) return [];
  return q.map((x, r) => (x && x.eq(best!) ? r : -1)).filter((r) => r >= 0);
}

/** Nueva fila pivote: la fila r dividida entre el pivote (columnas y lado derecho al final). */
export function newPivotRow(t: PTableau, r: number, e: number): Frac[] {
  const p = t.rows[r][e];
  return [...t.rows[r].map((x) => x.div(p)), t.rhs[r].div(p)];
}

/** Pivoteo exacto: entra la columna e, sale la variable básica de la fila r. */
export function pivot(t: PTableau, r: number, e: number): PTableau {
  const p = t.rows[r][e];
  if (p.sign() === 0) throw new Error('pivote nulo');
  const prow = t.rows[r].map((x) => x.div(p));
  const pb = t.rhs[r].div(p);
  const rows = t.rows.map((row, i) => {
    if (i === r) return prow;
    const f = row[e];
    if (f.isZero()) return row;
    return row.map((x, j) => x.sub(f.mul(prow[j])));
  });
  const rhs = t.rhs.map((b, i) => (i === r ? pb : t.rows[i][e].isZero() ? b : b.sub(t.rows[i][e].mul(pb))));
  const basis = [...t.basis];
  basis[r] = e;
  return { ...t, rows, rhs, basis, iteration: t.iteration + 1 };
}

/** Valor de cada columna en la solución básica actual. */
export function columnValues(t: PTableau): Frac[] {
  const v = t.columns.map(() => Frac.ZERO);
  t.basis.forEach((j, r) => (v[j] = t.rhs[r]));
  return v;
}

/** Hay óptimos alternativos: alguna no básica (no artificial) con C(j)−Z(j) = 0. */
export function alternateOptima(t: PTableau): boolean {
  const d = reduced(t);
  return t.columns.some((_, j) => !isBasic(t, j) && !isArtificial(t, j) && mvIsZero(d[j]));
}

// ------------------------------------------------------------ comprobación de decisiones

export type Level = 'ok' | 'warn' | 'bad' | 'info';

export interface Verdict {
  ok: boolean;
  level: Level;
  msgs: Msg[];
}

/** Comprueba la variable que entra elegida por el estudiante. */
export function checkEntering(tb: PTableau, j: number): Verdict {
  const d = reduced(tb);
  const name = tb.columns[j];
  if (isBasic(tb, j))
    return { ok: false, level: 'bad', msgs: [t('{0} ya es básica: su C(j)−Z(j) es 0 y no puede entrar.', name)] };
  if (!improves(tb.sense, d[j])) {
    const msgs: Msg[] = [
      tb.sense === 'max'
        ? t('C(j)−Z(j) de {0} es {1}: al maximizar solo mejora una columna con C(j)−Z(j) positivo.', name, fmtMV(d[j], true))
        : t('C(j)−Z(j) de {0} es {1}: al minimizar solo mejora una columna con C(j)−Z(j) negativo.', name, fmtMV(d[j], true))
    ];
    if (!improving(tb).length) msgs.push(t('Ninguna columna mejora: hay que declarar la conclusión de la tabla.'));
    return { ok: false, level: 'bad', msgs };
  }
  const best = dantzig(tb);
  if (best.includes(j)) {
    const msgs: Msg[] = [
      tb.sense === 'max'
        ? t('Correcto: entra {0}, con el C(j)−Z(j) más positivo ({1}).', name, fmtMV(d[j], true))
        : t('Correcto: entra {0}, con el C(j)−Z(j) más negativo ({1}).', name, fmtMV(d[j], true))
    ];
    if (best.length > 1) msgs.push(t('Hay empate en el C(j)−Z(j); cualquiera de las empatadas sirve.'));
    return { ok: true, level: 'ok', msgs };
  }
  const b = best[0];
  return {
    ok: true,
    level: 'warn',
    msgs: [
      t('Válida: {0} mejora el objetivo (C(j)−Z(j) = {1}).', name, fmtMV(d[j], true)),
      tb.sense === 'max'
        ? t('Pero la regla de Dantzig elige {0} (el C(j)−Z(j) más positivo, {1}). Seguimos con {2}.', tb.columns[b], fmtMV(d[b], true), name)
        : t('Pero la regla de Dantzig elige {0} (el C(j)−Z(j) más negativo, {1}). Seguimos con {2}.', tb.columns[b], fmtMV(d[b], true), name)
    ]
  };
}

/** Comprueba la fila que sale (prueba del cociente mínimo). */
export function checkLeaving(tb: PTableau, e: number, r: number): Verdict {
  const q = ratios(tb, e);
  const mins = minRatioRows(tb, e);
  const out = tb.columns[tb.basis[r]];
  const a = tb.rows[r][e];
  if (!mins.length)
    return { ok: false, level: 'bad', msgs: [t('La columna {0} no tiene elementos positivos: ninguna fila limita su crecimiento.', tb.columns[e])] };
  const best = q[mins[0]]!;
  if (!q[r])
    return {
      ok: false,
      level: 'bad',
      msgs: [
        t('En la fila de {0} el elemento de la columna {1} es {2}: solo se calculan cocientes con elementos positivos.', out, tb.columns[e], a.toString())
      ]
    };
  if (!mins.includes(r))
    return {
      ok: false,
      level: 'bad',
      msgs: [
        t('El cociente de la fila de {0} es {1}, pero el mínimo es {2} (fila de {3}).', out, q[r]!.toString(), best.toString(), tb.columns[tb.basis[mins[0]]]),
        t('Si saliera {0}, la variable básica de la fila del mínimo se haría negativa.', out)
      ]
    };
  const msgs: Msg[] = [t('Correcto: sale {0}, con el cociente mínimo {1}.', out, best.toString())];
  if (mins.length > 1) msgs.push(t('Hay empate en el cociente mínimo: cualquiera de esas filas sirve, y la nueva tabla será degenerada (una básica valdrá 0).'));
  else if (best.isZero()) msgs.push(t('El cociente es 0: es un pivoteo degenerado; el objetivo no cambia en esta iteración.'));
  return { ok: true, level: mins.length > 1 ? 'warn' : 'ok', msgs };
}

/** Comprueba la nueva fila pivote escrita por el estudiante. `cells` dice qué casillas están bien. */
export function checkPivotRow(tb: PTableau, r: number, e: number, typed: string[]): { ok: boolean; cells: (boolean | null)[]; exact: Frac[] } {
  const exact = newPivotRow(tb, r, e);
  const cells = exact.map((x, k) => {
    const v = parseFrac(typed[k] ?? '');
    return v === null ? null : closeEnough(v, x);
  });
  return { ok: cells.every((c) => c === true), cells, exact };
}

export type Conclusion = 'optimal' | 'unbounded' | 'infeasible';

/** Comprueba una conclusión declarada en la fase de elegir la columna que entra. */
export function checkDeclaration(tb: PTableau, kind: Conclusion): Verdict {
  const imp = improving(tb);
  const arts = positiveArtificials(tb);
  if (kind === 'optimal') {
    if (imp.length)
      return {
        ok: false,
        level: 'bad',
        msgs: [
          imp.length === 1
            ? t('Todavía no es óptima: {0} tiene un C(j)−Z(j) que mejora el objetivo.', tb.columns[imp[0]])
            : t('Todavía no es óptima: {0} tienen un C(j)−Z(j) que mejora el objetivo.', imp.map((j) => tb.columns[j]).join(', '))
        ]
      };
    if (arts.length)
      return {
        ok: false,
        level: 'bad',
        msgs: [t('Ninguna columna mejora, pero la artificial {0} sigue en la base con valor positivo: el problema no es factible.', tb.columns[tb.basis[arts[0]]])]
      };
    const msgs: Msg[] = [t('Correcto: ningún C(j)−Z(j) mejora el objetivo y no quedan artificiales positivas. Es la tabla óptima.')];
    if (alternateOptima(tb)) msgs.push(t('Hay una no básica con C(j)−Z(j) = 0: existen óptimos alternativos.'));
    return { ok: true, level: 'ok', msgs };
  }
  if (kind === 'infeasible') {
    if (provablyInfeasible(tb))
      return {
        ok: true,
        level: 'ok',
        msgs: [t('Correcto: la parte M ya no puede mejorar y la artificial {0} sigue positiva. El problema no es factible.', tb.columns[tb.basis[arts[0]]])]
      };
    if (!arts.length) return { ok: false, level: 'bad', msgs: [t('No hay artificiales positivas en la base: la solución de esta tabla es factible.')] };
    return { ok: false, level: 'bad', msgs: [t('Todavía hay columnas que reducen la parte M: las artificiales aún pueden salir.')] };
  }
  // no acotado
  const unb = imp.filter((j) => noPositive(tb, j));
  if (unb.length && !arts.length)
    return { ok: true, level: 'ok', msgs: [t('Correcto: {0} mejora el objetivo y ningún elemento de su columna es positivo. El problema no está acotado.', tb.columns[unb[0]])] };
  if (unb.length && provablyInfeasible(tb))
    return { ok: false, level: 'bad', msgs: [t('Ninguna fila limita, pero la artificial {0} sigue positiva y la parte M ya no mejora: el problema no es factible.', tb.columns[tb.basis[arts[0]]])] };
  if (!imp.length) return { ok: false, level: 'bad', msgs: [t('Ninguna columna mejora el objetivo, así que no puede ser no acotado.')] };
  return { ok: false, level: 'bad', msgs: [t('Toda columna que mejora tiene algún elemento positivo: la prueba del cociente sí limita el crecimiento.')] };
}

/** Conclusión correcta de la tabla, si ya se puede declarar (en el orden de la regla de Dantzig). */
export function expectedConclusion(tb: PTableau): Conclusion | null {
  if (provablyInfeasible(tb)) return 'infeasible';
  const imp = improving(tb);
  if (!imp.length) return positiveArtificials(tb).length ? 'infeasible' : 'optimal';
  const d = dantzig(tb);
  if (d.length && noPositive(tb, d[0]) && !positiveArtificials(tb).length) return 'unbounded';
  return null;
}

/** Pistas progresivas para elegir la columna que entra. */
export function hintEntering(tb: PTableau, level: number): Msg {
  if (level === 0)
    return tb.sense === 'max'
      ? t('Mira la fila C(j)−Z(j): al maximizar entra una no básica con valor positivo. Con la M, manda primero el coeficiente de M.')
      : t('Mira la fila C(j)−Z(j): al minimizar entra una no básica con valor negativo. Con la M, manda primero el coeficiente de M.');
  const imp = improving(tb);
  if (!imp.length) return positiveArtificials(tb).length ? t('Ninguna columna mejora y hay una artificial positiva en la base: ¿qué significa eso?') : t('Ninguna columna mejora el objetivo: ¿qué se concluye?');
  if (provablyInfeasible(tb)) return t('La parte M ya no puede mejorar y queda una artificial positiva en la base: ¿qué significa eso?');
  return t('Columnas que mejoran: {0}. La regla de Dantzig toma la de mayor mejora.', imp.map((j) => tb.columns[j]).join(', '));
}

/** Pistas progresivas para elegir la fila que sale. */
export function hintLeaving(tb: PTableau, e: number, level: number): Msg {
  if (level === 0) return t('Divide el lado derecho entre cada elemento positivo de la columna {0}; sale la fila del menor cociente.', tb.columns[e]);
  const q = ratios(tb, e);
  const parts = q.map((x, r) => (x ? `${tb.columns[tb.basis[r]]}: ${tb.rhs[r].toString()} / ${tb.rows[r][e].toString()} = ${x.toString()}` : null)).filter(Boolean);
  if (!parts.length) return t('La columna {0} no tiene elementos positivos: ninguna fila limita.', tb.columns[e]);
  return t('Cocientes: {0}.', parts.join(' · '));
}

export function hintPivotRow(tb: PTableau, r: number, e: number, level: number): Msg {
  const p = tb.rows[r][e];
  if (level === 0) return t('Divide cada elemento de la fila de {0} (y su lado derecho) entre el pivote {1}.', tb.columns[tb.basis[r]], p.toString());
  return t('En la columna {0} debe quedar 1, y en las demás columnas básicas, 0.', tb.columns[e]);
}

// ------------------------------------------------------------ variables del modelo

const finite = (x: number) => Number.isFinite(x);

/** Cotas efectivas de una variable (como el núcleo: binaria = [0,1], libre = (−∞, ∞)). */
export function effBounds(v: Variable): [number, number] {
  const b = (x: Variable['lower'], def: number) => (x === undefined ? def : x === 'M' ? Infinity : x === '-M' ? -Infinity : x);
  const l = b(v.lower, 0);
  const u = b(v.upper, Infinity);
  if (v.type === 'unrestricted') return [-Infinity, Infinity];
  if (v.type === 'binary') return [Math.max(l, 0), Math.min(u, 1)];
  return [l, u];
}

/**
 * Valores de las variables originales a partir de las columnas de la tabla
 * (x = cota inferior + columna, x = cota superior − Neg_x, o x = x⁺ − x⁻ si es libre).
 * Usa las cotas de la tabla inicial (por defecto las del modelo).
 */
export function variableValues(model: LpModel, tb: PTableau, bounds?: [number, number][]): Frac[] {
  const cv = columnValues(tb);
  let k = 0;
  return model.variables.map((v, j) => {
    const [l, u] = bounds?.[j] ?? effBounds(v);
    if (finite(l)) return fracOf(l).add(cv[k++] ?? Frac.ZERO);
    if (finite(u)) return fracOf(u).sub(cv[k++] ?? Frac.ZERO);
    const x = (cv[k] ?? Frac.ZERO).sub(cv[k + 1] ?? Frac.ZERO);
    k += 2;
    return x;
  });
}

/** ¿El modelo cabe en la práctica? (lo que resuelve el símplex didáctico del núcleo) */
export function practicable(model: LpModel): boolean {
  const n = model.variables.length;
  const m = model.constraints.length;
  if (!n || !m) return false;
  if (model.variables.some((v) => v.type === 'semicontinuous' || v.type === 'semiinteger')) return false;
  if ((model as { objectives?: unknown[] }).objectives?.length) return false;
  return n * (n + m) <= 40_000;
}

export const isIntVar = (v: Variable) => v.type === 'integer' || v.type === 'binary';

// ------------------------------------------------------------ branch and bound

export type NodeStatus = 'optimal' | 'infeasible' | 'unbounded' | 'error';
export type Decision = 'branch' | 'prune_infeasible' | 'prune_bound' | 'incumbent';

export interface BNode {
  id: number;
  parent: number | null;
  depth: number;
  /** Rama que lo creó, p. ej. «X1 ≤ 2» (null en la raíz). */
  branch: string | null;
  bounds: [number, number][];
  status: NodeStatus;
  z: number | null;
  values: number[];
  /** Lo que se decidió (cuando ya se procesó). */
  decision?: Decision;
  branchVar?: number;
  /** Orden en que se exploró (1, 2, …). */
  order?: number;
}

export function isIntegral(x: number, tol: number): boolean {
  return Math.abs(x - Math.round(x)) <= tol;
}

/** Variables enteras con valor fraccionario en el nodo. */
export function fractionalVars(model: LpModel, values: number[], tol: number): number[] {
  return model.variables.map((v, j) => (isIntVar(v) && !isIntegral(values[j], tol) ? j : -1)).filter((j) => j >= 0);
}

/** ¿z es estrictamente mejor que la mejor solución entera? */
export function betterThan(sense: Sense, z: number, best: number): boolean {
  const e = 1e-9 * (1 + Math.abs(best));
  return sense === 'max' ? z > best + e : z < best - e;
}

/** La decisión correcta en un nodo, con su explicación. */
export function correctDecision(model: LpModel, node: BNode, incumbent: number | null, tol: number): { decision: Decision; why: Msg } {
  const sense = model.objective;
  if (node.status !== 'optimal' || node.z === null) return { decision: 'prune_infeasible', why: t('El subproblema no tiene solución factible: se poda por no factibilidad.') };
  if (incumbent !== null && !betterThan(sense, node.z, incumbent))
    return {
      decision: 'prune_bound',
      why:
        sense === 'max'
          ? t('z = {0} no supera a la mejor solución entera ({1}): ninguna solución de esta rama puede mejorarla. Se poda por cota.', fmtNum(node.z), fmtNum(incumbent))
          : t('z = {0} no es menor que la mejor solución entera ({1}): ninguna solución de esta rama puede mejorarla. Se poda por cota.', fmtNum(node.z), fmtNum(incumbent))
    };
  const fr = fractionalVars(model, node.values, tol);
  if (!fr.length)
    return {
      decision: 'incumbent',
      why: incumbent === null ? t('Todas las variables enteras toman valores enteros: es la primera solución entera (z = {0}).', fmtNum(node.z)) : t('Todas las variables enteras toman valores enteros y z = {0} mejora a {1}: nueva mejor solución entera.', fmtNum(node.z), fmtNum(incumbent))
    };
  return {
    decision: 'branch',
    why: (() => {
      const names = fr.map((j) => model.variables[j].name).join(', ');
      if (incumbent === null)
        return fr.length === 1 ? t('{0} es fraccionaria y aún no hay solución entera: hay que ramificar.', names) : t('{0} son fraccionarias y aún no hay solución entera: hay que ramificar.', names);
      return fr.length === 1
        ? t('{0} es fraccionaria y z = {1} todavía puede mejorar a {2}: hay que ramificar.', names, fmtNum(node.z), fmtNum(incumbent))
        : t('{0} son fraccionarias y z = {1} todavía puede mejorar a {2}: hay que ramificar.', names, fmtNum(node.z), fmtNum(incumbent));
    })()
  };
}

/** Comprueba la decisión del estudiante en un nodo. */
export function checkDecision(model: LpModel, node: BNode, incumbent: number | null, tol: number, d: Decision): Verdict {
  const right = correctDecision(model, node, incumbent, tol);
  if (right.decision === d) return { ok: true, level: 'ok', msgs: [t('Correcto.'), right.why] };
  const msgs: Msg[] = [];
  const fr = fractionalVars(model, node.values, tol);
  if (d === 'prune_infeasible') msgs.push(t('El subproblema sí tiene solución (z = {0}): no es no factible.', fmtNum(node.z)));
  else if (node.status !== 'optimal') msgs.push(t('El subproblema no tiene solución: no hay z ni valores que comparar.'));
  else if (d === 'prune_bound' && incumbent === null) msgs.push(t('Aún no hay ninguna solución entera con la que comparar: no se puede podar por cota.'));
  else if (d === 'prune_bound') msgs.push(t('z = {0} es mejor que la mejor solución entera ({1}): esta rama todavía puede mejorarla.', fmtNum(node.z), fmtNum(incumbent)));
  else if (d === 'incumbent' && fr.length) msgs.push(
      fr.length === 1
        ? t('{0} no es entera: esta solución no sirve para el problema entero.', `${model.variables[fr[0]].name} = ${fmtNum(node.values[fr[0]])}`)
        : t('{0} no son enteras: esta solución no sirve para el problema entero.', fr.map((j) => `${model.variables[j].name} = ${fmtNum(node.values[j])}`).join(', '))
    );
  else if (d === 'incumbent') msgs.push(t('La solución es entera, pero no mejora a la mejor solución entera ({0}).', fmtNum(incumbent)));
  else if (d === 'branch' && right.decision === 'incumbent') msgs.push(t('Todas las variables enteras ya son enteras: no hay sobre qué ramificar.'));
  else if (d === 'branch') msgs.push(t('No hace falta ramificar: este nodo se puede podar.'));
  msgs.push(right.why);
  return { ok: false, level: 'bad', msgs };
}

/** Comprueba la variable elegida para ramificar. */
export function checkBranchVar(model: LpModel, node: BNode, j: number, tol: number): Verdict {
  const v = model.variables[j];
  const fr = fractionalVars(model, node.values, tol);
  if (!isIntVar(v)) return { ok: false, level: 'bad', msgs: [t('{0} es continua: puede tomar valores fraccionarios, no se ramifica sobre ella.', v.name)] };
  if (!fr.includes(j)) return { ok: false, level: 'bad', msgs: [t('{0} = {1} ya es entera: ramificar sobre ella no corta la solución actual.', v.name, fmtNum(node.values[j]))] };
  const msgs: Msg[] = [t('Correcto: {0} = {1} es fraccionaria.', v.name, fmtNum(node.values[j]))];
  if (fr.length > 1 && fr[0] !== j) msgs.push(t('Cualquier variable fraccionaria sirve; WinQSB toma la primera ({0}).', model.variables[fr[0]].name));
  return { ok: true, level: 'ok', msgs };
}

/** Cotas de los dos hijos al ramificar sobre j: x ≤ ⌊v⌋ y x ≥ ⌈v⌉. */
export function childBounds(bounds: [number, number][], j: number, value: number): { down: [number, number][]; up: [number, number][]; fl: number; ce: number } {
  const fl = Math.floor(value);
  const ce = fl + 1;
  const down = bounds.map((b) => [...b] as [number, number]);
  const up = bounds.map((b) => [...b] as [number, number]);
  down[j] = [bounds[j][0], Math.min(bounds[j][1], fl)];
  up[j] = [Math.max(bounds[j][0], ce), bounds[j][1]];
  return { down, up, fl, ce };
}

/** Copia del modelo con las cotas del nodo (para resolver su relajación lineal). */
export function modelWithBounds(model: LpModel, bounds: [number, number][]): LpModel {
  const b = (x: number) => (x === Infinity ? 'M' : x === -Infinity ? '-M' : x);
  return {
    ...model,
    variables: model.variables.map((v, j) => {
      const [l, u] = bounds[j];
      // una variable libre con cota ya no es libre
      const type = v.type === 'unrestricted' && (finite(l) || finite(u)) ? 'continuous' : v.type;
      return { ...v, type, lower: b(l), upper: b(u) };
    }),
    constraints: model.constraints.map((c) => ({ ...c, coefs: [...c.coefs] }))
  };
}

/** Posición de cada nodo del árbol: las hojas en columnas consecutivas, cada padre centrado sobre sus hijos. */
export function layoutTree(nodes: BNode[]): Map<number, { x: number; y: number }> {
  const kids = new Map<number, BNode[]>();
  for (const n of nodes) if (n.parent !== null) kids.set(n.parent, [...(kids.get(n.parent) ?? []), n]);
  const pos = new Map<number, { x: number; y: number }>();
  let next = 0;
  const walk = (n: BNode): number => {
    const ch = kids.get(n.id) ?? [];
    let x: number;
    if (!ch.length) x = next++;
    else {
      const xs = ch.map(walk);
      x = (xs[0] + xs[xs.length - 1]) / 2;
    }
    pos.set(n.id, { x, y: n.depth });
    return x;
  };
  for (const r of nodes.filter((n) => n.parent === null)) walk(r);
  return pos;
}

/** Número corto para los mensajes (hasta 4 decimales). */
export function fmtNum(x: number | null | undefined): string {
  if (x === null || x === undefined || !Number.isFinite(x)) return '–';
  let s = (Math.abs(x) < 1e-10 ? 0 : x).toFixed(4);
  if (s.includes('.')) s = s.replace(/0+$/, '').replace(/\.$/, '');
  if (s === '-0') s = '0';
  return s.replace('-', '−');
}

/** ¿Coinciden dos valores del objetivo? */
export function sameObjective(a: number | null | undefined, b: number | null | undefined): boolean {
  if (a === null || a === undefined || b === null || b === undefined) return false;
  return Math.abs(a - b) <= 1e-6 * (1 + Math.abs(b));
}
