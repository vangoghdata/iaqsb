<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { num } from '../lib/format';
  import { t } from '../lib/i18n.svelte';

  const info = $derived(app.report?.branch_and_bound);
  const nodes = $derived(info?.trace ?? []);
  const pretty = (s: string) => s.replace('<=', '≤').replace('>=', '≥');
  const kind = (a: string) =>
    // el núcleo manda la acción en el idioma de la app
    /^(nueva mejor|new best)/.test(a) ? 'best' : /^(ramificar|branch)/.test(a) ? 'branch' : /^(solución entera|integer solution)/.test(a) ? 'int' : 'cut';
</script>

{#if !info}
  <p class="muted">{t('El modelo no tiene variables enteras ni binarias.')}</p>
{:else}
  <div class="bnb">
    <p class="sum">
      {info.incumbents_found === 1
        ? t('{0} nodos explorados, {1} solución entera mejorada.', info.nodes_explored, info.incumbents_found)
        : t('{0} nodos explorados, {1} soluciones enteras mejoradas.', info.nodes_explored, info.incumbents_found)}
      {t('Óptimo relajado:')}
      <strong>{num(info.relaxed_objective)}</strong>.
    </p>
    <ol class="tree">
      {#each nodes as n (n.node)}
        <li class={kind(n.action)}>
          <span class="id">{t('Nodo {0}', n.node)}</span>
          <span class="br" style={`padding-left:${n.depth * 18}px`}>{n.branches.length ? pretty(n.branches[n.branches.length - 1]) : t('Relajación lineal')}</span>
          <span class="z">{n.objective_value === null ? '–' : `z = ${num(n.objective_value)}`}</span>
          <span class="act">{n.action}</span>
        </li>
      {/each}
    </ol>
    <p class="muted small">{t('La sangría indica la profundidad de cada rama. Se explora primero la rama «≤» y se poda todo nodo que no puede mejorar la mejor solución entera.')}</p>
  </div>
{/if}

<style>
  .bnb {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .sum {
    margin: 0;
  }
  .tree {
    list-style: none;
    margin: 0;
    padding: 6px 0;
    background: var(--sheet);
    border: 1px solid var(--rule);
    border-radius: var(--r-md);
  }
  li {
    display: grid;
    grid-template-columns: 64px 220px 110px 1fr;
    gap: 14px;
    align-items: baseline;
    padding: 6px 14px;
    border-bottom: 1px solid var(--rule-soft);
    font-variant-numeric: tabular-nums;
  }
  li:last-child {
    border-bottom: 0;
  }
  .id {
    color: var(--ink-3);
    font-size: 12.5px;
  }
  .br {
    font-weight: 650;
  }
  .z {
    text-align: right;
  }
  .act {
    color: var(--ink-2);
  }
  li.best {
    background: var(--binding-tint);
  }
  li.best .act {
    color: var(--ink);
    font-weight: 650;
  }
  li.cut .br,
  li.cut .z {
    color: var(--ink-3);
  }
  .small {
    font-size: 12.5px;
    margin: 0;
  }
</style>
