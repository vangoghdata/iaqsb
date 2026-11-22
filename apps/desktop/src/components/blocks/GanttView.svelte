<script lang="ts">
  import type { GanttBlock } from '../../lib/gtypes';
  import { num } from '../../lib/format';
  import { t } from '../../lib/i18n.svelte';
  import { ticks, toneVar } from './fmt';

  let { g }: { g: GanttBlock } = $props();

  const W = 820;
  const rowH = 30;
  const labelW = $derived(Math.min(220, Math.max(90, Math.max(...g.rows.map((r) => r.label.length)) * 7.4 + 24)));
  // etiquetas de los marcadores en niveles para que no se pisen
  const mkLabels = $derived.by(() => {
    const placed: { x0: number; x1: number; lv: number }[] = [];
    return (g.markers ?? []).map((m) => {
      const w = m.label.length * 6.6 + 6;
      const x1 = sx(m.value) - 4;
      let lv = 0;
      while (placed.some((p) => p.lv === lv && !(x1 < p.x0 || x1 - w > p.x1))) lv++;
      placed.push({ x0: x1 - w, x1, lv });
      return { m, x: x1, lv };
    });
  });
  const levels = $derived(Math.max(1, ...mkLabels.map((l) => l.lv + 1)));
  const top = $derived(30 + g.rows.length * rowH);
  const H = $derived(top + 10 + levels * 15 + (g.legend?.length ? 24 : 6));
  const tmax = $derived(Math.max(1, ...g.rows.flatMap((r) => r.bars.map((b) => b.end)), ...(g.markers ?? []).map((m) => m.value)));
  const tmin = $derived(Math.min(0, ...g.rows.flatMap((r) => r.bars.map((b) => b.start))));
  const sx = (v: number) => labelW + ((v - tmin) / (tmax - tmin || 1)) * (W - labelW - 24);
  const tt = $derived(ticks(tmin, tmax, 10));
</script>

<figure class="gantt">
  {#if g.title}<figcaption>{g.title}</figcaption>{/if}
  <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label={g.title ?? t('Diagrama de Gantt')}>
    {#each tt as v}
      <line class="grid" x1={sx(v)} x2={sx(v)} y1="26" y2={H - 24} />
      <text class="tick" x={sx(v)} y="18" text-anchor="middle">{num(v, 2)}</text>
    {/each}
    {#each g.rows as r, i}
      {@const y = 30 + i * rowH}
      <text class="lab" x={labelW - 10} y={y + rowH / 2 + 4} text-anchor="end">{r.label}</text>
      <line class="rowline" x1={labelW} x2={W - 24} y1={y + rowH} y2={y + rowH} />
      {#each r.bars as b}
        <rect
          x={sx(b.start)}
          y={y + 6}
          width={Math.max(2, sx(b.end) - sx(b.start))}
          height={rowH - 12}
          rx="4"
          fill={b.tone === 'muted' ? 'var(--rule)' : toneVar[b.tone ?? 'accent'] ?? 'var(--solver)'}
          opacity={b.tone === 'muted' ? 0.9 : 0.85}
        />
        {#if b.label && sx(b.end) - sx(b.start) > b.label.length * 6.5 + 8}
          <text class="bl" x={(sx(b.start) + sx(b.end)) / 2} y={y + rowH / 2 + 4} text-anchor="middle">{b.label}</text>
        {/if}
      {/each}
    {/each}
    {#each g.markers ?? [] as m}
      <line class="mk" x1={sx(m.value)} x2={sx(m.value)} y1="24" y2={top + 4} stroke={toneVar[m.tone ?? 'highlight']} />
    {/each}
    {#each mkLabels as l}
      <text class="mkt" x={l.x} y={top + 18 + l.lv * 15} text-anchor="end" fill={toneVar[l.m.tone ?? 'highlight']}>{l.m.label}</text>
    {/each}
    <text class="unit" x={labelW - 10} y="18" text-anchor="end">{g.unit}</text>
    {#if g.legend?.length}
      {@const ly = H - 10}
      {#each g.legend as [tone, label], k}
        <rect x={labelW + k * 110} y={ly - 9} width="14" height="10" rx="2" fill={tone === 'muted' ? 'var(--rule)' : toneVar[tone] ?? 'var(--solver)'} />
        <text class="leg" x={labelW + k * 110 + 20} y={ly}>{label}</text>
      {/each}
    {/if}
  </svg>
</figure>

<style>
  .gantt {
    margin: 0;
    max-width: 900px;
  }
  figcaption {
    font-family: var(--font-brand);
    font-weight: 600;
    font-size: 14px;
    margin-bottom: 8px;
  }
  svg {
    width: 100%;
    height: auto;
    background: var(--sheet);
    border: 0.5px solid var(--rule);
    border-radius: var(--r-lg);
    font-size: 12px;
  }
  .grid {
    stroke: var(--rule-soft);
  }
  .rowline {
    stroke: var(--rule-soft);
  }
  .tick,
  .unit {
    fill: var(--ink-3);
    font-size: 11px;
  }
  .lab {
    fill: var(--ink);
    font-weight: 600;
  }
  .bl {
    fill: #fff;
    font-weight: 600;
    font-size: 11px;
  }
  .mk {
    stroke-width: 1.5;
    stroke-dasharray: 5 4;
  }
  .leg {
    fill: var(--ink-2);
    font-size: 11.5px;
  }
  .mkt {
    font-weight: 600;
    font-size: 11.5px;
  }
</style>
