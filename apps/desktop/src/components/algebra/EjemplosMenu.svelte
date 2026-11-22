<script lang="ts">
  // Menú «Ejemplos» del modelo algebraico (todos originales de IAQSB).
  import Icon from '../Icon.svelte';
  import { EJEMPLOS_ALGEBRA } from '../../lib/algebra-ejemplos';
  import { t } from '../../lib/i18n.svelte';

  let { onpick, actual = null }: { onpick: (id: string) => void; actual?: string | null } = $props();

  let abierto = $state(false);
  let caja: HTMLElement | undefined = $state();
  let pos = $state({ top: 0, left: 0 });

  function alternar(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    pos = { top: r.bottom + 6, left: Math.max(8, Math.min(r.left, window.innerWidth - 420)) };
    abierto = !abierto;
  }
  function fuera(e: MouseEvent) {
    if (abierto && caja && !caja.contains(e.target as Node)) abierto = false;
  }
  function elegir(id: string) {
    abierto = false;
    onpick(id);
  }
</script>

<svelte:window onclick={fuera} onkeydown={(e) => e.key === 'Escape' && (abierto = false)} />

<div class="em" bind:this={caja}>
  <button class="btn" class:on={abierto} aria-haspopup="menu" aria-expanded={abierto} onclick={alternar}>
    <Icon name="libro" size={15} />{t('Ejemplos del lenguaje')}<Icon name="abajo" size={13} />
  </button>
  {#if abierto}
    <div class="popover menu" role="menu" aria-label={t('Ejemplos del lenguaje')} style:top={`${pos.top}px`} style:left={`${pos.left}px`}>
      {#each EJEMPLOS_ALGEBRA as ex (ex.id)}
        <button role="menuitem" class="it" class:on={actual === ex.id} onclick={() => elegir(ex.id)}>
          <span class="ti">{t(ex.titulo)}</span>
          <span class="re">{t(ex.resumen)}</span>
        </button>
      {/each}
      <p class="pie"><Icon name="info" size={13} />{t('Original de IAQSB: modelos y datos ilustrativos, no tomados de libros.')}</p>
    </div>
  {/if}
</div>

<style>
  .em {
    position: relative;
  }
  .btn.on {
    background: var(--rule-soft);
  }
  .menu {
    position: fixed;
    width: min(410px, 92vw);
    padding: 6px;
    display: grid;
    gap: 2px;
  }
  .it {
    display: grid;
    gap: 1px;
    text-align: left;
    border: 0;
    background: none;
    border-radius: var(--r-sm);
    padding: 7px 10px;
  }
  .it:hover,
  .it:focus-visible {
    background: var(--menu-hover);
    outline: none;
  }
  .it.on .ti::after {
    content: ' ✓';
    color: var(--solver);
  }
  .ti {
    font-weight: 600;
    font-size: 13px;
  }
  .re {
    font-size: 12px;
    color: var(--ink-2);
  }
  .pie {
    display: flex;
    gap: 6px;
    align-items: center;
    margin: 4px 10px 4px;
    padding-top: 7px;
    border-top: 0.5px solid var(--rule);
    font-size: 11.5px;
    color: var(--ink-3);
  }
</style>
