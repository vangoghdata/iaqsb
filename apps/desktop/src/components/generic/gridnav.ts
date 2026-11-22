// Navegación tipo hoja de cálculo: Intro y flechas arriba/abajo mueven entre filas.

export function gridKeys(e: KeyboardEvent) {
  const el = e.target as HTMLElement;
  if (!(el instanceof HTMLInputElement || el instanceof HTMLSelectElement)) return;
  if (el instanceof HTMLInputElement && el.type === 'checkbox') return;
  const r = Number(el.dataset.r);
  const c = Number(el.dataset.c);
  if (Number.isNaN(r) || Number.isNaN(c)) return;
  let dr = 0;
  if (e.key === 'Enter' || (e.key === 'ArrowDown' && el instanceof HTMLInputElement)) dr = 1;
  if (e.key === 'ArrowUp' && el instanceof HTMLInputElement) dr = -1;
  if (!dr) return;
  e.preventDefault();
  el.dispatchEvent(new Event('change'));
  const scope = (e.currentTarget as HTMLElement) ?? document;
  for (let k = r + dr; k >= 0 && k < 2000; k += dr) {
    const next = scope.querySelector<HTMLElement>(`[data-r="${k}"][data-c="${c}"]`);
    if (next) {
      next.focus();
      if (next instanceof HTMLInputElement) next.select();
      return;
    }
  }
}

/** Nombre nuevo que no choca con los existentes: Nodo6, Origen4… */
export function freshName(existing: string[], fallback: string): string {
  const last = existing[existing.length - 1] ?? fallback;
  const prefix = last.replace(/\d+$/, '') || fallback;
  const used = new Set(existing.map((s) => s.toUpperCase()));
  for (let i = existing.length + 1; i < 100000; i++) {
    const n = `${prefix}${i}`;
    if (!used.has(n.toUpperCase())) return n;
  }
  return `${prefix}${Date.now()}`;
}
