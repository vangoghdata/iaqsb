<script lang="ts">
  // Diálogo breve al pegar un bloque más grande que la hoja: crecer, usar la
  // última columna como lado derecho o pegar solo lo que cabe. Lo monta lib/excel.ts.
  import { onMount } from 'svelte';
  import Icon from './Icon.svelte';
  import { t } from '../lib/i18n.svelte';
  import type { Opcion } from '../lib/excel';

  let { titulo, texto, opciones, cerrar }: { titulo: string; texto: string; opciones: Opcion[]; cerrar: (id: string | null) => void } = $props();

  let primera: HTMLButtonElement | undefined = $state();
  let previo: Element | null = null;

  onMount(() => {
    previo = document.activeElement;
    primera?.focus();
  });

  function fin(id: string | null) {
    // el foco vuelve a la celda donde se pegó
    if (previo instanceof HTMLElement) previo.focus({ preventScroll: true });
    cerrar(id);
  }

  function teclas(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      fin(null);
    }
  }
</script>

<svelte:window onkeydown={teclas} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" onclick={(e) => e.target === e.currentTarget && fin(null)}>
  <div class="dialog caja" role="dialog" aria-modal="true" aria-labelledby="pegar-tit">
    <div class="cab">
      <span class="ico" aria-hidden="true"><Icon name="tabla" size={18} /></span>
      <div>
        <h2 id="pegar-tit">{titulo}</h2>
        <p class="muted">{texto}</p>
      </div>
    </div>
    <div class="ops">
      {#each opciones as o, k (o.id)}
        {#if k === 0}
          <button class="op principal" bind:this={primera} onclick={() => fin(o.id)}>
            <span class="et">{o.etiqueta}</span>
            {#if o.detalle}<span class="de">{o.detalle}</span>{/if}
          </button>
        {:else}
          <button class="op" onclick={() => fin(o.id)}>
            <span class="et">{o.etiqueta}</span>
            {#if o.detalle}<span class="de">{o.detalle}</span>{/if}
          </button>
        {/if}
      {/each}
    </div>
    <div class="pie">
      <button class="btn ghost" onclick={() => fin(null)}>{t('Cancelar')}</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    z-index: 300;
  }
  .caja {
    width: min(460px, 100%);
    padding: 20px 20px 14px;
  }
  .cab {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    margin-bottom: 14px;
  }
  .ico {
    flex: none;
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 9px;
    background: var(--solver-tint);
    color: var(--solver);
  }
  h2 {
    font-size: 16px;
    margin: 0 0 4px;
  }
  .cab p {
    margin: 0;
    font-size: 13px;
    line-height: 1.45;
  }
  .ops {
    display: grid;
    gap: 8px;
  }
  .op {
    display: grid;
    gap: 2px;
    text-align: left;
    padding: 10px 12px;
    border: 1px solid var(--rule);
    border-radius: var(--r-md);
    background: var(--rule-soft);
    color: var(--ink);
  }
  .op:hover {
    background: var(--rule);
  }
  .op:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }
  .op.principal {
    border-color: color-mix(in oklab, var(--accent-btn) 55%, transparent);
    background: var(--solver-tint);
  }
  .op.principal .et {
    color: var(--solver);
  }
  .et {
    font-weight: 650;
    font-size: 13.5px;
  }
  .de {
    font-size: 12px;
    color: var(--ink-2);
  }
  .pie {
    display: flex;
    justify-content: flex-end;
    margin-top: 10px;
  }
</style>
