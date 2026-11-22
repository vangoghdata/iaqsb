// Pegar tablas copiadas de Excel, Google Sheets o LibreOffice en las hojas del editor.
//
// Lógica pura (sin Svelte, se prueba con tools/test_pegar.ts): leer el texto del
// portapapeles, entender los números (coma o punto decimal, miles, porcentajes,
// M y -M) y calcular el modelo que queda después de pegar. El pegado se ancla en
// la celda enfocada y llena hacia la derecha y hacia abajo; una fila o columna
// de nombres al borde del bloque se reconoce y se usa para renombrar.
//
// Coordenadas lógicas de cada hoja (sin los huecos de los botones «+ …»):
//   LP-ILP:  filas 0 nombres · 1 objetivo · 2..m+1 restricciones · m+2 cota inf. · m+3 cota sup. · m+4 tipo
//            columnas 0 nombres · 1..n variables · n+1 sentido · n+2 lado derecho
//   Matriz:  fila -1 nombres de columnas · 0..R-1 filas · R.. datos extra de las columnas
//            columna -1 nombres de filas · 0..C-1 columnas · C.. datos extra de las filas
//   Registros: fila = registro, columna = columna visible

import type { Bound, Direction, LpModel, VarType } from './types';
import type { Field, Widget } from './gtypes';

export type Tabla = string[][];
export type Celda = [number, number];

// ------------------------------------------------------------- lectura del texto

/** Separa el texto en celdas respetando «"…"» (celdas con saltos de línea o comillas). */
function partir(s: string, sep: string): Tabla {
  const filas: Tabla = [];
  let fila: string[] = [];
  let celda = '';
  let comillas = false;
  let inicio = true;
  for (let i = 0; i < s.length; i++) {
    const ch = s[i];
    if (comillas) {
      if (ch === '"') {
        if (s[i + 1] === '"') {
          celda += '"';
          i++;
        } else comillas = false;
      } else celda += ch;
      continue;
    }
    if (ch === '"' && inicio) {
      comillas = true;
      inicio = false;
    } else if (ch === sep) {
      fila.push(celda);
      celda = '';
      inicio = true;
    } else if (ch === '\n') {
      fila.push(celda);
      filas.push(fila);
      fila = [];
      celda = '';
      inicio = true;
    } else {
      celda += ch;
      inicio = false;
    }
  }
  if (celda !== '' || fila.length) {
    fila.push(celda);
    filas.push(fila);
  }
  return filas;
}

/**
 * El texto del portapapeles como filas de celdas. Excel, Sheets y LibreOffice
 * separan con tabuladores; también se acepta «;» (CSV en español) y, si no hay
 * separador, números separados por espacios.
 */
export function leerTabla(texto: string): Tabla {
  const s = texto.replace(/^﻿/, '').replace(/\r\n?/g, '\n');
  let filas: Tabla;
  if (s.includes('\t')) filas = partir(s, '\t');
  else if (s.includes(';')) filas = partir(s, ';');
  else
    filas = s.split('\n').map((l) => {
      const toks = l.trim().split(/\s+/).filter(Boolean);
      return toks.length > 1 && toks.every((x) => pareceNumero(x) || sentido(x) !== null || cota(x, false) !== null) ? toks : [l];
    });
  // Excel termina con un salto de línea: fuera las filas vacías del final y las columnas vacías a la derecha
  while (filas.length && filas[filas.length - 1].every((c) => c.trim() === '')) filas.pop();
  let ancho = 0;
  for (const f of filas) {
    let k = f.length;
    while (k > 0 && f[k - 1].trim() === '') k--;
    ancho = Math.max(ancho, k);
  }
  return filas.map((f) => Array.from({ length: ancho }, (_, j) => (f[j] ?? '').trim()));
}

// ------------------------------------------------------------- números

const MONEDA = /R\$|US\$|S\/|[$€£¥]/g;

/** Quita espacios de miles, moneda, signo «−» y paréntesis contables. */
function limpiar(s: string): { t: string; neg: boolean; pct: boolean } | null {
  let t = s.trim().replace(/[\s   ']/g, '').replace(/[−‒–]/g, '-').replace(MONEDA, '');
  let neg = false;
  if (/^\(.+\)$/.test(t)) {
    neg = true;
    t = t.slice(1, -1);
  }
  let pct = false;
  if (t.endsWith('%')) {
    pct = true;
    t = t.slice(0, -1);
  }
  if (!/^[+-]?[\d.,]*\d[\d.,]*([eE][+-]?\d+)?$/.test(t) || /^[+-]?[\d.,]*[.,]{2}/.test(t)) return null;
  return { t, neg, pct };
}

function cuenta(s: string, ch: string): number {
  return s.split(ch).length - 1;
}

/** ¿El número es ambiguo (1.234 o 1,234: miles o decimales)? */
const AMBIGUO = /^[+-]?\d{1,3}[.,]\d{3}$/;

/**
 * Lee un número escrito en Excel: «3,5», «3.5», «1.234,5», «1,234.5», «15 %»,
 * «−2», «(4)», «$ 1.200», «1,5E-3». `coma` dice qué hacer con los casos ambiguos
 * («1.234»): con coma decimal es mil doscientos treinta y cuatro.
 */
export function numero(s: string, coma: boolean): number | null {
  const l = limpiar(s);
  if (!l) return null;
  const k = l.t.search(/[eE]/);
  let man = k < 0 ? l.t : l.t.slice(0, k);
  const exp = k < 0 ? '' : l.t.slice(k);
  const puntos = cuenta(man, '.');
  const comas = cuenta(man, ',');
  if (puntos && comas) {
    // el último separador es el decimal
    const dec = man.lastIndexOf(',') > man.lastIndexOf('.') ? ',' : '.';
    if (cuenta(man, dec) > 1) return null;
    man = man.split(dec === ',' ? '.' : ',').join('').replace(',', '.');
  } else if (puntos + comas > 1) {
    // 1.234.567 o 1,234,567: solo miles
    man = man.replace(/[.,]/g, '');
  } else if (puntos + comas === 1) {
    const sep = puntos ? '.' : ',';
    const miles = AMBIGUO.test(man) && (sep === '.' ? coma : !coma);
    man = miles ? man.replace(sep, '') : man.replace(',', '.');
  }
  let v = Number(man + exp);
  if (!Number.isFinite(v)) return null;
  if (l.neg) v = -v;
  if (l.pct) v = Number((v / 100).toPrecision(15));
  return v;
}

export function pareceNumero(s: string): boolean {
  return limpiar(s) !== null;
}

/**
 * ¿El bloque usa coma decimal? Se decide por los números que no son ambiguos
 * («3,5» vota coma; «0.25» vota punto; «1.234,5» vota coma). Si ninguno
 * decide, se usa la preferencia (la configuración regional del equipo).
 */
export function detectarComa(tabla: Tabla, preferida: boolean): boolean {
  let coma = 0;
  let punto = 0;
  for (const f of tabla)
    for (const c of f) {
      const l = limpiar(c);
      if (!l) continue;
      const man = l.t.replace(/[eE].*$/, '');
      const p = cuenta(man, '.');
      const k = cuenta(man, ',');
      if (p && k) man.lastIndexOf(',') > man.lastIndexOf('.') ? coma++ : punto++;
      else if (p > 1) coma++;
      else if (k > 1) punto++;
      else if (p + k === 1 && !AMBIGUO.test(man)) k ? coma++ : punto++;
    }
  return coma > punto ? true : punto > coma ? false : preferida;
}

// ------------------------------------------------------------- otros valores

/** Texto sin tildes, en minúsculas y sin espacios de sobra (para comparar etiquetas). */
export function norm(s: string): string {
  return s
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[:.]+$/, '')
    .replace(/\s+/g, ' ')
    .trim();
}

export function sentido(s: string): Direction | null {
  const t = s.replace(/\s/g, '');
  if (['<=', '=<', '≤', '<', '⩽'].includes(t)) return '<=';
  if (['>=', '=>', '≥', '>', '⩾'].includes(t)) return '>=';
  if (['=', '=='].includes(t)) return '=';
  return null;
}

/** Cota de una variable: número, M (infinito) o -M. */
export function cota(s: string, coma: boolean): Bound | null {
  const u = s.trim().toUpperCase().replace(/\s/g, '').replace(/[−–]/g, '-');
  if (['M', '+M', 'INF', '+INF', '∞', '+∞', 'INFINITO', 'INFINITY', '1E+30'].includes(u)) return 'M';
  if (['-M', '-INF', '-∞', '-INFINITO', '-INFINITY', '-1E+30'].includes(u)) return '-M';
  return numero(s, coma);
}

/** Tipo de variable en español, inglés o portugués («Entera», «int», «Binária»…). */
export function tipoVariable(s: string): VarType | null {
  const t = norm(s);
  if (!t) return null;
  if (/^semi.?cont/.test(t)) return 'semicontinuous';
  if (/^semi.?(ent|int)/.test(t)) return 'semiinteger';
  if (/^(cont|real|c$)/.test(t)) return 'continuous';
  if (/^(ent|int|inteir|i$|e$)/.test(t)) return 'integer';
  if (/^(bin|b$|0\/1|0-1)/.test(t)) return 'binary';
  if (/^(lib|unres|free|livre|irrest|u$|l$)/.test(t)) return 'unrestricted';
  return null;
}

function objetivo(s: string): 'max' | 'min' | null {
  const t = norm(s);
  if (/^max/.test(t)) return 'max';
  if (/^min/.test(t)) return 'min';
  return null;
}

/** ¿Es un rótulo (texto que no es número, sentido ni M)? */
function rotulo(s: string): boolean {
  return s.trim() !== '' && !pareceNumero(s) && sentido(s) === null && cota(s, false) === null;
}

/** ¿La mayoría de las celdas no vacías son rótulos (y hay al menos uno)? */
function mayoriaRotulos(celdas: string[]): boolean {
  const llenas = celdas.filter((c) => c.trim() !== '');
  const r = llenas.filter(rotulo).length;
  return r > 0 && r * 2 >= llenas.length;
}

function hayNumeros(tabla: Tabla, desde: number): boolean {
  return tabla.slice(desde).some((f) => f.some((c) => pareceNumero(c)));
}

/** Nombre nuevo que no choca con los existentes (X3, X3', Origen4…). */
export function nombreLibre(existentes: string[], prefijo: string, n: number): string {
  const usados = new Set(existentes.map((s) => s.toUpperCase()));
  for (let i = n; i < n + 100000; i++) {
    const s = `${prefijo}${i}`;
    if (!usados.has(s.toUpperCase())) return s;
  }
  return `${prefijo}${Date.now()}`;
}

/**
 * Valor de una celda de un módulo genérico según el tipo del campo.
 * Devuelve `{ v }`, `null` si no se pudo leer, o `undefined` si la celda no cambia.
 */
export function valorCampo(f: Field, s: string, coma: boolean): { v: unknown } | null | undefined {
  const k = f.kind.type;
  const vacia = s.trim() === '';
  if (k === 'number' || k === 'integer' || k === 'bound') {
    if (vacia) return { v: f.optional ? null : 0 };
    if (k === 'bound') {
      const b = cota(s, coma);
      return b === null ? null : { v: b };
    }
    const x = numero(s, coma);
    if (x === null) return null;
    if (k === 'integer') {
      const r = Math.round(x);
      return Math.abs(r - x) < 1e-9 ? { v: r } : null;
    }
    return { v: x };
  }
  if (k === 'bool') {
    if (vacia) return { v: false };
    const t = norm(s);
    if (['1', 'true', 'verdadero', 'si', 'sí', 'yes', 'sim', 'x', '✓', '✔', 'v'].includes(t)) return { v: true };
    if (['0', 'false', 'falso', 'no', 'nao', 'não', 'f', '-'].includes(t)) return { v: false };
    return null;
  }
  if (k === 'choice') {
    if (vacia) return undefined;
    const t = norm(s);
    const o = f.kind.options.find(([v, l]) => norm(v) === t || norm(l) === t) ?? f.kind.options.find(([, l]) => norm(l).startsWith(t));
    return o ? { v: o[0] } : null;
  }
  if (k === 'list')
    return {
      v: s
        .split(/[,;]/)
        .map((x) => x.trim())
        .filter(Boolean)
    };
  return { v: s };
}

// ------------------------------------------------------------- resultado común

export interface Resultado<M> {
  modelo: M;
  /** Celdas escritas, en coordenadas lógicas de la hoja (para resaltarlas). */
  escritas: Celda[];
  /** Celdas que no se pudieron leer (quedaron como estaban). */
  invalidas: number;
  /** Celdas con datos que quedaron fuera de la hoja. */
  fuera: number;
  renombradas: number;
  nuevasFilas: number;
  nuevasColumnas: number;
}

/** Lo que falta para que el bloque quepa (se le ofrece a la persona). */
export interface Falta {
  filas: number;
  columnas: number;
  /** LP-ILP: sin columna de sentido, la última columna puede ser el lado derecho. */
  ultimaLD: boolean;
}

export interface Eleccion {
  crecer: boolean;
  ultimaLD?: boolean;
}

function copia<T>(x: T): T {
  return JSON.parse(JSON.stringify(x));
}

// ------------------------------------------------------------- LP-ILP

const LD = /^(ld|rhs|b|lado derecho|lado der|right.?hand|disponib|capacidad|limite|lim|recurso|requer|valor)/;
const RE_INF = /^(cota inf|lower|lim(ite)? inf|inferior|li|lb)$/;
const RE_INF2 = /^(cota inf|lower|lim(ite)? inf)/;
const RE_SUP = /^(cota sup|upper|lim(ite)? sup|superior|ls|ub)$/;
const RE_SUP2 = /^(cota sup|upper|lim(ite)? sup)/;
const RE_TIPO = /^(tipo|type|clase)/;

type DestinoLP = { k: number; fila: 'obj' | 'res' | 'inf' | 'sup' | 'tipo' | 'fuera'; i: number; rotulo: string };

export interface AnalisisLP {
  tabla: Tabla;
  coma: boolean;
  /** Primera fila del bloque = nombres de las variables. */
  encabezado: boolean;
  /** Primera columna del bloque = nombres de las filas. */
  nombres: boolean;
  /** Columna lógica donde empiezan los datos (1..n variables, n+1 sentido, n+2 lado derecho). */
  c0: number;
  /** Columnas de datos del bloque (sin la de nombres). */
  ancho: number;
  colsVar: number;
  colSentido: number | null;
  colLD: number | null;
  filas: DestinoLP[];
  falta: Falta;
}

/** Entiende el bloque pegado en la hoja de LP-ILP anclado en la celda lógica (r, c). */
export function analizarLP(m: LpModel, r: number, c: number, tabla: Tabla, coma: boolean): AnalisisLP {
  const n = m.variables.length;
  const nr = m.constraints.length;
  const W = tabla[0]?.length ?? 0;
  // ¿encabezado con nombres de variables?
  let encabezado = r === 0;
  if (!encabezado && r >= 1 && tabla.length >= 2) {
    const fila0 = W === 1 ? (c >= 1 && c <= n ? tabla[0] : []) : tabla[0].slice(1);
    encabezado = mayoriaRotulos(fila0) && hayNumeros(tabla, 1);
  }
  const d0 = encabezado ? 1 : 0;
  // ¿columna de nombres de las filas?
  let nombres = c === 0;
  if (!nombres && W >= 2 && c >= 1) nombres = mayoriaRotulos(tabla.slice(d0).map((f) => f[0])) && tabla.slice(d0).some((f) => f.slice(1).some(pareceNumero));
  const j0 = nombres ? 1 : 0;
  const c0 = c === 0 ? 1 : c;
  const ancho = W - j0;
  const datos = tabla.slice(d0);
  const col = (j: number) => datos.map((f) => f[j0 + j] ?? '');

  let colsVar = 0;
  let colSentido: number | null = null;
  let colLD: number | null = null;
  let ultimaLD = false;
  if (c0 <= n) {
    for (let j = 0; j < ancho; j++) {
      const llenas = col(j).filter((x) => x.trim() !== '');
      if (llenas.length && llenas.every((x) => sentido(x) !== null)) {
        colSentido = j;
        break;
      }
    }
    if (colSentido !== null) {
      colsVar = colSentido;
      colLD = colSentido + 1 < ancho ? colSentido + 1 : null;
    } else {
      const k = encabezado ? tabla[0].slice(j0).findIndex((h, j) => j > 0 && LD.test(norm(h))) : -1;
      if (k > 0) {
        colsVar = k;
        colLD = k;
      } else {
        colsVar = ancho;
        // si se pasa de las variables, la última columna puede ser el lado derecho
        ultimaLD = c0 + ancho - 1 > n && ancho >= 2;
      }
    }
  } else if (c0 === n + 1) {
    // empieza en el sentido: si esa columna trae números, es el lado derecho
    if (col(0).some((x) => sentido(x) !== null)) {
      colSentido = 0;
      colLD = ancho > 1 ? 1 : null;
    } else colLD = 0;
  } else colLD = 0;

  // filas del bloque
  const filas: DestinoLP[] = [];
  const r0 = r === 0 ? 1 : r;
  let paso = 0;
  let maxRes = -1;
  datos.forEach((f, k) => {
    const rot = nombres ? f[0] : '';
    const t = norm(rot);
    let fila: DestinoLP['fila'] | null = null;
    if (t && (RE_INF.test(t) || RE_INF2.test(t))) fila = 'inf';
    else if (t && (RE_SUP.test(t) || RE_SUP2.test(t))) fila = 'sup';
    else if (t && RE_TIPO.test(t)) fila = 'tipo';
    if (fila) {
      filas.push({ k: k + d0, fila, i: -1, rotulo: rot });
      return;
    }
    const L = r0 + paso++;
    if (r0 >= nr + 2) {
      const b = L - (nr + 2);
      filas.push({ k: k + d0, fila: b === 0 ? 'inf' : b === 1 ? 'sup' : b === 2 ? 'tipo' : 'fuera', i: -1, rotulo: rot });
    } else if (L === 1) filas.push({ k: k + d0, fila: 'obj', i: -1, rotulo: rot });
    else {
      filas.push({ k: k + d0, fila: 'res', i: L - 2, rotulo: rot });
      maxRes = Math.max(maxRes, L - 2);
    }
  });
  if (!filas.some((x) => x.fila === 'res')) ultimaLD = false;
  return {
    tabla,
    coma,
    encabezado,
    nombres,
    c0,
    ancho,
    colsVar,
    colSentido,
    colLD,
    filas,
    falta: {
      filas: Math.max(0, maxRes + 1 - nr),
      columnas: c0 <= n ? Math.max(0, c0 - 1 + colsVar - n) : 0,
      ultimaLD
    }
  };
}

/** Aplica el bloque analizado a una copia del modelo. */
export function pegarLP(original: LpModel, a: AnalisisLP, el: Eleccion): Resultado<LpModel> {
  const m = copia(original);
  let { colsVar, colLD } = a;
  if (el.ultimaLD && a.colSentido === null && colLD === null && colsVar >= 2) {
    colsVar -= 1;
    colLD = colsVar;
  }
  const n0 = m.variables.length;
  const res: Resultado<LpModel> = { modelo: m, escritas: [], invalidas: 0, fuera: 0, renombradas: 0, nuevasFilas: 0, nuevasColumnas: 0 };
  const j0 = a.nombres ? 1 : 0;
  const cab = a.encabezado ? a.tabla[0] : null;

  // variables nuevas
  if (el.crecer && a.c0 <= n0) {
    const faltan = Math.max(0, a.c0 - 1 + colsVar - n0);
    for (let q = 0; q < faltan; q++) {
      const j = n0 + q;
      const h = cab?.[j0 + (j - (a.c0 - 1))]?.trim();
      const name = h || nombreLibre(m.variables.map((v) => v.name), 'X', m.variables.length + 1);
      m.variables.push({ name, cost: 0, lower: 0, upper: 'M', type: 'continuous' });
      for (const c of m.constraints) c.coefs.push(0);
      res.nuevasColumnas++;
    }
  }
  const n = m.variables.length;
  // restricciones nuevas
  if (el.crecer) {
    const maxRes = Math.max(-1, ...a.filas.filter((x) => x.fila === 'res').map((x) => x.i));
    while (m.constraints.length <= maxRes) {
      const f = a.filas.find((x) => x.fila === 'res' && x.i === m.constraints.length);
      const name = f?.rotulo.trim() || nombreLibre(m.constraints.map((c) => c.name), 'C', m.constraints.length + 1);
      m.constraints.push({ name, coefs: Array(n).fill(0), direction: '<=', rhs: 0 });
      res.nuevasFilas++;
    }
  }
  const nr = m.constraints.length;
  // fila lógica de cada destino (después de crecer) para resaltar
  const filaLogica = (d: DestinoLP) => (d.fila === 'obj' ? 1 : d.fila === 'res' ? d.i + 2 : d.fila === 'inf' ? nr + 2 : d.fila === 'sup' ? nr + 3 : nr + 4);

  // nombres de las variables (encabezado)
  if (cab && a.c0 <= n0 + res.nuevasColumnas) {
    for (let j = 0; j < colsVar; j++) {
      const v = a.c0 - 1 + j;
      const h = cab[j0 + j]?.trim();
      if (!h) continue;
      if (v >= n) {
        res.fuera++;
        continue;
      }
      if (m.variables[v].name !== h) {
        m.variables[v].name = h;
        res.renombradas++;
      }
      res.escritas.push([0, v + 1]);
    }
  }

  for (const d of a.filas) {
    const f = a.tabla[d.k];
    if (d.fila === 'fuera' || (d.fila === 'res' && d.i >= nr)) {
      res.fuera += f.slice(j0).filter((x) => x.trim() !== '').length;
      continue;
    }
    const L = filaLogica(d);
    // rótulo de la fila
    if (a.nombres && d.rotulo.trim()) {
      if (d.fila === 'res') {
        const c = m.constraints[d.i];
        if (c.name !== d.rotulo.trim()) {
          c.name = d.rotulo.trim();
          res.renombradas++;
        }
        res.escritas.push([L, 0]);
      } else if (d.fila === 'obj') {
        const o = objetivo(d.rotulo);
        if (o) m.objective = o;
      }
    }
    for (let j = 0; j < a.ancho; j++) {
      const s = f[j0 + j] ?? '';
      const vacia = s.trim() === '';
      if (j < colsVar && a.c0 <= n0 + res.nuevasColumnas) {
        const v = a.c0 - 1 + j;
        if (v >= n) {
          if (!vacia) res.fuera++;
          continue;
        }
        const x = m.variables[v];
        if (d.fila === 'obj' || d.fila === 'res') {
          const y = vacia ? 0 : numero(s, a.coma);
          if (y === null) {
            res.invalidas++;
            continue;
          }
          if (d.fila === 'obj') x.cost = y;
          else m.constraints[d.i].coefs[v] = y;
        } else if (d.fila === 'inf' || d.fila === 'sup') {
          if (vacia) continue;
          const b = cota(s, a.coma);
          if (b === null) {
            res.invalidas++;
            continue;
          }
          x[d.fila === 'inf' ? 'lower' : 'upper'] = b;
        } else if (d.fila === 'tipo') {
          if (vacia) continue;
          const t = tipoVariable(s);
          if (!t) {
            res.invalidas++;
            continue;
          }
          x.type = t;
          if (t === 'binary') {
            x.lower = 0;
            x.upper = 1;
          } else if (t === 'unrestricted') {
            x.lower = '-M';
            x.upper = 'M';
          } else if (x.lower === '-M' && x.upper === 'M') x.lower = 0;
        }
        res.escritas.push([L, v + 1]);
      } else if (j === a.colSentido) {
        if (d.fila !== 'res' || vacia) continue;
        const dir = sentido(s);
        if (!dir) {
          res.invalidas++;
          continue;
        }
        m.constraints[d.i].direction = dir;
        res.escritas.push([L, n + 1]);
      } else if (j === colLD) {
        if (d.fila !== 'res') {
          if (!vacia) res.fuera++;
          continue;
        }
        const y = vacia ? 0 : numero(s, a.coma);
        if (y === null) {
          res.invalidas++;
          continue;
        }
        m.constraints[d.i].rhs = y;
        res.escritas.push([L, n + 2]);
      } else if (!vacia) res.fuera++;
    }
  }
  return res;
}

// ------------------------------------------------------------- matrices de los módulos genéricos

export type MatrixW = Extract<Widget, { type: 'matrix' }>;
export type RecordsW = Extract<Widget, { type: 'records' }>;

function numerico(f: Field): boolean {
  return ['number', 'integer', 'bound', 'bool'].includes(f.kind.type);
}

function defaultsDe(fields: Field[]): Record<string, unknown> {
  const o: Record<string, unknown> = {};
  for (const f of fields) o[f.key] = f.default ?? null;
  return o;
}

function coincide(h: string, f: Field): boolean {
  const t = norm(h);
  if (!t) return false;
  const l = norm(f.label);
  return t === l || t === norm(f.key) || (t.length >= 3 && l.startsWith(t)) || (l.length >= 3 && t.startsWith(l));
}

type Rol = { tipo: 'mat'; j: number } | { tipo: 'extra'; k: number } | { tipo: 'fuera' };

export interface AnalisisMatriz {
  tabla: Tabla;
  coma: boolean;
  encabezado: boolean;
  nombres: boolean;
  /** Rol de cada columna de datos y de cada fila de datos (índices del bloque). */
  cols: { j: number; rol: Rol; nombre: string }[];
  filas: { k: number; rol: Rol; nombre: string }[];
  matCols: number;
  matFilas: number;
  falta: Falta;
  r0: number;
  c0: number;
}

/** Entiende el bloque pegado en una matriz anclado en la celda lógica (r, c). */
export function analizarMatriz(w: MatrixW, model: any, r: number, c: number, tabla: Tabla, coma: boolean, puedeFilas: boolean, puedeCols: boolean): AnalisisMatriz {
  const R = (model[w.rows.path] ?? []).length;
  const C = (model[w.cols.path] ?? []).length;
  const exF = w.rows.extra; // columnas extra (un dato por fila)
  const exC = w.cols.extra; // filas extra (un dato por columna)
  const W = tabla[0]?.length ?? 0;
  const esNum = numerico(w.cell);
  let encabezado = r === -1;
  if (!encabezado && esNum && tabla.length >= 2) encabezado = mayoriaRotulos(W === 1 ? tabla[0] : tabla[0].slice(1)) && hayNumeros(tabla, 1);
  const d0 = encabezado ? 1 : 0;
  let nombres = c === -1;
  if (!nombres && esNum && W >= 2) nombres = mayoriaRotulos(tabla.slice(d0).map((f) => f[0])) && tabla.slice(d0).some((f) => f.slice(1).some(pareceNumero));
  const j0 = nombres ? 1 : 0;
  const r0 = Math.max(0, r);
  const c0 = Math.max(0, c);
  const square = w.rows.path === w.cols.path;

  // columnas: por rótulo (si hay encabezado) o por posición
  const cols: AnalisisMatriz['cols'] = [];
  const nc = W - j0;
  let matCols = 0;
  if (encabezado && c0 < C) {
    let jm = c0;
    for (let j = 0; j < nc; j++) {
      const h = tabla[0][j0 + j];
      const k = exF.findIndex((f) => coincide(h, f));
      if (k >= 0) cols.push({ j: j0 + j, rol: { tipo: 'extra', k }, nombre: h });
      else {
        cols.push({ j: j0 + j, rol: { tipo: 'mat', j: jm++ }, nombre: h });
        matCols = jm;
      }
    }
  } else {
    // si el bloque se pasa, las columnas nuevas van a la matriz y las últimas siguen siendo los datos extra
    const sobra = c0 < C ? Math.max(0, c0 + nc - (C + exF.length)) : 0;
    const Cn = C + sobra;
    for (let j = 0; j < nc; j++) {
      const L = c0 + j;
      const rol: Rol = c0 >= C ? (L - C < exF.length ? { tipo: 'extra', k: L - C } : { tipo: 'fuera' }) : L < Cn ? { tipo: 'mat', j: L } : L - Cn < exF.length ? { tipo: 'extra', k: L - Cn } : { tipo: 'fuera' };
      cols.push({ j: j0 + j, rol, nombre: encabezado ? tabla[0][j0 + j] : '' });
      if (rol.tipo === 'mat') matCols = Math.max(matCols, rol.j + 1);
    }
  }

  // filas: por rótulo (si hay nombres) o por posición
  const filas: AnalisisMatriz['filas'] = [];
  const nf = tabla.length - d0;
  let matFilas = 0;
  if (nombres && r0 < R && exC.length) {
    let im = r0;
    for (let k = 0; k < nf; k++) {
      const h = tabla[d0 + k][0];
      const q = exC.findIndex((f) => coincide(h, f));
      if (q >= 0) filas.push({ k: d0 + k, rol: { tipo: 'extra', k: q }, nombre: h });
      else {
        filas.push({ k: d0 + k, rol: { tipo: 'mat', j: im++ }, nombre: h });
        matFilas = im;
      }
    }
  } else {
    let sobra = r0 < R ? Math.max(0, r0 + nf - (R + exC.length)) : 0;
    // una fila que caería en los datos de las columnas (Demanda) pero trae un dato
    // en una columna extra (Oferta) es una fila más de la matriz
    const colsExtra = cols.filter((x) => x.rol.tipo === 'extra').map((x) => x.j);
    if (r0 < R && colsExtra.length)
      for (let k = nf - 1; k >= 0; k--) {
        const L = r0 + k;
        if (L >= R + sobra && colsExtra.some((j) => (tabla[d0 + k][j] ?? '').trim() !== '')) {
          sobra = L - R + 1;
          break;
        }
      }
    const Rn = R + sobra;
    for (let k = 0; k < nf; k++) {
      const L = r0 + k;
      const rol: Rol = r0 >= R ? (L - R < exC.length ? { tipo: 'extra', k: L - R } : { tipo: 'fuera' }) : L < Rn ? { tipo: 'mat', j: L } : L - Rn < exC.length ? { tipo: 'extra', k: L - Rn } : { tipo: 'fuera' };
      filas.push({ k: d0 + k, rol, nombre: nombres ? tabla[d0 + k][0] : '' });
      if (rol.tipo === 'mat') matFilas = Math.max(matFilas, rol.j + 1);
    }
  }
  let faltaF = Math.max(0, matFilas - R);
  let faltaC = Math.max(0, matCols - C);
  if (square) faltaF = faltaC = Math.max(faltaF, faltaC);
  // una matriz cuadrada crece por su lista (filas y columnas a la vez)
  if (!puedeFilas) {
    faltaF = 0;
    if (square) faltaC = 0;
  }
  if (!puedeCols && !square) faltaC = 0;
  return { tabla, coma, encabezado, nombres, cols, filas, matCols, matFilas, falta: { filas: faltaF, columnas: faltaC, ultimaLD: false }, r0, c0 };
}

/**
 * Aplica el bloque a una copia de las listas y matrices del modelo. Al crecer,
 * se ajustan todas las matrices que usan la lista (como en MatrixGrid).
 * Devuelve las claves del modelo que cambiaron.
 */
export function pegarMatriz(
  original: any,
  w: MatrixW,
  hermanas: MatrixW[],
  a: AnalisisMatriz,
  el: Eleccion,
  rotulos: { filas: string; cols: string }
): Resultado<any> & { claves: string[] } {
  const all = hermanas.length ? hermanas : [w];
  const claves = [...new Set([w.rows.path, w.cols.path, ...all.map((s) => s.path)])];
  const model: any = {};
  for (const k of claves) model[k] = copia(original[k] ?? []);
  const res: Resultado<any> & { claves: string[] } = { modelo: model, escritas: [], invalidas: 0, fuera: 0, renombradas: 0, nuevasFilas: 0, nuevasColumnas: 0, claves };
  const square = w.rows.path === w.cols.path;

  const blank = (s: MatrixW) => (s.cell.optional ? null : (s.cell.default ?? 0));
  const crecer = (path: string) => {
    for (const s of all) {
      const mtx = (model[s.path] ??= []) as any[][];
      const b = blank(s);
      if (s.rows.path === path && s.cols.path === path) {
        for (const row of mtx) row.push(b);
        mtx.push(Array(model[path].length).fill(b));
      } else if (s.cols.path === path) {
        for (const row of mtx) row.push(b);
      } else if (s.rows.path === path) {
        mtx.push(Array((model[s.cols.path] ?? original[s.cols.path] ?? []).length).fill(b));
      }
    }
  };
  const nuevo = (path: string, key: string, label: string, nombre: string) => {
    const list = model[path] as any[];
    const extras = all.flatMap((s) => [...(s.rows.path === path ? s.rows.extra : []), ...(s.cols.path === path ? s.cols.extra : [])]);
    const base = String(list[list.length - 1]?.[key] ?? label).replace(/\d+$/, '') || label;
    const name = nombre.trim() || nombreLibre(list.map((x) => String(x[key] ?? '')), base, list.length + 1);
    list.push({ ...defaultsDe(extras), [key]: name });
    crecer(path);
  };

  if (el.crecer) {
    const nombreCol = (j: number) => a.cols.find((x) => x.rol.tipo === 'mat' && x.rol.j === j)?.nombre ?? '';
    const nombreFila = (i: number) => a.filas.find((x) => x.rol.tipo === 'mat' && x.rol.j === i)?.nombre ?? '';
    if (square) {
      const C = model[w.cols.path].length;
      for (let q = 0; q < a.falta.filas; q++) nuevo(w.rows.path, w.rows.label_key, rotulos.filas, nombreCol(C + q) || nombreFila(C + q));
      res.nuevasFilas = res.nuevasColumnas = a.falta.filas;
    } else {
      const C = model[w.cols.path].length;
      for (let q = 0; q < a.falta.columnas; q++) nuevo(w.cols.path, w.cols.label_key, rotulos.cols, nombreCol(C + q));
      const R = model[w.rows.path].length;
      for (let q = 0; q < a.falta.filas; q++) nuevo(w.rows.path, w.rows.label_key, rotulos.filas, nombreFila(R + q));
      res.nuevasFilas = a.falta.filas;
      res.nuevasColumnas = a.falta.columnas;
    }
  }
  const rows = model[w.rows.path] as any[];
  const cols = model[w.cols.path] as any[];
  const mat = (model[w.path] ??= []) as any[][];
  const R = rows.length;
  const C = cols.length;

  // nombres
  if (a.encabezado)
    for (const x of a.cols) {
      const h = x.nombre.trim();
      if (!h || x.rol.tipo !== 'mat') continue;
      if (x.rol.j >= C) {
        res.fuera++;
        continue;
      }
      if (cols[x.rol.j][w.cols.label_key] !== h) {
        cols[x.rol.j][w.cols.label_key] = h;
        res.renombradas++;
      }
      res.escritas.push([-1, x.rol.j]);
    }
  if (a.nombres)
    for (const y of a.filas) {
      const h = y.nombre.trim();
      if (!h || y.rol.tipo !== 'mat') continue;
      if (y.rol.j >= R) {
        res.fuera++;
        continue;
      }
      if (rows[y.rol.j][w.rows.label_key] !== h) {
        rows[y.rol.j][w.rows.label_key] = h;
        res.renombradas++;
      }
      res.escritas.push([y.rol.j, -1]);
    }

  // valores
  for (const y of a.filas)
    for (const x of a.cols) {
      const s = a.tabla[y.k][x.j] ?? '';
      const vacia = s.trim() === '';
      const fuera = () => {
        if (!vacia) res.fuera++;
      };
      if (y.rol.tipo === 'fuera' || x.rol.tipo === 'fuera') {
        fuera();
        continue;
      }
      if (y.rol.tipo === 'mat' && x.rol.tipo === 'mat') {
        const i = y.rol.j;
        const j = x.rol.j;
        if (i >= R || j >= C) {
          fuera();
          continue;
        }
        if (square && i === j && !w.allow_diagonal) continue;
        const v = valorCampo(w.cell, s, a.coma);
        if (v === null) res.invalidas++;
        else if (v) {
          (mat[i] ??= [])[j] = v.v;
          res.escritas.push([i, j]);
        }
      } else if (y.rol.tipo === 'mat' && x.rol.tipo === 'extra') {
        const i = y.rol.j;
        const f = w.rows.extra[x.rol.k];
        if (i >= R) {
          fuera();
          continue;
        }
        const v = valorCampo(f, s, a.coma);
        if (v === null) res.invalidas++;
        else if (v) {
          rows[i][f.key] = v.v;
          res.escritas.push([i, C + x.rol.k]);
        }
      } else if (y.rol.tipo === 'extra' && x.rol.tipo === 'mat') {
        const j = x.rol.j;
        const f = w.cols.extra[y.rol.k];
        if (j >= C) {
          fuera();
          continue;
        }
        const v = valorCampo(f, s, a.coma);
        if (v === null) res.invalidas++;
        else if (v) {
          cols[j][f.key] = v.v;
          res.escritas.push([R + y.rol.k, j]);
        }
      } else fuera();
    }
  return res;
}

// ------------------------------------------------------------- registros (listas con columnas)

export interface AnalisisRegistros {
  tabla: Tabla;
  coma: boolean;
  encabezado: boolean;
  /** Columna visible de destino de cada columna del bloque (-1: no cabe). */
  destino: number[];
  r0: number;
  falta: Falta;
}

export function analizarRegistros(model: any, w: RecordsW, columnas: Field[], r: number, c: number, tabla: Tabla, coma: boolean, puedeFilas: boolean): AnalisisRegistros {
  const L = (model[w.path] ?? []).length;
  const W = tabla[0]?.length ?? 0;
  // encabezado: la primera fila trae los títulos de las columnas
  const fila0 = tabla[0] ?? [];
  const llenas = fila0.filter((x) => x.trim() !== '');
  const casan = llenas.filter((h) => columnas.some((f) => coincide(h, f))).length;
  const encabezado = tabla.length >= 1 && casan > 0 && casan * 2 >= llenas.length && llenas.every((h) => !pareceNumero(h) || columnas.some((f) => coincide(h, f)));
  const destino = encabezado
    ? fila0.map((h) => columnas.findIndex((f) => coincide(h, f)))
    : Array.from({ length: W }, (_, j) => (c + j < columnas.length ? c + j : -1));
  const nf = tabla.length - (encabezado ? 1 : 0);
  return { tabla, coma, encabezado, destino, r0: r, falta: { filas: puedeFilas ? Math.max(0, r + nf - L) : 0, columnas: 0, ultimaLD: false } };
}

export function pegarRegistros(original: any, w: RecordsW, columnas: Field[], a: AnalisisRegistros, el: Eleccion, base: string): Resultado<any[]> {
  const list = copia((original[w.path] ?? []) as any[]);
  const res: Resultado<any[]> = { modelo: list, escritas: [], invalidas: 0, fuera: 0, renombradas: 0, nuevasFilas: 0, nuevasColumnas: 0 };
  if (el.crecer)
    for (let q = 0; q < a.falta.filas; q++) {
      const item: any = defaultsDe(w.columns);
      if (w.columns.some((f) => f.key === 'name')) item.name = nombreLibre(list.map((x) => String(x.name ?? '')), String(list[list.length - 1]?.name ?? base).replace(/\d+$/, '') || base, list.length + 1);
      list.push(item);
      res.nuevasFilas++;
    }
  const d0 = a.encabezado ? 1 : 0;
  for (let k = d0; k < a.tabla.length; k++) {
    const i = a.r0 + k - d0;
    a.tabla[k].forEach((s, j) => {
      const vacia = s.trim() === '';
      const ci = a.destino[j];
      if (i >= list.length || ci < 0) {
        if (!vacia) res.fuera++;
        return;
      }
      const f = columnas[ci];
      const v = valorCampo(f, s, a.coma);
      if (v === null) res.invalidas++;
      else if (v) {
        list[i][f.key] = v.v;
        res.escritas.push([i, ci]);
      }
    });
  }
  return res;
}
