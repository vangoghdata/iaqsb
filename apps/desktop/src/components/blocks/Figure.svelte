<script lang="ts">
  // Marco de una gráfica: barra con Ampliar, Copiar y Descargar (PNG o SVG),
  // y un visor a pantalla completa con zoom (rueda o botones) y arrastre.
  import type { Snippet } from 'svelte';
  import Icon from '../Icon.svelte';
  import { copyGraphic, saveGraphic } from '../../lib/backend';
  import { ui } from '../../lib/ui.svelte';
  import { t } from '../../lib/i18n.svelte';

  let { block, children }: { block: { type: string; title?: string | null }; children: Snippet } = $props();

  const nombre = $derived(block.title || t('grafica'));
  let abierto = $state(false);
  let escala = $state(1);
  let tx = $state(0);
  let ty = $state(0);
  let arrastre = $state<{ x: number; y: number; tx: number; ty: number } | null>(null);

  function abrir() {
    escala = 1;
    tx = 0;
    ty = 0;
    abierto = true;
  }
  function zoom(f: number, cx = 0, cy = 0) {
    const n = Math.min(8, Math.max(0.4, escala * f));
    // mantiene quieto el punto bajo el cursor
    tx = cx - ((cx - tx) * n) / escala;
    ty = cy - ((cy - ty) * n) / escala;
    escala = n;
  }
  function rueda(e: WheelEvent) {
    e.preventDefault();
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    zoom(e.deltaY < 0 ? 1.12 : 1 / 1.12, e.clientX - r.left - r.width / 2, e.clientY - r.top - r.height / 2);
  }
  function bajar(e: PointerEvent) {
    if (e.button !== 0) return;
    arrastre = { x: e.clientX, y: e.clientY, tx, ty };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function mover(e: PointerEvent) {
    if (!arrastre) return;
    tx = arrastre.tx + e.clientX - arrastre.x;
    ty = arrastre.ty + e.clientY - arrastre.y;
  }
  function soltar() {
    arrastre = null;
  }
  async function descargar(f: 'png' | 'svg') {
    try {
      const r = await saveGraphic($state.snapshot(block), nombre, f);
      if (r) ui.avisar(t('Gráfica guardada: {0}', r.split(/[\\/]/).pop()));
    } catch (e) {
      ui.avisar(t('No se pudo guardar la gráfica: {0}', e instanceof Error ? e.message : e), 'error');
    }
  }
  async function copiar() {
    try {
      await copyGraphic($state.snapshot(block));
      ui.avisar(t('Gráfica copiada: pégala en Word, PowerPoint o un chat'));
    } catch (e) {
      ui.avisar(t('No se pudo copiar: {0}', e instanceof Error ? e.message : e), 'error');
    }
  }
  function teclas(e: KeyboardEvent) {
    if (!abierto) return;
    if (e.key === 'Escape') abierto = false;
    else if (e.key === '+' || e.key === '=') zoom(1.2);
    else if (e.key === '-') zoom(1 / 1.2);
    else if (e.key === '0') {
      escala = 1;
      tx = ty = 0;
    }
  }
</script>

<svelte:window onkeydown={teclas} />

<div class="fig">
  <div class="tools" role="toolbar" aria-label={t('Gráfica')}>
    <button class="tb" title={t('Ampliar')} aria-label={t('Ampliar')} onclick={abrir}><Icon name="ampliar" size={14} /></button>
    <button class="tb" title={t('Copiar como imagen')} aria-label={t('Copiar como imagen')} onclick={copiar}><Icon name="copiar" size={14} /></button>
    <button class="tb txt" title={t('Descargar en PNG')} onclick={() => descargar('png')}><Icon name="descargar" size={14} />PNG</button>
    <button class="tb txt" title={t('Descargar en SVG (vectorial)')} onclick={() => descargar('svg')}>SVG</button>
  </div>
  <div class="cuerpo" role="presentation" ondblclick={abrir}>
    {@render children()}
  </div>
</div>

{#if abierto}
  <div class="lb" role="dialog" aria-modal="true" aria-label={block.title ?? t('Gráfica')}>
    <div class="lb-bar">
      <span class="lb-t">{block.title ?? ''}</span>
      <div class="lb-z">
        <button class="tb" title={t('Alejar (−)')} aria-label={t('Alejar')} onclick={() => zoom(1 / 1.2)}><Icon name="menos" size={14} /></button>
        <button class="tb pct" title={t('Tamaño original (0)')} onclick={() => ((escala = 1), (tx = 0), (ty = 0))}>{Math.round(escala * 100)} %</button>
        <button class="tb" title={t('Acercar (+)')} aria-label={t('Acercar')} onclick={() => zoom(1.2)}><Icon name="mas" size={14} /></button>
      </div>
      <div class="lb-a">
        <button class="tb" title={t('Copiar como imagen')} aria-label={t('Copiar')} onclick={copiar}><Icon name="copiar" size={14} /></button>
        <button class="tb txt" title={t('Descargar en PNG')} onclick={() => descargar('png')}><Icon name="descargar" size={14} />PNG</button>
        <button class="tb txt" title={t('Descargar en SVG (vectorial)')} onclick={() => descargar('svg')}>SVG</button>
        <button class="tb" title={t('Cerrar (Esc)')} aria-label={t('Cerrar')} onclick={() => (abierto = false)}><Icon name="cerrar" size={15} /></button>
      </div>
    </div>
    <div
      class="lb-stage"
      class:mueve={!!arrastre}
      role="presentation"
      onwheel={rueda}
      onpointerdown={bajar}
      onpointermove={mover}
      onpointerup={soltar}
      onpointercancel={soltar}
      ondblclick={() => zoom(1.5)}
    >
      <div class="lb-canvas" style:transform={`translate(${tx}px, ${ty}px) scale(${escala})`}>
        {@render children()}
      </div>
    </div>
    <p class="lb-ayuda">{t('Rueda del mouse para acercar · arrastra para mover · doble clic para ampliar · Esc para cerrar')}</p>
  </div>
{/if}

<style>
  .fig {
    position: relative;
    max-width: 900px;
  }
  .fig :global(figure) {
    max-width: none;
  }
  .tools {
    position: absolute;
    top: -4px;
    right: 0;
    z-index: 3;
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: 9px;
    background: color-mix(in oklab, var(--sheet) 82%, transparent);
    -webkit-backdrop-filter: var(--vidrio);
    backdrop-filter: var(--vidrio);
    border: 0.5px solid var(--rule);
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.12);
    opacity: 0;
    transform: translateY(-2px);
    transition: opacity 0.15s, transform 0.15s;
  }
  .fig:hover .tools,
  .fig:focus-within .tools {
    opacity: 1;
    transform: none;
  }
  .tb {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    min-width: 26px;
    height: 26px;
    padding: 0 6px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--ink-2);
    font-size: 11.5px;
    font-weight: 600;
  }
  .tb:hover {
    background: var(--rule-soft);
    color: var(--ink);
  }
  .cuerpo {
    cursor: zoom-in;
  }
  .lb {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    background: color-mix(in oklab, var(--paper) 97%, transparent);
    -webkit-backdrop-filter: var(--vidrio);
    backdrop-filter: var(--vidrio);
    animation: aparecer 0.16s ease-out;
  }
  @keyframes aparecer {
    from {
      opacity: 0;
    }
  }
  .lb-bar {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 12px;
    padding: 8px 14px;
    border-bottom: 0.5px solid var(--rule);
    background: var(--barra-bg);
  }
  .lb-t {
    font-weight: 600;
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .lb-z,
  .lb-a {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .lb-a {
    justify-content: flex-end;
  }
  .pct {
    min-width: 54px;
    font-variant-numeric: tabular-nums;
  }
  .lb-stage {
    position: relative;
    overflow: hidden;
    display: grid;
    place-items: center;
    cursor: grab;
    touch-action: none;
  }
  .lb-stage.mueve {
    cursor: grabbing;
  }
  .lb-canvas {
    width: min(1400px, 94vw);
    transform-origin: center center;
    will-change: transform;
  }
  .lb-canvas :global(figure),
  .lb-canvas :global(.chart),
  .lb-canvas :global(.net),
  .lb-canvas :global(.map),
  .lb-canvas :global(.plan),
  .lb-canvas :global(.gantt),
  .lb-canvas :global(.tree) {
    max-width: none !important;
    width: 100%;
  }
  .lb-canvas :global(svg) {
    max-height: 82vh;
  }
  .lb-ayuda {
    margin: 0;
    padding: 6px;
    text-align: center;
    font-size: 11.5px;
    color: var(--ink-3);
  }
</style>
