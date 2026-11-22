import type { Bound, LpModel, MValue } from './types';
import { t } from './i18n.svelte';
import { ajustes } from './ajustes.svelte';

/** Número para mostrar: decimales y separador según los Ajustes. */
export function num(x: number | null | undefined, digits?: number): string {
  if (x === null || x === undefined || Number.isNaN(x)) return '–';
  if (!Number.isFinite(x)) return x > 0 ? '∞' : '−∞';
  const d = Math.max(0, Math.min(10, digits ?? ajustes.decimales));
  const r = Math.abs(x) < 1e-10 ? 0 : x;
  let s = r.toFixed(d);
  if (s.includes('.')) s = s.replace(/0+$/, '').replace(/\.$/, '');
  if (s === '-0') s = '0';
  s = s.replace('-', '−');
  return ajustes.separador === 'coma' ? s.replace('.', ',') : s;
}

export function bound(b: Bound | null | undefined): string {
  if (b === null || b === undefined) return '–';
  if (b === 'M') return '∞';
  if (b === '-M') return '−∞';
  return num(b);
}

/** Texto de celda para una cota: número o M / -M como en WinQSB. */
export function boundCell(b: Bound | undefined, fallback: Bound): string {
  const v = b ?? fallback;
  return typeof v === 'number' ? String(v) : v;
}

export function parseBound(s: string): Bound | null {
  const t = s.trim().toUpperCase().replace(/\s/g, '');
  if (['M', '+M', 'INF', '∞', '+∞'].includes(t)) return 'M';
  if (['-M', '-INF', '−M', '-∞', '−∞'].includes(t)) return '-M';
  const v = parseNum(s);
  return v === null ? null : v;
}

/** Acepta coma o punto decimal. */
export function parseNum(s: string): number | null {
  let t = s.trim().replace(/\s/g, '').replace('−', '-');
  if (t === '') return 0;
  if (/^-?\d+,\d+$/.test(t)) t = t.replace(',', '.');
  t = t.replace(/[$,]/g, '');
  const v = Number(t);
  return Number.isFinite(v) ? v : null;
}

export function mval(v: MValue): string {
  const m = Math.abs(v.m) < 1e-10 ? 0 : v.m;
  const c = Math.abs(v.c) < 1e-10 ? 0 : v.c;
  if (m === 0) return num(c);
  const mp = m === 1 ? 'M' : m === -1 ? '−M' : `${num(m)}M`;
  if (c === 0) return mp;
  return `${mp} ${c > 0 ? '+' : '−'} ${num(Math.abs(c))}`;
}

function term(c: number, name: string, first: boolean): string {
  if (c === 0) return '';
  const sign = c < 0 ? (first ? '−' : ' − ') : first ? '' : ' + ';
  const a = Math.abs(c);
  return `${sign}${a === 1 ? '' : `${num(a)} `}${name}`;
}

export function expression(coefs: number[], names: string[]): string {
  let s = '';
  coefs.forEach((c, j) => {
    s += term(c, names[j], s === '');
  });
  return s || '0';
}

export function algebraic(model: LpModel): string[] {
  const names = model.variables.map((v) => v.name);
  const lines = [
    `${model.objective === 'max' ? t('Maximizar') : t('Minimizar')}  ${expression(model.variables.map((v) => v.cost), names)}`
  ];
  for (const c of model.constraints) {
    const d = c.direction === '<=' ? '≤' : c.direction === '>=' ? '≥' : '=';
    lines.push(`${c.name}:  ${expression(c.coefs, names)} ${d} ${num(c.rhs)}`);
  }
  return lines;
}
