<script lang="ts">
  // Plano de rutas: puntos en sus coordenadas (sin deformar) y un color por vehículo.
  import type { MapBlock } from '../../lib/gtypes';
  import { num } from '../../lib/format';
  import { t } from '../../lib/i18n.svelte';
  import { ticks } from './fmt';
  import GeoMap from '../generic/GeoMap.svelte';
  import type { PuntoGeo, RutaGeo } from '../../lib/grafo';

  let { m }: { m: MapBlock } = $props();

  // con latitud y longitud: mapa real (OpenStreetMap) o el plano de siempre
  let real = $state(true);
  const geoPuntos = $derived<PuntoGeo[]>(m.points.map((p) => ({ lat: p.y, lon: p.x, nombre: p.label, deposito: p.kind === 'depot', nota: p.note, ruta: p.route })));
  const geoRutas = $derived.by<RutaGeo[]>(() => {
    const by = new Map(m.points.map((p) => [p.id, p]));
    return m.routes.map((r) => ({ nombre: r.name, coords: r.stops.map((s) => by.get(s)).filter((p) => !!p).map((p) => [p!.y, p!.x] as [number, number]) }));
  });

  const COLORS = ['var(--solver)', 'var(--binding)', '#10b981', '#ef4444', '#0ea5e9', '#d946ef', '#84cc16', '#f97316', '#14b8a6', '#a855f7', '#64748b', '#eab308'];
  const color = (k: number | undefined) => (k === undefined ? 'var(--ink-3)' : COLORS[k % COLORS.length]);

  const W = 820;
  const pad = 46;
  const box = $derived.by(() => {
    const xs = m.points.map((p) => p.x);
    const ys = m.points.map((p) => p.y);
    let [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
    if (x1 - x0 < 1e-9) [x0, x1] = [x0 - 1, x1 + 1];
    if (y1 - y0 < 1e-9) [y0, y1] = [y0 - 1, y1 + 1];
    const mx = (x1 - x0) * 0.06;
    const my = (y1 - y0) * 0.06;
    return { x0: x0 - mx, x1: x1 + mx, y0: y0 - my, y1: y1 + my };
  });
  // con latitud y longitud, un grado de longitud mide cos(latitud) grados de latitud
  // (proyección equirectangular, como iaqsb-render)
  const kx = $derived.by(() => {
    if (!m.geo) return 1;
    const ys = m.points.map((p) => p.y);
    return Math.max(0.05, Math.cos((((Math.min(...ys) + Math.max(...ys)) / 2) * Math.PI) / 180));
  });
  // misma escala en X y en Y para no deformar los ángulos (método del barrido)
  const scale = $derived(Math.min((W - 2 * pad) / ((box.x1 - box.x0) * kx), 560 / (box.y1 - box.y0)));
  const H = $derived(Math.max(260, (box.y1 - box.y0) * scale + 2 * pad));
  const offX = $derived((W - (box.x1 - box.x0) * kx * scale) / 2);
  const sx = (x: number) => offX + (x - box.x0) * kx * scale;
  const tick = (v: number) => (m.geo ? `${num(v, 4)}°` : num(v, 3));
  const sy = (y: number) => H - pad - (y - box.y0) * scale;

  const byId = $derived(new Map(m.points.map((p) => [p.id, p])));
  const xt = $derived(ticks(box.x0, box.x1, 8));
  const yt = $derived(ticks(box.y0, box.y1, 6));

  function rayEnd(r: { from: string; angle: number }) {
    const p = byId.get(r.from);
    if (!p) return null;
    const a = (r.angle * Math.PI) / 180;
    const len = Math.hypot((box.x1 - box.x0) * kx, box.y1 - box.y0);
    // corta el rayo en el borde del recuadro (el ángulo es el del plano proyectado)
    let t = len;
    const dx = Math.cos(a) / kx;
    const dy = Math.sin(a);
    if (dx > 1e-9) t = Math.min(t, (box.x1 - p.x) / dx);
    if (dx < -1e-9) t = Math.min(t, (box.x0 - p.x) / dx);
    if (dy > 1e-9) t = Math.min(t, (box.y1 - p.y) / dy);
    if (dy < -1e-9) t = Math.min(t, (box.y0 - p.y) / dy);
    return { x1: sx(p.x), y1: sy(p.y), x2: sx(p.x + dx * t), y2: sy(p.y + dy * t) };
  }

  function segs(stops: string[]) {
    const out: { x1: number; y1: number; x2: number; y2: number; mx: number; my: number; ang: number; long: boolean }[] = [];
    for (let k = 0; k + 1 < stops.length; k++) {
      const a = byId.get(stops[k]);
      const b = byId.get(stops[k + 1]);
      if (!a || !b) continue;
      const [x1, y1, x2, y2] = [sx(a.x), sy(a.y), sx(b.x), sy(b.y)];
      out.push({ x1, y1, x2, y2, mx: (x1 + x2) / 2, my: (y1 + y2) / 2, ang: (Math.atan2(y2 - y1, x2 - x1) * 180) / Math.PI, long: Math.hypot(x2 - x1, y2 - y1) > 34 });
    }
    return out;
  }
  </script>

<figure class="map">
  {#if m.title}<figcaption>{m.title}</figcaption>{/if}
  {#if m.geo}
    <div class="vista" role="group" aria-label={t('Vista del mapa')}>
      <button class:on={real} aria-pressed={real} onclick={() => (real = true)}>{t('Mapa real')}</button>
      <button class:on={!real} aria-pressed={!real} onclick={() => (real = false)}>{t('Plano')}</button>
    </div>
  {/if}
  {#if m.geo && real}
    <GeoMap puntos={geoPuntos} rutas={geoRutas} alto={480} />
  {:else}
  <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label={m.title ?? t('Mapa de rutas')}>
    <g class="grid">
      {#each xt as v}<line x1={sx(v)} x2={sx(v)} y1={pad * 0.4} y2={H - pad * 0.6} />{/each}
      {#each yt as v}<line x1={offX - 6} x2={W - offX + 6} y1={sy(v)} y2={sy(v)} />{/each}
    </g>
    <g class="tk">
      {#each xt as v}<text x={sx(v)} y={H - 10} text-anchor="middle">{tick(v)}</text>{/each}
      {#each yt as v}<text x={offX - 10} y={sy(v) + 4} text-anchor="end">{tick(v)}</text>{/each}
    </g>
    {#each m.rays ?? [] as r}
      {@const e = rayEnd(r)}
      {#if e}
        <line class="ray" class:start={r.label === 'inicio' || r.label === 'start'} x1={e.x1} y1={e.y1} x2={e.x2} y2={e.y2} />
        {#if r.label}<text class="rayl" x={e.x2} y={e.y2 - 6} text-anchor="middle">{r.label}</text>{/if}
      {/if}
    {/each}
    {#each m.routes as r, k}
      <g class="route" style:--c={color(k)}>
        {#each segs(r.stops) as s}
          <line x1={s.x1} y1={s.y1} x2={s.x2} y2={s.y2} stroke-dasharray={r.dashed ? '6 5' : undefined} />
          {#if s.long}
            <path class="dir" d="M-5,-4.5 L5,0 L-5,4.5 z" transform={`translate(${s.mx},${s.my}) rotate(${s.ang})`} />
          {/if}
        {/each}
      </g>
    {/each}
    {#each m.points as p}
      {@const x = sx(p.x)}
      {@const y = sy(p.y)}
      {#if p.kind === 'depot'}
        <g class="depot">
          <rect x={x - 11} y={y - 11} width="22" height="22" rx="6" />
          <path d={`M${x - 5},${y + 1} L${x},${y - 4} L${x + 5},${y + 1} M${x - 3.5},${y} V${y + 5} H${x + 3.5} V${y}`} />
          <text class="lab" x={x} y={y - 17} text-anchor="middle">{p.label}</text>
        </g>
      {:else}
        <g class="stop" style:--c={color(p.route)}>
          <title>{p.label}{p.note ? ` · ${t('demanda {0}', p.note)}` : ''}</title>
          <circle cx={x} cy={y} r="7" />
          <text class="lab" x={x} y={y - 12} text-anchor="middle">{p.label}</text>
          {#if p.note}<text class="nt" x={x} y={y + 20} text-anchor="middle">{p.note}</text>{/if}
        </g>
      {/if}
    {/each}
  </svg>
  {/if}
  {#if m.routes.length}
    <div class="legend">
      {#each m.routes as r, k}
        <span><i style:background={color(k)}></i><b>{r.name}</b>{#if r.note}<em>{r.note}</em>{/if}</span>
      {/each}
    </div>
  {/if}
</figure>

<style>
  .map {
    margin: 0;
    max-width: 900px;
  }
  .vista {
    display: inline-flex;
    padding: 2px;
    gap: 2px;
    margin-bottom: 8px;
    background: var(--sheet-2);
    border: 1px solid var(--rule);
    border-radius: var(--r-sm);
  }
  .vista button {
    border: 0;
    background: none;
    padding: 3px 12px;
    border-radius: 5px;
    font-size: 12px;
    font-weight: 600;
    color: var(--ink-2);
  }
  .vista button.on {
    background: var(--sheet);
    color: var(--ink);
    box-shadow: var(--shadow);
  }
  figcaption {
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
  .grid line {
    stroke: var(--rule-soft);
    stroke-width: 1;
  }
  .tk text {
    fill: var(--ink-3);
    font-size: 10.5px;
    font-variant-numeric: tabular-nums;
  }
  .ray {
    stroke: var(--ink-3);
    stroke-width: 1.2;
    stroke-dasharray: 5 5;
    opacity: 0.8;
  }
  .ray.start {
    stroke: var(--solver);
  }
  .rayl {
    fill: var(--solver);
    font-size: 11px;
    font-weight: 600;
  }
  .route line {
    stroke: var(--c);
    stroke-width: 2.6;
    stroke-linecap: round;
    opacity: 0.9;
  }
  .route .dir {
    fill: var(--c);
  }
  .depot rect {
    fill: var(--accent-btn);
    stroke: var(--sheet);
    stroke-width: 2;
  }
  .depot path {
    fill: none;
    stroke: var(--accent-btn-ink);
    stroke-width: 1.6;
    stroke-linejoin: round;
  }
  .stop circle {
    fill: var(--sheet);
    stroke: var(--c);
    stroke-width: 3;
  }
  .lab {
    fill: var(--ink);
    font-weight: 600;
    paint-order: stroke;
    stroke: var(--sheet);
    stroke-width: 4px;
    stroke-linejoin: round;
  }
  .nt {
    fill: var(--ink-3);
    font-size: 10.5px;
    paint-order: stroke;
    stroke: var(--sheet);
    stroke-width: 3px;
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
    margin-top: 10px;
    font-size: 12.5px;
  }
  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 7px;
  }
  .legend i {
    width: 14px;
    height: 4px;
    border-radius: 2px;
  }
  .legend em {
    font-style: normal;
    color: var(--ink-3);
  }
</style>
