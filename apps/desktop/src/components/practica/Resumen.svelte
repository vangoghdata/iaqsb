<script lang="ts">
  // Marcador de la práctica: decisiones, aciertos, pistas.
  import type { Marcador } from '../../lib/practica_sesion.svelte';
  import { t } from '../../lib/i18n.svelte';

  let { m, pasos, pasosLabel }: { m: Marcador; pasos?: number; pasosLabel?: string } = $props();
</script>

<dl class="marc">
  {#if pasos !== undefined}
    <div><dt>{pasosLabel ?? t('Iteraciones')}</dt><dd>{pasos}</dd></div>
  {/if}
  <div><dt>{t('Decisiones')}</dt><dd>{m.decisiones}</dd></div>
  <div><dt>{t('Aciertos / intentos')}</dt><dd>{m.aciertos} / {m.intentos}</dd></div>
  <div><dt>{t('Al primer intento')}</dt><dd>{m.primera}</dd></div>
  <div><dt>{t('Pistas')}</dt><dd>{m.pistas}</dd></div>
  <div><dt>{t('Respuestas mostradas')}</dt><dd>{m.respuestas}</dd></div>
</dl>

<style>
  .marc {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 0;
  }
  .marc div {
    min-width: 104px;
    padding: 8px 12px;
    border-radius: var(--r-sm);
    background: var(--rule-soft);
    border: 1px solid var(--rule-soft);
  }
  dt {
    font-size: 11.5px;
    color: var(--ink-2);
  }
  dd {
    margin: 2px 0 0;
    font-family: var(--font-brand);
    font-size: 19px;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
  }
</style>
