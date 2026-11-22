<script lang="ts">
  import type { TreeBlock } from '../../lib/gtypes';
  import { t } from '../../lib/i18n.svelte';

  let { t: tree }: { t: TreeBlock } = $props();

  // disposición de izquierda a derecha: profundidad → x, orden de hojas → y
  const layout = $derived.by(() => {
    const kids = new Map<number, number[]>();
    const roots: number[] = [];
    for (const n of tree.nodes) {
      if (n.parent === undefined || n.parent === null) roots.push(n.id);
      else kids.set(n.parent, [...(kids.get(n.parent) ?? []), n.id]);
    }
    const depth = new Map<number, number>();
    const y = new Map<number, number>();
    let leaf = 0;
    const visit = (id: number, d: number) => {
      depth.set(id, d);
      const ch = kids.get(id) ?? [];
      if (!ch.length) {
        y.set(id, leaf++);
        return;
      }
      for (const c of ch) visit(c, d + 1);
      y.set(id, (y.get(ch[0])! + y.get(ch[ch.length - 1])!) / 2);
    };
    for (const r of roots) visit(r, 0);
    const maxD = Math.max(0, ...depth.values());
    return { depth, y, leaves: Math.max(1, leaf), maxD };
  });

  const colW = $derived(Math.max(150, Math.min(220, 760 / (layout.maxD + 1))));
  const W = $derived(60 + (layout.maxD + 1) * colW + 160);
  const rowH = 46;
  const H = $derived(40 + layout.leaves * rowH);
  const X = (id: number) => 40 + (layout.depth.get(id) ?? 0) * colW;
  const Y = (id: number) => 30 + (layout.y.get(id) ?? 0) * rowH;
  const byId = $derived(new Map(tree.nodes.map((n) => [n.id, n])));
</script>

<figure class="tree">
  {#if tree.title}<figcaption>{tree.title}</figcaption>{/if}
  <div class="scroll">
    <svg viewBox={`0 0 ${W} ${H}`} width={W} role="img" aria-label={tree.title ?? t('Árbol')}>
      {#each tree.nodes.filter((n) => n.parent !== undefined && n.parent !== null) as n}
        {@const p = byId.get(n.parent!)!}
        {@const x1 = X(p.id) + 12}
        {@const y1 = Y(p.id)}
        {@const x2 = X(n.id) - 12}
        {@const y2 = Y(n.id)}
        <path class={`edge t-${n.tone ?? 'normal'}`} d={`M${x1},${y1} L${x1 + 24},${y2} L${x2},${y2}`} />
        {#if n.edge}<text class="el" x={x1 + 30} y={y2 - 6}>{n.edge}</text>{/if}
      {/each}
      {#each tree.nodes as n}
        {@const x = X(n.id)}
        {@const y = Y(n.id)}
        <g class={`node t-${n.tone ?? 'normal'}`}>
          {#if n.kind === 'decision'}
            <rect x={x - 11} y={y - 11} width="22" height="22" rx="3" />
          {:else if n.kind === 'chance'}
            <circle cx={x} cy={y} r="11" />
          {:else if n.kind === 'end'}
            <path d={`M${x - 10},${y} L${x + 9},${y - 10} L${x + 9},${y + 10} Z`} />
          {:else}
            <circle cx={x} cy={y} r="7" />
          {/if}
          <text class="nl" x={x} y={y - 16} text-anchor="middle">{n.label}</text>
          {#if n.value}<text class="nv" x={x + (n.kind === 'end' ? 16 : 0)} y={n.kind === 'end' ? y + 4 : y + 26} text-anchor={n.kind === 'end' ? 'start' : 'middle'}>{n.value}</text>{/if}
        </g>
      {/each}
    </svg>
  </div>
  {#if tree.nodes.some((n) => n.kind === 'decision' || n.kind === 'chance' || n.kind === 'end')}
    <p class="legend"><span class="sq"></span> {t('decisión')} <span class="ci"></span> {t('azar')} <span class="tr"></span> {t('resultado')}</p>
  {/if}
</figure>

<style>
  .tree {
    margin: 0;
  }
  figcaption {
    font-family: var(--font-brand);
    font-weight: 600;
    font-size: 14px;
    margin-bottom: 8px;
  }
  .scroll {
    overflow: auto;
    max-width: 100%;
    background: var(--sheet);
    border: 0.5px solid var(--rule);
    border-radius: var(--r-lg);
  }
  svg {
    display: block;
    font-size: 12px;
    max-width: none;
  }
  .edge {
    fill: none;
    stroke: var(--ink-3);
    stroke-width: 1.3;
  }
  .edge.t-highlight,
  .edge.t-accent {
    stroke: var(--binding);
    stroke-width: 2.4;
  }
  .edge.t-muted {
    stroke: var(--rule);
  }
  .el {
    fill: var(--ink-2);
    font-size: 11.5px;
  }
  .node rect,
  .node circle,
  .node path {
    fill: var(--sheet);
    stroke: var(--ink-2);
    stroke-width: 1.5;
  }
  .node.t-highlight rect,
  .node.t-highlight circle,
  .node.t-highlight path,
  .node.t-accent rect,
  .node.t-accent circle,
  .node.t-accent path {
    fill: var(--binding-tint);
    stroke: var(--binding);
    stroke-width: 2.2;
  }
  .nl {
    fill: var(--ink);
    font-weight: 600;
  }
  .nv {
    fill: var(--solver);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .legend {
    font-size: 12px;
    color: var(--ink-2);
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 6px 2px 0;
  }
  .legend span {
    display: inline-block;
    width: 11px;
    height: 11px;
    border: 1.5px solid var(--ink-2);
    margin-left: 8px;
  }
  .legend .ci {
    border-radius: 50%;
  }
  .legend .tr {
    border: 0;
    width: 0;
    height: 0;
    border-top: 6px solid transparent;
    border-bottom: 6px solid transparent;
    border-right: 11px solid var(--ink-2);
  }
</style>
