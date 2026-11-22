<script lang="ts">
  import type { ChartBlock } from '../../lib/gtypes';
  import { num } from '../../lib/format';
  import { t } from '../../lib/i18n.svelte';
  import { niceRange, seriesColors, ticks, toneVar } from './fmt';

  let { c }: { c: ChartBlock } = $props();

  const W = 760;
  // categorías largas: se parten en renglones del ancho de su espacio
  function wrap(txt: string, room: number): string[] {
    const cw = 6.4;
    if (txt.length * cw <= room) return [txt];
    const lines: string[] = [];
    let cur = '';
    for (const w of txt.split(/\s+/)) {
      const cand = cur ? `${cur} ${w}` : w;
      if (cand.length * cw <= room || !cur) cur = cand;
      else {
        lines.push(cur);
        cur = w;
      }
    }
    if (cur) lines.push(cur);
    if (lines.length > 3) {
      const max = Math.max(2, Math.floor(room / cw));
      let last = lines.slice(2).join(' ');
      if (last.length > max) last = last.slice(0, max - 1) + '…';
      return [lines[0], lines[1], last];
    }
    return lines;
  }
  const catLines = $derived.by(() => {
    if (!cats) return [] as string[][];
    const step = Math.max(1, Math.ceil(cats.length / 14));
    const room = ((W - 84) / Math.max(1, cats.length)) * step - 6;
    return cats.map((cat) => wrap(cat, room));
  });
  const extra = $derived(Math.max(0, Math.max(1, ...catLines.map((l) => l.length)) - 1) * 13);
  const H = $derived(360 + extra);

  const cats = $derived(c.categories ?? null);
  const allPts = $derived(c.series.flatMap((s) => s.points).filter((p) => Number.isFinite(p[0]) && Number.isFinite(p[1])));
  const hasBars = $derived(c.series.some((s) => s.kind === 'bar'));

  const xr = $derived.by((): [number, number] => {
    if (cats) return [-0.5, cats.length - 0.5];
    const xs = [...allPts.map((p) => p[0]), ...(c.vlines ?? []).map((v) => v.value)];
    if (!xs.length) return [0, 1];
    return niceRange(Math.min(...xs), Math.max(...xs));
  });
  // barras apiladas: la base de cada barra es la suma de las anteriores en la misma x
  const stackBase = $derived.by(() => {
    const base = new Map<string, number>();
    const acc = new Map<number, number>();
    if (!c.stacked) return base;
    c.series.forEach((s, i) => {
      if (s.kind !== 'bar') return;
      for (const p of s.points) {
        const b = acc.get(p[0]) ?? 0;
        base.set(`${i}:${p[0]}`, b);
        acc.set(p[0], b + (Number.isFinite(p[1]) ? p[1] : 0));
      }
    });
    return base;
  });
  const stackTops = $derived(c.stacked ? c.series.flatMap((s, i) => (s.kind === 'bar' ? s.points.map((p) => (stackBase.get(`${i}:${p[0]}`) ?? 0) + p[1]) : [])) : []);

  const yr = $derived.by((): [number, number] => {
    const ys = [...allPts.map((p) => p[1]), ...stackTops, ...(c.hlines ?? []).map((h) => h.value)];
    if (!ys.length) return [0, 1];
    return niceRange(Math.min(...ys), Math.max(...ys), c.y_zero || hasBars);
  });

  // margen izquierdo según el largo de las etiquetas del eje Y
  const pad = $derived.by(() => {
    const longest = Math.max(1, ...ticks(yr[0], yr[1], 5).map((v) => num(v, 3).length));
    return { l: Math.max(62, 34 + longest * 7.4), r: 22, t: 18, b: 52 + extra };
  });

  const sx = (x: number) => pad.l + ((x - xr[0]) / (xr[1] - xr[0] || 1)) * (W - pad.l - pad.r);
  const sy = (y: number) => H - pad.b - ((y - yr[0]) / (yr[1] - yr[0] || 1)) * (H - pad.t - pad.b);

  const xt = $derived(cats ? cats.map((_, i) => i) : ticks(xr[0], xr[1]));
  const yt = $derived(ticks(yr[0], yr[1], 5));
  const catStep = $derived(cats ? Math.max(1, Math.ceil(cats.length / 14)) : 1);

  const bars = $derived(c.series.filter((s) => s.kind === 'bar'));
  const bw = $derived.by(() => {
    if (!bars.length) return 0;
    const xs = [...new Set(bars.flatMap((s) => s.points.map((p) => p[0])))].sort((a, b) => a - b);
    let gap = xs.length > 1 ? Math.min(...xs.slice(1).map((x, i) => x - xs[i])) : xr[1] - xr[0];
    if (cats) gap = 1;
    return (Math.abs(sx(gap) - sx(0)) * 0.72) / (c.stacked ? 1 : bars.length);
  });

  // las series sin tono toman colores de la paleta que no usen las series con tono
  const plainColors = $derived.by(() => {
    const used = new Set(c.series.filter((s) => s.tone && s.tone !== 'normal').map((s) => toneVar[s.tone!]));
    const free = seriesColors.filter((x) => !used.has(x));
    const out = new Map<number, string>();
    let k = 0;
    c.series.forEach((s, i) => {
      if (!s.tone || s.tone === 'normal') out.set(i, free[k++ % free.length]);
    });
    return out;
  });

  function color(i: number, tone?: string) {
    if (tone && tone !== 'normal') return toneVar[tone];
    return plainColors.get(i) ?? seriesColors[i % seriesColors.length];
  }

  const fin = (p: [number, number]) => Number.isFinite(p[0]) && Number.isFinite(p[1]);

  // ---- valor bajo el cursor: guía vertical y cuadro con el valor de cada serie
  const xsAll = $derived([...new Set(c.series.flatMap((s) => s.points.filter(fin).map((p) => p[0])))].sort((a, b) => a - b));
  let fig: HTMLElement | undefined = $state();
  let hover = $state<{ x: number; left: number; top: number } | null>(null);
  function mover(e: PointerEvent) {
    const svg = e.currentTarget as SVGSVGElement;
    const r = svg.getBoundingClientRect();
    const vx = ((e.clientX - r.left) / r.width) * W;
    if (vx < pad.l - 4 || vx > W - pad.r + 4 || !xsAll.length) {
      hover = null;
      return;
    }
    const xv = xr[0] + ((vx - pad.l) / (W - pad.l - pad.r)) * (xr[1] - xr[0]);
    let best = xsAll[0];
    for (const x of xsAll) if (Math.abs(x - xv) < Math.abs(best - xv)) best = x;
    const fr = fig?.getBoundingClientRect();
    hover = { x: best, left: e.clientX - (fr?.left ?? 0), top: e.clientY - (fr?.top ?? 0) };
  }
  const hoverRows = $derived(
    hover
      ? c.series
          .map((s, i) => ({ name: s.name, color: color(i, s.tone), p: s.points.find((p) => fin(p) && Math.abs(p[0] - hover!.x) < 1e-9) }))
          .filter((r) => r.p)
      : []
  );

  // un punto vacío (null) corta la línea: así una serie puede tener varios tramos (curvas de nivel)
  function path(pts: [number, number][], step: boolean) {
    let d = '';
    let pen = false;
    for (const p of pts) {
      if (!fin(p)) {
        pen = false;
        continue;
      }
      if (!pen) d += ` M${sx(p[0])},${sy(p[1])}`;
      else if (step) d += ` H${sx(p[0])} V${sy(p[1])}`;
      else d += ` L${sx(p[0])},${sy(p[1])}`;
      pen = true;
    }
    return d.trim();
  }
</script>

<figure class="chart" bind:this={fig}>
  {#if c.title}<figcaption>{c.title}</figcaption>{/if}
  <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label={c.title ?? t('{0} según {1}', c.y_label, c.x_label)} onpointermove={mover} onpointerleave={() => (hover = null)}>
    <g class="grid">
      {#each yt as v}<line x1={pad.l} x2={W - pad.r} y1={sy(v)} y2={sy(v)} />{/each}
    </g>
    {#each c.series as s, i}
      {@const col = color(i, s.tone)}
      {#if s.kind === 'area'}
        <path d={`${path(s.points, false)} L${sx(s.points[s.points.length - 1]?.[0] ?? 0)},${sy(Math.max(yr[0], 0))} L${sx(s.points[0]?.[0] ?? 0)},${sy(Math.max(yr[0], 0))} Z`} fill={col} opacity="0.16" />
        <path d={path(s.points, false)} fill="none" stroke={col} stroke-width="2.25" stroke-linejoin="round" />
      {:else if s.kind === 'line' || s.kind === 'step'}
        <path d={path(s.points, s.kind === 'step')} fill="none" stroke={col} stroke-width="2.25" stroke-linejoin="round" stroke-linecap="round" stroke-dasharray={s.dashed ? '6 5' : undefined} />
        {#if s.points.length <= 40}
          {#each s.points.filter(fin) as p}<circle cx={sx(p[0])} cy={sy(p[1])} r="3.2" fill={col} stroke="var(--sheet)" stroke-width="1.4" />{/each}
        {/if}
      {:else if s.kind === 'scatter'}
        {@const many = s.points.length > 60}
        {#each s.points.filter(fin) as p}<circle cx={sx(p[0])} cy={sy(p[1])} r={many ? 1.7 : 4} fill={col} opacity={many ? 0.3 : 0.85} />{/each}
      {:else if s.kind === 'bar'}
        {@const bi = bars.indexOf(s)}
        {#each s.points as p}
          {#if c.stacked}
            {@const b0 = stackBase.get(`${i}:${p[0]}`) ?? 0}
            <rect x={sx(p[0]) - bw / 2} y={Math.min(sy(b0 + p[1]), sy(b0))} width={Math.max(1, bw)} height={Math.abs(sy(b0 + p[1]) - sy(b0))} rx="2" fill={col} stroke="var(--sheet)" stroke-width="1"><title>{s.name}: {num(p[1])}</title></rect>
          {:else}
            {@const x0 = sx(p[0]) - (bw * bars.length) / 2 + bi * bw}
            <rect x={x0} y={Math.min(sy(p[1]), sy(0))} width={Math.max(1, bw - 2)} height={Math.abs(sy(p[1]) - sy(Math.max(yr[0], 0)))} fill={col} rx="4"><title>{s.name}: {num(p[1])}</title></rect>
          {/if}
        {/each}
      {/if}
    {/each}
    {#each c.hlines ?? [] as h}
      <line class="ref" x1={pad.l} x2={W - pad.r} y1={sy(h.value)} y2={sy(h.value)} stroke={toneVar[h.tone ?? 'normal']} />
      <text class="reft" x={W - pad.r - 4} y={sy(h.value) - 5} text-anchor="end" fill={toneVar[h.tone ?? 'normal']}>{h.label === num(h.value) || h.label === '' ? num(h.value) : `${h.label} = ${num(h.value)}`}</text>
    {/each}
    {#each c.vlines ?? [] as v}
      <line class="ref" y1={pad.t} y2={H - pad.b} x1={sx(v.value)} x2={sx(v.value)} stroke={toneVar[v.tone ?? 'normal']} />
      <text class="reft" x={sx(v.value) + 4} y={pad.t + 12} fill={toneVar[v.tone ?? 'normal']}>{v.label}</text>
    {/each}
    {#if hover}
      <line class="guia" x1={sx(hover.x)} x2={sx(hover.x)} y1={pad.t} y2={H - pad.b} />
      {#each hoverRows as r}<circle cx={sx(r.p![0])} cy={sy(r.p![1])} r="4.5" fill={r.color} stroke="var(--sheet)" stroke-width="2" />{/each}
    {/if}
    <g class="axes">
      <line x1={pad.l} x2={W - pad.r} y1={H - pad.b} y2={H - pad.b} />
      {#each xt as v, i}
        {#if !cats || i % catStep === 0}
          {#if cats}
            <text text-anchor="middle">{#each catLines[v] ?? [] as line, li}<tspan x={sx(v)} y={H - pad.b + 17 + li * 13}>{line}</tspan>{/each}</text>
          {:else}
            <text x={sx(v)} y={H - pad.b + 17} text-anchor="middle">{num(v, 3)}</text>
          {/if}
        {/if}
      {/each}
      {#each yt as v}<text x={pad.l - 8} y={sy(v) + 4} text-anchor="end">{num(v, 3)}</text>{/each}
      <text class="axn" x={(pad.l + W - pad.r) / 2} y={H - 10} text-anchor="middle">{c.x_label}</text>
      <text class="axn" transform={`translate(16 ${(pad.t + H - pad.b) / 2}) rotate(-90)`} text-anchor="middle">{c.y_label}</text>
    </g>
  </svg>
  {#if c.series.length > 1}
    <div class="legend">
      {#each c.series as s, i}
        <span><i style={`background:${color(i, s.tone)}`} class:dash={s.dashed}></i>{s.name}</span>
      {/each}
    </div>
  {/if}
  {#if hover && hoverRows.length}
    <div class="tip" style:left={`${hover.left}px`} style:top={`${hover.top}px`} class:izq={fig && hover.left > fig.clientWidth * 0.6}>
      <b>{cats ? cats[hover.x] ?? num(hover.x, 3) : `${c.x_label}: ${num(hover.x, 4)}`}</b>
      {#each hoverRows as r}
        <span><i style={`background:${r.color}`}></i>{r.name}<em>{num(r.p![1], 4)}</em></span>
      {/each}
    </div>
  {/if}
</figure>

<style>
  .chart {
    position: relative;
    margin: 0;
    max-width: 860px;
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
    font-size: 11.5px;
    box-shadow: var(--shadow);
  }
  .grid line {
    stroke: var(--rule-soft);
  }
  .axes line {
    stroke: var(--rule);
  }
  .axes text {
    fill: var(--ink-3);
    font-variant-numeric: tabular-nums;
  }
  .axn {
    font-weight: 600;
    fill: var(--ink-2) !important;
    font-size: 12px;
  }
  .ref {
    stroke-width: 1.3;
    stroke-dasharray: 4 4;
  }
  .reft {
    font-size: 11.5px;
    font-weight: 600;
    paint-order: stroke;
    stroke: var(--sheet);
    stroke-width: 3px;
  }
  .guia {
    stroke: var(--ink-3);
    stroke-width: 1;
    stroke-dasharray: 3 3;
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 16px;
    margin-top: 8px;
    font-size: 12px;
    color: var(--ink-2);
  }
  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .legend i {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    display: inline-block;
  }
  .legend i.dash {
    width: 16px;
    height: 3px;
    border-radius: 2px;
  }
  .tip {
    position: absolute;
    z-index: 4;
    pointer-events: none;
    transform: translate(14px, -50%);
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 140px;
    padding: 8px 10px;
    border-radius: 10px;
    background: color-mix(in oklab, var(--sheet) 90%, transparent);
    -webkit-backdrop-filter: var(--vidrio);
    backdrop-filter: var(--vidrio);
    border: 0.5px solid var(--rule);
    box-shadow: var(--shadow-pop);
    font-size: 12px;
    white-space: nowrap;
  }
  .tip.izq {
    transform: translate(calc(-100% - 14px), -50%);
  }
  .tip b {
    font-weight: 600;
    margin-bottom: 2px;
  }
  .tip span {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--ink-2);
  }
  .tip i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .tip em {
    margin-left: auto;
    padding-left: 12px;
    font-style: normal;
    font-weight: 600;
    color: var(--ink);
    font-variant-numeric: tabular-nums;
  }
</style>
