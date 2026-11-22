<script lang="ts">
  // Plano a escala (estibas, bloques, pasillos) con cotas.
  import type { LayoutBlock } from '../../lib/gtypes';
  import { num } from '../../lib/format';
  import { t } from '../../lib/i18n.svelte';
  import { toneVar } from './fmt';

  let { p }: { p: LayoutBlock } = $props();

  const W = 760;
  const pad = 46;
  const scale = $derived(Math.min((W - 2 * pad) / Math.max(p.width, 1e-9), 520 / Math.max(p.height, 1e-9)));
  const H = $derived(p.height * scale + 2 * pad);
  const X = (x: number) => pad + x * scale;
  const Y = (y: number) => pad + y * scale;
  const fill = (tone?: string, soft?: boolean) =>
    soft ? 'color-mix(in srgb, var(--ink-3) 10%, transparent)' : `color-mix(in srgb, ${toneVar[tone ?? 'normal'] ?? toneVar.normal} ${tone && tone !== 'normal' ? 26 : 14}%, var(--sheet))`;
  const stroke = (tone?: string) => toneVar[tone ?? 'normal'] ?? toneVar.normal;
</script>

<figure class="plan">
  {#if p.title}<figcaption>{p.title}</figcaption>{/if}
  <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label={p.title ?? t('Plano')}>
    <rect x={X(0)} y={Y(0)} width={p.width * scale} height={p.height * scale} class="floor" />
    {#each p.rects as r}
      <rect
        x={X(r.x)}
        y={Y(r.y)}
        width={Math.max(0.5, r.w * scale)}
        height={Math.max(0.5, r.h * scale)}
        fill={fill(r.tone, r.soft)}
        stroke={r.soft ? 'none' : stroke(r.tone)}
        stroke-width={r.soft ? 0 : 1}
        rx="1.5"
      />
      {#if r.label && r.w * scale > 26 && r.h * scale > 13}
        <text x={X(r.x + r.w / 2)} y={Y(r.y + r.h / 2)} class="lbl" class:soft={r.soft} dominant-baseline="middle" text-anchor="middle"
          >{r.label}</text
        >
      {/if}
    {/each}
    {#each p.dims as d}
      {@const horiz = Math.abs(d.y2 - d.y1) < 1e-9}
      <line x1={X(d.x1)} y1={Y(d.y1)} x2={X(d.x2)} y2={Y(d.y2)} class="dim" marker-start="url(#tick)" marker-end="url(#tick)" />
      {#if horiz}
        <text x={X((d.x1 + d.x2) / 2)} y={Y(d.y1) - 6} class="dl" text-anchor="middle">{d.label}</text>
      {:else}
        <text x={X(d.x1) - 6} y={Y((d.y1 + d.y2) / 2)} class="dl" text-anchor="middle" transform={`rotate(-90 ${X(d.x1) - 6} ${Y((d.y1 + d.y2) / 2)})`}>{d.label}</text>
      {/if}
    {/each}
    <defs>
      <marker id="tick" viewBox="0 0 6 6" refX="3" refY="3" markerWidth="6" markerHeight="6" orient="auto">
        <path d="M3 0 L3 6" stroke="var(--ink-2)" stroke-width="1.2" />
      </marker>
    </defs>
  </svg>
  <p class="muted small">{t('Escala: {0} × {1} {2}', num(p.width, 3), num(p.height, 3), p.unit)}</p>
</figure>

<style>
  .plan {
    margin: 0;
  }
  figcaption {
    font-weight: 600;
    font-size: 14px;
    margin-bottom: 6px;
  }
  svg {
    width: 100%;
    max-width: 760px;
    height: auto;
    display: block;
    background: var(--sheet);
    border: 0.5px solid var(--rule);
    border-radius: var(--r-lg);
  }
  .floor {
    fill: none;
    stroke: var(--ink-2);
    stroke-width: 1.5;
  }
  .lbl {
    font-size: 11px;
    fill: var(--ink);
  }
  .lbl.soft {
    fill: var(--ink-3);
    font-size: 10px;
  }
  .dim {
    stroke: var(--ink-2);
    stroke-width: 1;
  }
  .dl {
    font-size: 11px;
    fill: var(--ink-2);
  }
  .muted {
    color: var(--ink-2);
  }
  .small {
    font-size: 12px;
    margin: 4px 0 0;
  }
</style>
