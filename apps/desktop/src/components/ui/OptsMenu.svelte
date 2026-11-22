<script lang="ts">
  // Botón «Opciones» con un panel flotante: las opciones del cálculo no ocupan
  // espacio de la pantalla hasta que se necesitan.
  import type { Snippet } from 'svelte';
  import Icon from '../Icon.svelte';
  import { t } from '../../lib/i18n.svelte';

  let { label, resumen = '', children }: { label?: string; resumen?: string; children: Snippet } = $props();
  const lab = $derived(label ?? t('Opciones'));
  let abierto = $state(false);
  let caja: HTMLElement | undefined = $state();
  // el panel va en posición fija: la barra de pestañas recorta lo que se sale de ella
  let pos = $state({ top: 0, right: 0 });
  function alternar(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    pos = { top: r.bottom + 6, right: Math.max(8, window.innerWidth - r.right) };
    abierto = !abierto;
  }

  function fuera(e: MouseEvent) {
    if (abierto && caja && !caja.contains(e.target as Node)) abierto = false;
  }
</script>

<svelte:window onclick={fuera} onkeydown={(e) => e.key === 'Escape' && (abierto = false)} />

<div class="om" bind:this={caja}>
  <button class="btn ghost om-b" class:on={abierto} aria-haspopup="dialog" aria-expanded={abierto} onclick={alternar} title={resumen || lab}>
    <Icon name="ajustes" size={15} />
    <span class="om-l">{lab}</span>
    {#if resumen}<span class="om-r">{resumen}</span>{/if}
    <Icon name="abajo" size={13} />
  </button>
  {#if abierto}
    <div class="popover om-p" role="dialog" aria-label={lab} style:top={`${pos.top}px`} style:right={`${pos.right}px`}>
      {@render children()}
    </div>
  {/if}
</div>

<style>
  .om {
    position: relative;
    flex: none;
  }
  .om-b {
    height: 26px;
    padding: 0 8px;
    gap: 5px;
    font-size: 12.5px;
  }
  .om-b.on {
    background: var(--rule-soft);
    color: var(--ink);
  }
  .om-r {
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--ink-3);
    font-weight: 450;
  }
  .om-p {
    position: fixed;
    text-align: left;
    min-width: 280px;
    max-width: min(440px, 90vw);
    max-height: 70vh;
    overflow: auto;
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  @media (max-width: 700px) {
    .om-l {
      display: none;
    }
  }
</style>
