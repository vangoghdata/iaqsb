<script lang="ts">
  // Tabla del símplex del modo práctica: encabezados de columna y variables
  // básicas que se pueden pulsar, cocientes y fila para escribir la nueva fila pivote.
  import * as P from '../../lib/practica';
  import { t } from '../../lib/i18n.svelte';

  let {
    tb,
    frac = true,
    pick = 'none',
    entra = null,
    sale = null,
    malCols = [],
    malFilas = [],
    descartadas = [],
    cocientes = false,
    fila = null,
    filaOk = [],
    onpickcol,
    onpickrow,
    onsubmitrow,
    onrowinput,
    compacta = false
  }: {
    tb: P.PTableau;
    frac?: boolean;
    pick?: 'cols' | 'rows' | 'none';
    entra?: number | null;
    sale?: number | null;
    malCols?: number[];
    malFilas?: number[];
    descartadas?: number[];
    cocientes?: boolean;
    /** Casillas de la nueva fila pivote (null = no se pide). */
    fila?: string[] | null;
    filaOk?: (boolean | null)[];
    onpickcol?: (j: number) => void;
    onpickrow?: (r: number) => void;
    onsubmitrow?: () => void;
    onrowinput?: (k: number, v: string) => void;
    compacta?: boolean;
  } = $props();

  const f = (x: P.Frac) => P.fmtFrac(x, frac);
  const fm = (x: P.MV) => P.fmtMV(x, frac);
  const cbs = $derived(P.cb(tb));
  const z = $derived(P.zj(tb));
  const d = $derived(P.reduced(tb));
  const obj = $derived(P.objective(tb));
  const q = $derived(entra !== null && cocientes ? P.ratios(tb, entra) : null);
  const minRows = $derived(entra !== null && cocientes ? P.minRatioRows(tb, entra) : []);

  // navegación con flechas entre los botones de columnas o de filas
  function nav(e: KeyboardEvent) {
    const el = e.target as HTMLElement;
    const grp = el.dataset.nav;
    if (!grp) return;
    const horiz = grp === 'col';
    const fwd = horiz ? e.key === 'ArrowRight' : e.key === 'ArrowDown';
    const back = horiz ? e.key === 'ArrowLeft' : e.key === 'ArrowUp';
    if (!fwd && !back) return;
    const all = [...(el.closest('table')?.querySelectorAll<HTMLElement>(`[data-nav="${grp}"]`) ?? [])];
    const i = all.indexOf(el);
    const next = all[i + (fwd ? 1 : -1)];
    if (next) {
      e.preventDefault();
      next.focus();
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="tw" class:compacta role="group" aria-label={t('Tabla del símplex')} onkeydown={nav}>
  <table>
    <thead>
      <tr>
        <th class="l">{t('Base')}</th>
        <th>C(B)</th>
        {#each tb.columns as c, j}
          <th class:col={j === entra} class:mal={malCols.includes(j)} class:desc={descartadas.includes(j)}>
            {#if pick === 'cols' && !P.isBasic(tb, j)}
              <button class="pk" data-nav="col" onclick={() => onpickcol?.(j)} title={t('Entra {0}', c)} aria-label={t('Entra {0}', c)}>{c}</button>
            {:else}
              {c}
            {/if}
          </th>
        {/each}
        <th class="rhs">{t('Lado derecho')}</th>
        {#if q}<th class="ratio">{t('Cociente')}</th>{/if}
      </tr>
      <tr class="cj">
        <th class="l">C(j)</th>
        <th></th>
        {#each tb.cj as v, j}<th class:col={j === entra}>{fm(v)}</th>{/each}
        <th class="rhs"></th>
        {#if q}<th class="ratio"></th>{/if}
      </tr>
    </thead>
    <tbody>
      {#each tb.rows as row, r}
        <tr class:pivrow={r === sale} class:malf={malFilas.includes(r)}>
          <td class="l b">
            {#if pick === 'rows'}
              <button class="pk" data-nav="row" onclick={() => onpickrow?.(r)} title={t('Sale {0}', tb.columns[tb.basis[r]])} aria-label={t('Sale {0}', tb.columns[tb.basis[r]])}>{tb.columns[tb.basis[r]]}</button>
            {:else}
              {tb.columns[tb.basis[r]]}
            {/if}
          </td>
          <td>{fm(cbs[r])}</td>
          {#each row as x, j}
            <td class:col={j === entra} class:pivot={j === entra && r === sale}>{f(x)}</td>
          {/each}
          <td class="rhs">{f(tb.rhs[r])}</td>
          {#if q}
            <td class="ratio" class:min={minRows.includes(r)}>
              {#if q[r]}{f(tb.rhs[r])} / {f(tb.rows[r][entra!])} = <strong>{f(q[r]!)}</strong>{:else}<span class="muted">—</span>{/if}
            </td>
          {/if}
        </tr>
        {#if fila && r === sale}
          <tr class="nueva">
            <td class="l b">{t('Nueva fila de {0}', tb.columns[entra!])}</td>
            <td>{fm(tb.cj[entra!])}</td>
            {#each fila as _, k}
              <td class:rhs={k === fila.length - 1}>
                <input
                  class="cell"
                  class:bien={filaOk[k] === true}
                  class:malc={filaOk.length > 0 && filaOk[k] !== true}
                  value={fila[k]}
                  oninput={(e) => onrowinput?.(k, (e.currentTarget as HTMLInputElement).value)}
                  inputmode="decimal"
                  autocomplete="off"
                  spellcheck="false"
                  aria-label={k === fila.length - 1 ? t('Nuevo lado derecho') : t('Nuevo valor en {0}', tb.columns[k])}
                  onkeydown={(e) => {
                    if (e.key === 'Enter') {
                      e.preventDefault();
                      onsubmitrow?.();
                    }
                  }}
                />
              </td>
            {/each}
            {#if q}<td class="ratio"></td>{/if}
          </tr>
        {/if}
      {/each}
    </tbody>
    <tfoot>
      {#if !compacta}
        <tr class="zj">
          <td class="l">Z(j)</td>
          <td></td>
          {#each z as v, j}<td class:col={j === entra}>{fm(v)}</td>{/each}
          <td class="rhs"></td>
          {#if q}<td class="ratio"></td>{/if}
        </tr>
      {/if}
      <tr>
        <td class="l b">C(j)−Z(j)</td>
        <td></td>
        {#each d as v, j}<td class:col={j === entra} class:mejora={!P.isBasic(tb, j) && P.improves(tb.sense, v) && pick !== 'cols'}>{fm(v)}</td>{/each}
        <td class="rhs" title={t('Valor del objetivo')}>{fm(obj)}</td>
        {#if q}<td class="ratio"></td>{/if}
      </tr>
    </tfoot>
  </table>
</div>

<style>
  .tw {
    width: max-content;
    max-width: 100%;
    overflow: auto;
    background: var(--sheet);
    border: 1px solid var(--rule);
    border-radius: var(--r-md);
  }
  th,
  td {
    text-align: right;
    padding: 7px 12px;
    white-space: nowrap;
    border-bottom: 1px solid var(--rule-soft);
    font-variant-numeric: tabular-nums;
  }
  .compacta th,
  .compacta td {
    padding: 4px 10px;
    font-size: 12.5px;
  }
  th {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--ink-2);
    background: var(--sheet-2);
  }
  .cj th {
    font-weight: 500;
    border-bottom: 1px solid var(--rule);
  }
  .l {
    text-align: left;
  }
  .b {
    font-weight: 650;
  }
  .col {
    background: var(--solver-tint);
  }
  th.mal {
    background: var(--bad-tint);
  }
  th.desc {
    text-decoration: line-through;
  }
  .pivrow td {
    background: var(--binding-tint);
  }
  tr.malf td {
    background: var(--bad-tint);
  }
  td.pivot {
    outline: 2px solid var(--solver);
    outline-offset: -3px;
    font-weight: 750;
  }
  .rhs {
    font-weight: 650;
    border-left: 1px solid var(--rule);
  }
  .ratio {
    border-left: 1px solid var(--rule);
    color: var(--ink-2);
    font-weight: 500;
    text-align: left;
  }
  .ratio.min {
    color: var(--ink);
    background: var(--good-tint);
  }
  tfoot td {
    border-top: 1px solid var(--rule);
    border-bottom: 0;
    font-weight: 600;
  }
  tfoot .zj td {
    font-weight: 500;
    color: var(--ink-2);
  }
  td.mejora {
    color: var(--solver);
  }
  .pk {
    border: 1px solid var(--rule);
    background: var(--field);
    color: var(--ink);
    font: inherit;
    font-weight: 650;
    padding: 2px 9px;
    border-radius: var(--r-xs);
    cursor: pointer;
  }
  .pk:hover {
    border-color: var(--solver);
    background: var(--solver-tint);
  }
  .mal .pk,
  .malf .pk {
    border-color: color-mix(in oklab, var(--bad) 60%, transparent);
  }
  tr.nueva td {
    background: color-mix(in oklab, var(--solver-tint) 60%, transparent);
    padding-top: 5px;
    padding-bottom: 5px;
  }
  .cell {
    width: 72px;
    text-align: right;
    border: 1px solid var(--rule);
    border-radius: var(--r-xs);
    background: var(--field);
    padding: 3px 6px;
    font-variant-numeric: tabular-nums;
  }
  .cell.bien {
    border-color: var(--good);
    background: var(--good-tint);
  }
  .cell.malc {
    border-color: var(--bad);
    background: var(--bad-tint);
  }
</style>
