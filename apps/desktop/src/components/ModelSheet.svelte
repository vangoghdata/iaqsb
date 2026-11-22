<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { t } from '../lib/i18n.svelte';
  import { algebraic, boundCell, num, parseBound, parseNum } from '../lib/format';
  import type { Direction, VarType } from '../lib/types';
  import { pegarEnLP } from '../lib/excel';

  const m = $derived(app.model);
  const rep = $derived(app.report && !app.stale && (app.report.status === 'optimal' || app.report.status === 'feasible') ? app.report : null);
  const binding = (i: number) => {
    const c = rep?.constraints[i];
    return !!c && Math.abs(c.slack_or_surplus) < 1e-9;
  };

  let bad = $state<string | null>(null);

  function setNum(e: Event, apply: (v: number) => void, key: string) {
    const el = e.currentTarget as HTMLInputElement;
    const v = parseNum(el.value);
    if (v === null) {
      bad = key;
      app.error = t('“{0}” no es un número.', el.value);
      return;
    }
    bad = null;
    apply(v);
    app.changed();
  }

  function setBound(e: Event, j: number, which: 'lower' | 'upper') {
    const el = e.currentTarget as HTMLInputElement;
    const v = parseBound(el.value);
    if (v === null) {
      bad = `${which}-${j}`;
      app.error = t('“{0}” no es una cota válida. Usa un número, M o -M.', el.value);
      return;
    }
    bad = null;
    m.variables[j][which] = v;
    app.changed();
  }

  function setType(j: number, tipo: VarType) {
    const v = m.variables[j];
    v.type = tipo;
    if (tipo === 'binary') {
      v.lower = 0;
      v.upper = 1;
    } else if (tipo === 'unrestricted') {
      v.lower = '-M';
      v.upper = 'M';
    } else if (v.lower === '-M' && v.upper === 'M') {
      v.lower = 0;
    }
    app.changed();
  }

  function rename(e: Event, apply: (s: string) => void) {
    const s = (e.currentTarget as HTMLInputElement).value.trim();
    if (s) {
      apply(s);
      app.changed();
    }
  }

  // Intro baja a la misma columna de la fila siguiente, como en una hoja de cálculo
  function nav(e: KeyboardEvent) {
    const el = e.target as HTMLElement;
    if (!(el instanceof HTMLInputElement || el instanceof HTMLSelectElement)) return;
    const r = Number(el.dataset.r);
    const c = Number(el.dataset.c);
    if (Number.isNaN(r)) return;
    let dr = 0;
    if (e.key === 'Enter' || (e.key === 'ArrowDown' && el instanceof HTMLInputElement)) dr = 1;
    if (e.key === 'ArrowUp' && el instanceof HTMLInputElement) dr = -1;
    if (!dr) return;
    e.preventDefault();
    (el as HTMLInputElement).dispatchEvent(new Event('change'));
    for (let k = r + dr; k >= 0 && k < 200; k += dr) {
      const next = document.querySelector<HTMLElement>(`[data-r="${k}"][data-c="${c}"]`);
      if (next) {
        next.focus();
        if (next instanceof HTMLInputElement) next.select();
        return;
      }
    }
  }

  const dirs: Direction[] = ['<=', '>=', '='];
  const dirLabel = { '<=': '≤', '>=': '≥', '=': '=' } as const;
  const types: [VarType, string][] = [
    ['continuous', 'Continua'],
    ['integer', 'Entera'],
    ['binary', 'Binaria'],
    ['unrestricted', 'Libre'],
    ['semicontinuous', 'Semicontinua'],
    ['semiinteger', 'Semientera']
  ];
  const n = $derived(m.variables.length);
  const lines = $derived(algebraic(m));
</script>

<div class="wrap">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="scroller" onkeydown={nav} onpaste={pegarEnLP}>
    <table class="sheet">
      <thead>
        <tr>
          <th class="corner">{t('Variables')}</th>
          {#each m.variables as v, j (j)}
            <th class="vh">
              <div class="vname">
                <input
                  aria-label={t('Nombre de la variable {0}', j + 1)}
                  value={v.name}
                  data-r="0"
                  data-c={j + 1}
                  onchange={(e) => rename(e, (s) => (v.name = s))}
                />
                {#if n > 1}
                  <button class="x" title={t('Quitar {0}', v.name)} aria-label={t('Quitar {0}', v.name)} onclick={() => app.removeVariable(j)}>×</button>
                {/if}
              </div>
            </th>
          {/each}
          <th class="addcol">
            <button class="add" onclick={() => app.addVariable()} title={t('Añadir variable')}>+ {t('Variable')}</button>
          </th>
          <th class="dir">{t('Sentido')}</th>
          <th class="num">{t('Lado derecho')}</th>
          {#if rep}
            <th class="num out">{t('Lado izq.')}</th>
            <th class="num out">{t('Holgura')}</th>
          {/if}
        </tr>
      </thead>
      <tbody>
        <tr class="obj">
          <th class="rowh">
            <span class="objlabel">{m.objective === 'max' ? t('Maximizar') : t('Minimizar')}</span>
          </th>
          {#each m.variables as v, j (j)}
            <td>
              <input
                class="cell"
                class:bad={bad === `c-${j}`}
                inputmode="decimal"
                aria-label={t('Coeficiente de {0} en el objetivo', v.name)}
                value={String(v.cost)}
                data-r="1"
                data-c={j + 1}
                onchange={(e) => setNum(e, (x) => (v.cost = x), `c-${j}`)}
              />
            </td>
          {/each}
          <td></td>
          <td></td>
          <td class="zcell">
            {#if rep}<span class="z">z = {num(rep.objective_value)}</span>{/if}
          </td>
          {#if rep}<td></td><td></td>{/if}
        </tr>

        {#each m.constraints as c, i (i)}
          <tr class="con" class:binding={binding(i)}>
            <th class="rowh">
              <div class="cname">
                <input
                  aria-label={t('Nombre de la restricción {0}', i + 1)}
                  value={c.name}
                  data-r={i + 2}
                  data-c="0"
                  onchange={(e) => rename(e, (s) => (c.name = s))}
                />
                <button class="x" title={t('Quitar {0}', c.name)} aria-label={t('Quitar {0}', c.name)} onclick={() => app.removeConstraint(i)}>×</button>
              </div>
            </th>
            {#each c.coefs as a, j (j)}
              <td>
                <input
                  class="cell"
                  class:zero={a === 0}
                  class:bad={bad === `a-${i}-${j}`}
                  inputmode="decimal"
                  aria-label={t('Coeficiente de {0} en {1}', m.variables[j]?.name, c.name)}
                  value={String(a)}
                  data-r={i + 2}
                  data-c={j + 1}
                  onchange={(e) => setNum(e, (x) => (c.coefs[j] = x), `a-${i}-${j}`)}
                />
              </td>
            {/each}
            <td></td>
            <td class="dir">
              <select
                aria-label={t('Sentido de {0}', c.name)}
                value={c.direction}
                data-r={i + 2}
                data-c={n + 2}
                onchange={(e) => {
                  c.direction = (e.currentTarget as HTMLSelectElement).value as Direction;
                  app.changed();
                }}
              >
                {#each dirs as d}<option value={d}>{dirLabel[d]}</option>{/each}
              </select>
            </td>
            <td>
              <input
                class="cell"
                class:bad={bad === `b-${i}`}
                inputmode="decimal"
                aria-label={t('Lado derecho de {0}', c.name)}
                value={String(c.rhs)}
                data-r={i + 2}
                data-c={n + 3}
                onchange={(e) => setNum(e, (x) => (c.rhs = x), `b-${i}`)}
              />
            </td>
            {#if rep}
              <td class="out">{num(rep.constraints[i]?.lhs)}</td>
              <td class="out">
                {#if binding(i)}<span class="active" title={t('La restricción se cumple con igualdad: limita la solución')}>{t('activa')}</span>{:else}{num(rep.constraints[i]?.slack_or_surplus)}{/if}
              </td>
            {/if}
          </tr>
        {/each}

        <tr class="addrow">
          <th class="rowh"><button class="add" onclick={() => app.addConstraint()}>+ {t('Restricción')}</button></th>
          <td colspan={n + 3 + (rep ? 2 : 0)}></td>
        </tr>

        <tr class="meta first">
          <th class="rowh">{t('Cota inferior')}</th>
          {#each m.variables as v, j (j)}
            <td>
              <input
                class="cell"
                class:bad={bad === `lower-${j}`}
                aria-label={t('Cota inferior de {0}', v.name)}
                value={boundCell(v.lower, 0)}
                disabled={v.type === 'binary' || v.type === 'unrestricted'}
                data-r={m.constraints.length + 3}
                data-c={j + 1}
                onchange={(e) => setBound(e, j, 'lower')}
              />
            </td>
          {/each}
          <td colspan={3 + (rep ? 2 : 0)}></td>
        </tr>
        <tr class="meta">
          <th class="rowh">{t('Cota superior')}</th>
          {#each m.variables as v, j (j)}
            <td>
              <input
                class="cell"
                class:bad={bad === `upper-${j}`}
                aria-label={t('Cota superior de {0}', v.name)}
                value={boundCell(v.upper, 'M')}
                disabled={v.type === 'binary' || v.type === 'unrestricted'}
                data-r={m.constraints.length + 4}
                data-c={j + 1}
                onchange={(e) => setBound(e, j, 'upper')}
              />
            </td>
          {/each}
          <td colspan={3 + (rep ? 2 : 0)}></td>
        </tr>
        <tr class="meta">
          <th class="rowh">{t('Tipo')}</th>
          {#each m.variables as v, j (j)}
            <td>
              <select
                aria-label={t('Tipo de {0}', v.name)}
                value={v.type ?? 'continuous'}
                data-r={m.constraints.length + 5}
                data-c={j + 1}
                onchange={(e) => setType(j, (e.currentTarget as HTMLSelectElement).value as VarType)}
              >
                {#each types as [tipo, label]}<option value={tipo}>{t(label)}</option>{/each}
              </select>
            </td>
          {/each}
          <td colspan={3 + (rep ? 2 : 0)}></td>
        </tr>
        {#if rep}
          <tr class="sol">
            <th class="rowh">{t('Solución')}</th>
            {#each rep.variables as v (v.name)}
              <td class="solv">{num(v.value)}</td>
            {/each}
            <td colspan={3 + 2}></td>
          </tr>
        {/if}
      </tbody>
    </table>
  </div>

  <section class="alg" aria-label={t('Modelo en forma algebraica')}>
    <h3>{t('El modelo en forma algebraica')}</h3>
    <p class="first">{lines[0]?.replace(/^(Maximizar|Minimizar)/, (w) => t(w))}</p>
    {#if lines.length > 1}<p class="muted st">{t('sujeto a')}</p>{/if}
    {#each lines.slice(1) as l}<p>{l}</p>{/each}
  </section>
</div>

<style>
  .wrap {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }
  .scroller {
    width: max-content;
    max-width: 100%;
    overflow: auto;
    background: var(--sheet);
    border: 1px solid var(--rule);
    border-radius: var(--r-md);
  }
  .sheet {
    width: auto;
  }
  th,
  td {
    padding: 0;
    border-bottom: 1px solid var(--rule-soft);
    height: 36px;
    white-space: nowrap;
  }
  thead th {
    background: var(--sheet-2);
    border-bottom: 1px solid var(--rule);
    font-weight: 600;
    font-size: 12.5px;
    color: var(--ink-2);
    text-align: left;
    padding: 0 10px;
    height: 40px;
  }
  thead th.vh {
    padding: 0 4px;
  }
  .corner {
    width: 150px;
  }
  .rowh {
    text-align: left;
    padding: 0 10px;
    font-weight: 550;
    color: var(--ink-2);
    font-size: 13px;
    position: sticky;
    left: 0;
    background: var(--sheet);
    z-index: 1;
  }
  .vname,
  .cname {
    display: flex;
    align-items: center;
  }
  .vname input,
  .cname input {
    width: 100%;
    min-width: 0;
    border: 0;
    background: transparent;
    padding: 6px;
    border-radius: var(--r-sm);
    font-weight: 650;
    color: var(--ink);
  }
  .vname input {
    text-align: right;
    width: 104px;
  }
  .cname input {
    width: 132px;
  }
  .vname input:hover,
  .cname input:hover {
    background: var(--rule-soft);
  }
  .x {
    border: 0;
    background: none;
    color: var(--ink-3);
    font-size: 16px;
    width: 22px;
    height: 22px;
    border-radius: 4px;
    opacity: 0;
  }
  .vname:hover .x,
  .cname:hover .x,
  .x:focus-visible {
    opacity: 1;
  }
  .x:hover {
    color: var(--bad);
    background: var(--bad-tint);
  }
  .cell {
    width: 100%;
    min-width: 104px;
    max-width: 140px;
    height: 35px;
    border: 0;
    background: transparent;
    text-align: right;
    padding: 0 10px;
    font-variant-numeric: tabular-nums;
  }
  .cell:hover {
    background: var(--rule-soft);
  }
  .cell:focus {
    background: var(--solver-tint);
    outline: 2px solid var(--solver);
    outline-offset: -2px;
  }
  .cell.zero {
    color: var(--ink-3);
  }
  .cell.bad {
    background: var(--bad-tint);
    outline: 2px solid var(--bad);
    outline-offset: -2px;
  }
  .cell:disabled {
    color: var(--ink-3);
    background: none;
  }
  select {
    border: 0;
    background-color: transparent;
    height: 34px;
    padding: 0 8px;
    width: 100%;
  }
  td.dir select {
    text-align: center;
    font-size: 15px;
  }
  .num {
    text-align: right;
  }
  .dir {
    text-align: center;
    width: 84px;
  }
  .addcol {
    width: 1%;
  }
  .add {
    border: 1px dashed var(--rule);
    background: none;
    color: var(--ink-2);
    border-radius: var(--r-sm);
    padding: 4px 10px;
    font-size: 12.5px;
    font-weight: 550;
  }
  .add:hover {
    border-color: var(--solver);
    color: var(--solver);
  }
  .obj td,
  .obj th {
    border-bottom: 2px solid var(--rule);
  }
  .obj .cell {
    font-weight: 650;
  }
  .objlabel {
    color: var(--solver);
    font-weight: 700;
  }
  .zcell {
    text-align: right;
    padding: 0 10px;
  }
  .z {
    font-weight: 700;
    color: var(--solver);
  }
  tr.binding th,
  tr.binding td {
    background: var(--binding-tint);
  }
  tr.binding .rowh {
    box-shadow: inset 3px 0 0 var(--binding);
  }
  .active {
    display: inline-block;
    font-size: 11.5px;
    font-weight: 650;
    color: var(--binding);
    border: 1px solid var(--binding);
    border-radius: 999px;
    padding: 0 8px;
    line-height: 18px;
  }
  td.out {
    text-align: right;
    padding: 0 12px;
    color: var(--ink-2);
  }
  th.out {
    color: var(--ink-3);
  }
  .addrow th,
  .addrow td {
    height: 44px;
  }
  .meta th,
  .meta td {
    background: var(--sheet-2);
  }
  .meta .rowh {
    background: var(--sheet-2);
    font-weight: 500;
  }
  .meta.first th,
  .meta.first td {
    border-top: 1px solid var(--rule);
  }
  .sol th,
  .sol td {
    background: var(--solver-tint);
    border-bottom: 0;
  }
  .sol .rowh {
    color: var(--solver);
    font-weight: 700;
    background: var(--solver-tint);
  }
  .solv {
    text-align: right;
    padding: 0 10px;
    font-weight: 700;
    color: var(--solver);
  }
  .alg {
    max-width: 72ch;
    padding: 0 4px;
  }
  .alg h3 {
    font-size: 15px;
    margin-bottom: 8px;
  }
  .alg p {
    margin: 2px 0;
    font-variant-numeric: tabular-nums;
  }
  .alg .first {
    font-weight: 600;
  }
  .alg .st {
    margin: 6px 0 4px;
    font-style: italic;
  }
</style>
