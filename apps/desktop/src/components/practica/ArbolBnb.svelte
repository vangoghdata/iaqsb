<script lang="ts">
  // Árbol de branch and bound que crece durante la práctica (SVG).
  import * as P from '../../lib/practica';
  import type { LpModel } from '../../lib/types';
  import { t } from '../../lib/i18n.svelte';

  let {
    nodos,
    model,
    actual = null,
    pila = [],
    mejor = null
  }: { nodos: P.BNode[]; model: LpModel; actual?: number | null; pila?: number[]; mejor?: number | null } = $props();

  const W = 150;
  const H = 60;
  const DX = 166;
  const DY = 110;
  const PAD = 14;

  const pos = $derived(P.layoutTree(nodos));
  const cols = $derived(Math.max(1, ...[...pos.values()].map((p) => p.x + 1)));
  const depth = $derived(Math.max(0, ...nodos.map((n) => n.depth)));
  const width = $derived(PAD * 2 + (cols - 1) * DX + W);
  const height = $derived(PAD * 2 + depth * DY + H);
  const xy = (n: P.BNode) => {
    const p = pos.get(n.id) ?? { x: 0, y: 0 };
    return { x: PAD + p.x * DX, y: PAD + p.y * DY };
  };
  const byId = $derived(new Map(nodos.map((n) => [n.id, n])));

  function estado(n: P.BNode): string {
    if (n.id === actual) return t('nodo actual');
    if (pila.includes(n.id)) return t('por explorar');
    switch (n.decision) {
      case 'branch':
        return t('ramificado en {0}', model.variables[n.branchVar ?? 0]?.name ?? '');
      case 'prune_infeasible':
        return t('podado: no factible');
      case 'prune_bound':
        return t('podado por cota');
      case 'incumbent':
        return n.id === mejor ? t('mejor solución') : t('entera (superada)');
    }
    return '';
  }
  const zText = (n: P.BNode) => (n.status === 'optimal' ? `z = ${P.fmtNum(n.z)}` : n.status === 'unbounded' ? t('no acotado') : t('no factible'));
  const detalle = (n: P.BNode) =>
    n.status === 'optimal' ? model.variables.map((v, j) => `${v.name} = ${P.fmtNum(n.values[j])}`).join(', ') : t('Sin solución factible');
</script>

<div class="arbol card">
  <svg {width} {height} viewBox={`0 0 ${width} ${height}`} role="img" aria-label={t('Árbol de branch and bound con {0} nodos', nodos.length)}>
    {#each nodos as n (n.id)}
      {#if n.parent !== null && byId.get(n.parent)}
        {@const a = xy(byId.get(n.parent)!)}
        {@const b = xy(n)}
        {@const my = (a.y + H + b.y) / 2}
        {@const mx = b.x + W / 2}
        {@const ly = b.y - 17}
        <path class="edge" d={`M${a.x + W / 2},${a.y + H} C${a.x + W / 2},${my} ${b.x + W / 2},${my} ${b.x + W / 2},${b.y}`} />
        <g class="lbl">
          <rect x={mx - 42} y={ly - 10} width="84" height="20" rx="10" />
          <text x={mx} y={ly + 4} text-anchor="middle">{n.branch}</text>
        </g>
      {/if}
    {/each}
    {#each nodos as n (n.id)}
      {@const p = xy(n)}
      <g
        class="n"
        class:actual={n.id === actual}
        class:pend={pila.includes(n.id)}
        class:podado={n.decision === 'prune_infeasible' || n.decision === 'prune_bound'}
        class:entera={n.decision === 'incumbent'}
        class:mejor={n.id === mejor}
      >
        <title>{t('Nodo {0}', n.id)}: {detalle(n)}</title>
        <rect x={p.x} y={p.y} width={W} height={H} rx="9" />
        <text class="id" x={p.x + 10} y={p.y + 17}>{t('Nodo {0}', n.id)}</text>
        {#if n.order}<text class="ord" x={p.x + W - 10} y={p.y + 17} text-anchor="end">#{n.order}</text>{/if}
        <text class="z" x={p.x + 10} y={p.y + 35}>{zText(n)}</text>
        <text class="st" x={p.x + 10} y={p.y + 51}>{n.id === mejor ? '★ ' : ''}{estado(n)}</text>
      </g>
    {/each}
  </svg>
</div>

<style>
  .arbol {
    overflow: auto;
    max-width: 100%;
    padding: 4px;
  }
  svg {
    display: block;
  }
  .edge {
    fill: none;
    stroke: var(--rule);
    stroke-width: 1.5;
  }
  .lbl rect {
    fill: var(--sheet-2);
    stroke: var(--rule);
  }
  .lbl text {
    font-size: 11.5px;
    font-weight: 650;
    fill: var(--ink);
  }
  .n rect {
    fill: var(--sheet);
    stroke: var(--rule);
    stroke-width: 1.2;
  }
  .n text {
    fill: var(--ink);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .n .id {
    font-weight: 700;
  }
  .n .ord {
    fill: var(--ink-3);
    font-size: 11px;
  }
  .n .z {
    font-weight: 600;
  }
  .n .st {
    fill: var(--ink-2);
    font-size: 11px;
  }
  .n.pend rect {
    stroke-dasharray: 4 3;
    fill: transparent;
  }
  .n.podado text {
    fill: var(--ink-3);
  }
  .n.entera rect {
    fill: var(--good-tint);
  }
  .n.mejor rect {
    stroke: var(--good);
    stroke-width: 2;
  }
  .n.actual rect {
    stroke: var(--solver);
    stroke-width: 2.5;
    fill: var(--solver-tint);
  }
</style>
