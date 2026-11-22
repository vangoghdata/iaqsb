<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { num } from '../lib/format';
  import { t } from '../lib/i18n.svelte';

  const g = $derived(app.graph);
  const W = 640;
  const H = 480;
  const pad = { l: 56, r: 24, t: 20, b: 44 };

  const sx = $derived((x: number) => pad.l + ((x - g!.x_range[0]) / (g!.x_range[1] - g!.x_range[0])) * (W - pad.l - pad.r));
  const sy = $derived((y: number) => H - pad.b - ((y - g!.y_range[0]) / (g!.y_range[1] - g!.y_range[0])) * (H - pad.t - pad.b));

  function ticks(lo: number, hi: number, n = 6) {
    const span = hi - lo;
    const step0 = span / n;
    const mag = 10 ** Math.floor(Math.log10(step0));
    const step = [1, 2, 2.5, 5, 10].map((s) => s * mag).find((s) => s >= step0) ?? step0;
    const out = [];
    for (let v = Math.ceil(lo / step) * step; v <= hi + 1e-9; v += step) out.push(Math.abs(v) < 1e-12 ? 0 : v);
    return out;
  }

  // segmento de a*x + b*y = c dentro de la ventana
  function segment(a: number, b: number, c: number): [number, number][] | null {
    const [x0, x1] = g!.x_range;
    const [y0, y1] = g!.y_range;
    const pts: [number, number][] = [];
    if (Math.abs(b) > 1e-12) {
      for (const x of [x0, x1]) {
        const y = (c - a * x) / b;
        if (y >= y0 - 1e-9 && y <= y1 + 1e-9) pts.push([x, y]);
      }
    }
    if (Math.abs(a) > 1e-12) {
      for (const y of [y0, y1]) {
        const x = (c - b * y) / a;
        if (x >= x0 - 1e-9 && x <= x1 + 1e-9) pts.push([x, y]);
      }
    }
    if (pts.length < 2) return null;
    pts.sort((p, q) => p[0] - q[0] || p[1] - q[1]);
    return [pts[0], pts[pts.length - 1]];
  }

  const poly = $derived(g ? g.feasible_region.map(([x, y]) => `${sx(x)},${sy(y)}`).join(' ') : '');
  const iso = $derived(
    g && g.optimum && g.objective_value !== null
      ? segment(g.objective_coefs[0], g.objective_coefs[1], g.objective_value)
      : null
  );
  const xt = $derived(g ? ticks(g.x_range[0], g.x_range[1]) : []);
  const yt = $derived(g ? ticks(g.y_range[0], g.y_range[1]) : []);
</script>

{#if !g}
  <p class="muted">{t('El método gráfico está disponible para modelos de dos variables. Resuelve el problema para dibujarlo.')}</p>
{:else}
  <div class="graph">
    <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label={t('Región factible en el plano {0}, {1}', g.x_name, g.y_name)}>
      <g class="grid">
        {#each xt as v}<line x1={sx(v)} x2={sx(v)} y1={pad.t} y2={H - pad.b} />{/each}
        {#each yt as v}<line x1={pad.l} x2={W - pad.r} y1={sy(v)} y2={sy(v)} />{/each}
      </g>
      {#if g.feasible_region.length >= 3}
        <polygon class="region" points={poly} />
      {/if}
      {#each g.lines as l}
        {@const s = segment(l.a, l.b, l.rhs)}
        {#if s}
          <line class="con" x1={sx(s[0][0])} y1={sy(s[0][1])} x2={sx(s[1][0])} y2={sy(s[1][1])} />
          {@const top = sy(s[0][1]) <= sy(s[1][1]) ? s[0] : s[1]}
          {@const atRight = sx(top[0]) > W - pad.r - 60}
          <text class="clabel" x={sx(top[0]) + (atRight ? -6 : 6)} y={sy(top[1]) + (Math.abs(s[0][1] - s[1][1]) < 1e-9 ? -6 : 14)} text-anchor={atRight ? 'end' : 'start'}>{l.name}</text>
        {/if}
      {/each}
      {#if iso}
        <line class="iso" x1={sx(iso[0][0])} y1={sy(iso[0][1])} x2={sx(iso[1][0])} y2={sy(iso[1][1])} />
      {/if}
      <g class="axes">
        <line x1={pad.l} x2={W - pad.r} y1={H - pad.b} y2={H - pad.b} />
        <line x1={pad.l} x2={pad.l} y1={pad.t} y2={H - pad.b} />
        {#each xt as v}<text x={sx(v)} y={H - pad.b + 18} text-anchor="middle">{num(v, 2)}</text>{/each}
        {#each yt as v}<text x={pad.l - 8} y={sy(v) + 4} text-anchor="end">{num(v, 2)}</text>{/each}
        <text class="axname" x={W - pad.r} y={H - 8} text-anchor="end">{g.x_name}</text>
        <text class="axname" x={pad.l + 8} y={pad.t + 12}>{g.y_name}</text>
      </g>
      {#if g.optimum}
        <circle class="opt" cx={sx(g.optimum[0])} cy={sy(g.optimum[1])} r="7" />
        <text class="optlabel" x={sx(g.optimum[0]) + 12} y={sy(g.optimum[1]) - 10}>({num(g.optimum[0])}, {num(g.optimum[1])})</text>
      {/if}
    </svg>
    <div class="legend">
      <p><span class="sw region"></span> {t('Región factible')}{g.region_unbounded ? ` ${t('(continúa fuera del dibujo)')}` : ''}</p>
      <p><span class="sw iso"></span> {t('Objetivo en el óptimo (z = {0})', num(g.objective_value))}</p>
      <p><span class="sw opt"></span> {t('Punto óptimo')}</p>
    </div>
  </div>
{/if}

<style>
  .graph {
    display: grid;
    grid-template-columns: minmax(0, 720px) minmax(200px, 300px);
    gap: 24px;
    align-items: start;
  }
  svg {
    width: 100%;
    height: auto;
    background: var(--sheet);
    border: 1px solid var(--rule);
    border-radius: var(--r-md);
    font-size: 11.5px;
  }
  .grid line {
    stroke: var(--rule-soft);
  }
  .region {
    fill: var(--solver-tint);
    stroke: var(--solver);
    stroke-width: 1.5;
  }
  .con {
    stroke: var(--ink-2);
    stroke-width: 1.25;
  }
  .clabel {
    fill: var(--ink-2);
    font-weight: 600;
  }
  .iso {
    stroke: var(--binding);
    stroke-width: 2;
    stroke-dasharray: 6 5;
  }
  .axes line {
    stroke: var(--ink-3);
  }
  .axes text {
    fill: var(--ink-2);
  }
  .axname {
    font-weight: 650;
    fill: var(--ink) !important;
  }
  .opt {
    fill: var(--binding);
    stroke: var(--sheet);
    stroke-width: 2.5;
  }
  .optlabel {
    font-weight: 700;
    fill: var(--ink);
    font-size: 13px;
  }
  .legend p {
    display: flex;
    gap: 10px;
    align-items: center;
    margin: 0 0 10px;
  }
  .sw {
    width: 22px;
    height: 12px;
    border-radius: 3px;
    flex: none;
  }
  .sw.region {
    background: var(--solver-tint);
    border: 1.5px solid var(--solver);
  }
  .sw.iso {
    height: 0;
    border-top: 2px dashed var(--binding);
  }
  .sw.opt {
    width: 12px;
    border-radius: 50%;
    background: var(--binding);
  }
  @media (max-width: 900px) {
    .graph {
      grid-template-columns: 1fr;
    }
  }
</style>
