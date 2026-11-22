<script lang="ts">
  // Botones de deshacer y rehacer (Ctrl+Z, Ctrl+Y).
  import Icon from './Icon.svelte';
  import { t } from '../lib/i18n.svelte';
  import type { Historial } from '../lib/historial.svelte';

  let { historial, deshacer, rehacer }: { historial: Historial; deshacer: () => void; rehacer: () => void } = $props();
</script>

<div class="ur" role="group" aria-label={t('Deshacer y rehacer')}>
  <button class="btn ghost icon" onclick={deshacer} disabled={!historial.puedeDeshacer} title={t('Deshacer (Ctrl+Z)')} aria-label={t('Deshacer')}><Icon name="deshacer" size={16} /></button>
  <button class="btn ghost icon" onclick={rehacer} disabled={!historial.puedeRehacer} title={t('Rehacer (Ctrl+Y)')} aria-label={t('Rehacer')}><Icon name="rehacer" size={16} /></button>
</div>

<style>
  .ur {
    display: inline-flex;
    gap: 2px;
  }
</style>
