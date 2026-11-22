<script lang="ts">
  import type { NetworkBlock } from '../../lib/gtypes';
  import { t } from '../../lib/i18n.svelte';

  let { n }: { n: NetworkBlock } = $props();

  const W = 820;
  const H = $derived(Math.max(300, Math.min(620, 140 + 52 * maxPerColumn())));
  const pad = 60;

  function maxPerColumn() {
    const cols = new Map<number, number>();
    for (const nd of n.nodes) {
      const k = Math.round(nd.x * 20);
      cols.set(k, (cols.get(k) ?? 0) + 1);
    }
    return Math.max(4, ...cols.values());
  }

  const pos = $derived(
    new Map(
      n.nodes.map((nd) => {
        const w = Math.max(44, nd.label.length * 7.6 + 22);
        return [nd.id, { x: pad + nd.x * (W - 2 * pad), y: pad * 0.7 + nd.y * (H - 1.4 * pad), w, h: 30 }];
      })
    )
  );

  const pairSet = $derived(new Set(n.edges.map((e) => `${e.from}>${e.to}`)));

  // punto del borde del rectángulo redondeado en dirección (dx, dy)
  function border(p: { x: number; y: number; w: number; h: number }, dx: number, dy: number) {
    const len = Math.hypot(dx, dy) || 1;
    const ux = dx / len;
    const uy = dy / len;
    const tx = Math.abs(ux) > 1e-9 ? p.w / 2 / Math.abs(ux) : Infinity;
    const ty = Math.abs(uy) > 1e-9 ? p.h / 2 / Math.abs(uy) : Infinity;
    const t = Math.min(tx, ty) + 3;
    return { x: p.x + ux * t, y: p.y + uy * t };
  }

  function geom(e: NetworkBlock['edges'][number]) {
    const a = pos.get(e.from)!;
    const b = pos.get(e.to)!;
    if (!a || !b) return null;
    if (e.from === e.to) {
      // lazo: sale y entra por el lado del nodo que mira hacia fuera del dibujo
      let ox = a.x - W / 2;
      let oy = a.y - H / 2;
      const ol = Math.hypot(ox, oy);
      if (ol < 1) {
        ox = 0;
        oy = -1;
      } else {
        ox /= ol;
        oy /= ol;
      }
      const px = -oy;
      const py = ox;
      const base = border(a, ox, oy);
      const s = { x: base.x + px * 9, y: base.y + py * 9 };
      const t = { x: base.x - px * 9, y: base.y - py * 9 };
      const c1 = { x: s.x + ox * 46 + px * 22, y: s.y + oy * 46 + py * 22 };
      const c2 = { x: t.x + ox * 46 - px * 22, y: t.y + oy * 46 - py * 22 };
      const d = `M${s.x},${s.y} C${c1.x},${c1.y} ${c2.x},${c2.y} ${t.x},${t.y}`;
      const top = { x: base.x + ox * 38, y: base.y + oy * 38 };
      return { d, at: (_u: number) => top };
    }
    const both = n.directed && pairSet.has(`${e.to}>${e.from}`);
    const dx = b.x - a.x;
    const dy = b.y - a.y;
    const len = Math.hypot(dx, dy) || 1;
    // con arcos en ambos sentidos, cada uno se curva hacia su lado; los arcos
    // que vuelven hacia la izquierda se curvan para no pasar sobre los nodos
    const back = n.directed && !both && dx < -5;
    const curved = both || back;
    const off = both ? 18 : back ? Math.min(140, 22 + 0.22 * len) : 0;
    const nx = (-dy / len) * off;
    const ny = (dx / len) * off;
    const cx = (a.x + b.x) / 2 + nx;
    const cy = (a.y + b.y) / 2 + ny;
    const s = border(a, cx - a.x, cy - a.y);
    const t = border(b, cx - b.x, cy - b.y);
    const d = curved ? `M${s.x},${s.y} Q${cx},${cy} ${t.x},${t.y}` : `M${s.x},${s.y} L${t.x},${t.y}`;
    // punto del arco en la fracción u (para colocar la etiqueta)
    const at = (u: number) =>
      curved
        ? { x: (1 - u) ** 2 * s.x + 2 * (1 - u) * u * cx + u * u * t.x, y: (1 - u) ** 2 * s.y + 2 * (1 - u) * u * cy + u * u * t.y }
        : { x: s.x + (t.x - s.x) * u, y: s.y + (t.y - s.y) * u };
    return { d, at };
  }

  // etiquetas sin solaparse: se prueba el centro y después posiciones a lo largo del arco
  const labels = $derived.by(() => {
    const placed: { x: number; y: number; w: number }[] = [];
    const out = new Map<number, { x: number; y: number }>();
    n.edges.forEach((e, i) => {
      if (!e.label) return;
      const g = geom(e);
      if (!g) return;
      const w = e.label.length * 6.8 + 10;
      let best = g.at(0.5);
      for (const u of [0.5, 0.36, 0.64, 0.26, 0.74, 0.44, 0.56, 0.3, 0.7]) {
        const p = g.at(u);
        if (!placed.some((q) => Math.abs(q.x - p.x) < (q.w + w) / 2 + 2 && Math.abs(q.y - p.y) < 20)) {
          best = p;
          break;
        }
      }
      placed.push({ ...best, w });
      out.set(i, best);
    });
    return out;
  });

  const toneClass = (t?: string) => `t-${t ?? 'normal'}`;
  const uid = Math.random().toString(36).slice(2, 8);
</script>

<figure class="net">
  {#if n.title}<figcaption>{n.title}</figcaption>{/if}
  <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label={n.title ?? t('Red')}>
    <defs>
      {#each ['normal', 'muted', 'highlight', 'accent'] as tn}
        <marker id={`ar-${uid}-${tn}`} viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
          <path d="M0 0L10 5L0 10z" class={`fill-${tn}`} />
        </marker>
      {/each}
    </defs>
    {#each n.edges.filter((e) => e.tone !== 'highlight') as e}
      {@const g = geom(e)}
      {#if g}
        <path d={g.d} class={`edge ${toneClass(e.tone)}`} marker-end={n.directed ? `url(#ar-${uid}-${e.tone ?? 'normal'})` : undefined} />
      {/if}
    {/each}
    {#each n.edges.filter((e) => e.tone === 'highlight') as e}
      {@const g = geom(e)}
      {#if g}
        <path d={g.d} class="edge t-highlight" marker-end={n.directed ? `url(#ar-${uid}-highlight)` : undefined} />
      {/if}
    {/each}
    {#each n.edges as e, i}
      {@const p = labels.get(i)}
      {#if p && e.label}
        <g class={`elabel ${toneClass(e.tone)}`}>
          <rect x={p.x - e.label.length * 3.4 - 5} y={p.y - 9} width={e.label.length * 6.8 + 10} height="18" rx="9" />
          <text x={p.x} y={p.y + 4} text-anchor="middle">{e.label}</text>
        </g>
      {/if}
    {/each}
    {#each n.nodes as nd}
      {@const p = pos.get(nd.id)!}
      <g class={`node ${toneClass(nd.tone)}`}>
        <rect x={p.x - p.w / 2} y={p.y - p.h / 2} width={p.w} height={p.h} rx="15" />
        <text x={p.x} y={p.y + 4.5} text-anchor="middle">{nd.label}</text>
        {#if nd.note}
          <text class="note" x={p.x} y={p.y + p.h / 2 + 14} text-anchor="middle">
            {#each nd.note.split('\n') as line, li}<tspan x={p.x} dy={li ? 13 : 0}>{line}</tspan>{/each}
          </text>
        {/if}
      </g>
    {/each}
  </svg>
</figure>

<style>
  .net {
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
    font-size: 12.5px;
  }
  .edge {
    fill: none;
    stroke-width: 1.4;
  }
  .edge.t-normal {
    stroke: color-mix(in oklab, var(--ink-3) 60%, transparent);
  }
  .edge.t-muted {
    stroke: var(--rule);
  }
  .edge.t-highlight {
    stroke: var(--binding);
    stroke-width: 3;
    stroke-linecap: round;
  }
  .edge.t-accent {
    stroke: var(--solver);
    stroke-width: 2.2;
  }
  .fill-normal {
    fill: var(--ink-3);
  }
  .fill-muted {
    fill: var(--rule);
  }
  .fill-highlight {
    fill: var(--binding);
  }
  .fill-accent {
    fill: var(--solver);
  }
  .elabel rect {
    fill: var(--sheet);
    stroke: var(--rule-soft);
  }
  .elabel text {
    fill: var(--ink-2);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
  }
  .elabel.t-highlight text {
    fill: var(--ink);
    font-weight: 700;
  }
  .elabel.t-highlight rect {
    stroke: var(--binding);
  }
  .elabel.t-muted text {
    fill: var(--ink-3);
  }
  .node rect {
    fill: var(--sheet);
    stroke: color-mix(in oklab, var(--ink-3) 55%, transparent);
    stroke-width: 1.3;
  }
  .node text {
    fill: var(--ink);
    font-weight: 600;
  }
  .node.t-accent rect {
    fill: var(--solver-tint);
    stroke: var(--solver);
    stroke-width: 2;
  }
  .node.t-highlight rect {
    fill: var(--binding-tint);
    stroke: var(--binding);
    stroke-width: 2;
  }
  .node .note {
    paint-order: stroke;
    stroke: var(--sheet);
    stroke-width: 4px;
    stroke-linejoin: round;
    fill: var(--ink-2);
    font-weight: 500;
    font-size: 11px;
  }
</style>
