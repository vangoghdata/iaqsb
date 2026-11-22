<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { mval, num } from '../lib/format';
  import { t } from '../lib/i18n.svelte';

  const steps = $derived(app.report?.simplex_trace ?? []);
  let k = $state(0);
  $effect(() => {
    // al resolver de nuevo, vuelve a la primera tabla
    steps;
    k = 0;
  });
  const tb = $derived(steps[Math.min(k, steps.length - 1)]);
  const enterIdx = $derived(tb?.entering ? tb.columns.indexOf(tb.entering) : -1);
  const leaveRow = $derived(tb?.leaving ? tb.basis.indexOf(tb.leaving) : -1);
  const last = $derived(k >= steps.length - 1);

  function keys(e: KeyboardEvent) {
    if (e.key === 'ArrowRight') k = Math.min(k + 1, steps.length - 1);
    if (e.key === 'ArrowLeft') k = Math.max(k - 1, 0);
  }
</script>

{#if !steps.length}
  <p class="muted">
    {#if app.report?.method === 'branch_and_bound'}
      {t('En problemas enteros el paso a paso está en la pestaña Branch and bound.')}
    {:else}
      {t('Activa «Mostrar pasos» y resuelve de nuevo para ver las tablas del símplex.')}
    {/if}
  </p>
{:else if tb}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div class="steps" tabindex="0" onkeydown={keys} role="group" aria-label={t('Tablas del símplex')}>
    <div class="bar">
      <button class="btn" disabled={k === 0} onclick={() => (k = 0)}>{t('Inicio')}</button>
      <button class="btn" disabled={k === 0} onclick={() => k--}>{t('Anterior')}</button>
      <div class="where">
        <strong>{k === 0 ? t('Tabla inicial') : t('Iteración {0}', tb.iteration)}</strong>
        <span class="muted">{t('de {0}', steps.length - 1)}</span>
      </div>
      <button class="btn" disabled={last} onclick={() => k++}>{t('Siguiente')}</button>
      <button class="btn" disabled={last} onclick={() => (k = steps.length - 1)}>{t('Tabla final')}</button>
    </div>

    <p class="move">
      {#if tb.entering}
        {t('Entra')} <strong class="in">{tb.entering}</strong>{#if tb.leaving}, {t('sale')} <strong class="outv">{tb.leaving}</strong>{/if}.
        {#if !tb.leaving}{t('La columna no tiene elementos positivos: el problema no está acotado.')}{/if}
      {:else}
        {t('Ningún C(j)−Z(j) mejora el objetivo: esta es la tabla final.')}
      {/if}
    </p>

    <div class="tw">
      <table>
        <thead>
          <tr>
            <th class="l">{t('Base')}</th>
            <th>C(B)</th>
            {#each tb.columns as c, j}<th class:col={j === enterIdx}>{c}</th>{/each}
            <th>{t('Lado derecho')}</th>
          </tr>
          <tr class="cj">
            <th class="l">C(j)</th>
            <th></th>
            {#each tb.cj as v, j}<th class:col={j === enterIdx}>{mval(v)}</th>{/each}
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each tb.rows as row, r}
            <tr class:pivrow={r === leaveRow}>
              <td class="l b">{tb.basis[r]}</td>
              <td>{mval(tb.cb[r])}</td>
              {#each row as x, j}
                <td class:col={j === enterIdx} class:pivot={j === enterIdx && r === leaveRow}>{num(x)}</td>
              {/each}
              <td class="rhs">{num(tb.rhs[r])}</td>
            </tr>
          {/each}
        </tbody>
        <tfoot>
          <tr>
            <td class="l b">C(j)−Z(j)</td>
            <td></td>
            {#each tb.cj_minus_zj as v, j}<td class:col={j === enterIdx}>{mval(v)}</td>{/each}
            <td class="rhs">{mval(tb.objective)}</td>
          </tr>
        </tfoot>
      </table>
    </div>
    <p class="muted small">{t('Usa las flechas ← y → para avanzar. La columna que entra está resaltada y el pivote, enmarcado.')}</p>
  </div>
{/if}

<style>
  .steps {
    display: flex;
    flex-direction: column;
    gap: 14px;
    outline: none;
  }
  .bar {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  .where {
    min-width: 170px;
    text-align: center;
  }
  .where strong {
    font-family: var(--font-brand);
    font-size: 17px;
    margin-right: 6px;
  }
  .move {
    margin: 0;
  }
  .in {
    color: var(--solver);
  }
  .outv {
    color: var(--binding);
  }
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
  .pivrow td {
    background: var(--binding-tint);
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
  tfoot td {
    border-top: 1px solid var(--rule);
    border-bottom: 0;
    font-weight: 600;
  }
  .small {
    font-size: 12.5px;
    margin: 0;
  }
</style>
