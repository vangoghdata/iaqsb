<script lang="ts">
  // Botón que abre o cierra el panel «¿Qué pasa si…?» de un módulo.
  import Icon from '../Icon.svelte';
  import { t } from '../../lib/i18n.svelte';
  import { genq } from '../../lib/quepasa.svelte';

  let { modulo }: { modulo: string } = $props();
  const g = $derived(genq(modulo));
</script>

<button
  class="btn"
  class:on={g.abierto}
  aria-expanded={g.abierto}
  aria-controls={`qp-gen-${modulo}`}
  onclick={() => (g.abierto = !g.abierto)}
  title={t('Varía un dato del modelo y mira cómo cambia el resultado')}
>
  <Icon name="deslizadores" size={15} />{t('¿Qué pasa si…?')}
</button>

<style>
  .on {
    color: var(--solver);
    background: var(--solver-tint);
    border-color: color-mix(in oklab, var(--solver) 40%, transparent);
  }
</style>
