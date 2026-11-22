import { num } from '../../lib/format';
import type { Cell } from '../../lib/gtypes';

export function cell(v: Cell | undefined): string {
  if (v === null || v === undefined) return '';
  if (typeof v === 'number') return num(v);
  return v;
}

export function isNum(v: Cell | undefined): boolean {
  return typeof v === 'number' || (typeof v === 'string' && /^[−-]?∞$/.test(v));
}

/** Marcas de eje "bonitas" entre lo y hi. */
export function ticks(lo: number, hi: number, n = 6): number[] {
  if (!Number.isFinite(lo) || !Number.isFinite(hi)) return [];
  if (hi === lo) {
    hi = lo + 1;
  }
  const step0 = (hi - lo) / n;
  const mag = 10 ** Math.floor(Math.log10(step0));
  const step = [1, 2, 2.5, 5, 10].map((s) => s * mag).find((s) => s >= step0) ?? step0;
  const out: number[] = [];
  for (let v = Math.ceil(lo / step - 1e-9) * step; v <= hi + step * 1e-9; v += step) out.push(Math.abs(v) < step * 1e-9 ? 0 : v);
  return out;
}

/** Rango con margen y extremos redondeados a las marcas. */
export function niceRange(lo: number, hi: number, zero = false): [number, number] {
  if (zero) {
    lo = Math.min(lo, 0);
    hi = Math.max(hi, 0);
  }
  if (lo === hi) {
    lo -= 1;
    hi += 1;
  }
  const t = ticks(lo, hi);
  const step = t.length > 1 ? t[1] - t[0] : 1;
  return [Math.floor(lo / step + 1e-9) * step, Math.ceil(hi / step - 1e-9) * step];
}

export const toneVar: Record<string, string> = {
  normal: 'var(--ink-2)',
  accent: 'var(--solver)',
  highlight: 'var(--binding)',
  good: 'var(--good)',
  bad: 'var(--bad)',
  muted: 'var(--ink-3)',
  total: 'var(--ink)'
};

/** Colores para varias series (se reutilizan si hay más). */
export const seriesColors = ['var(--solver)', 'var(--binding)', '#34c759', '#af52de', '#30b0c7', '#ff2d55', '#5856d6', '#a2845e'];
