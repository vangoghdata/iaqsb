<script lang="ts">
  // Pestaña «Práctica» del módulo LP-ILP: el estudiante resuelve a mano y el
  // programa revisa cada decisión. Símplex de tablas y, en modelos enteros,
  // branch and bound.
  import { app } from '../../lib/state.svelte';
  import { practica } from '../../lib/practica_sesion.svelte';
  import { t } from '../../lib/i18n.svelte';
  import SimplexPractica from './SimplexPractica.svelte';
  import BnbPractica from './BnbPractica.svelte';

  const entero = $derived(app.isInteger);
  $effect(() => {
    if (!entero && practica.modo === 'bnb') practica.modo = 'simplex';
  });
</script>

<div class="pv">
  <header class="cab">
    <div>
      <h2>{t('Modo práctica')}</h2>
      <p class="muted">
        {practica.modo === 'simplex'
          ? t('Resuelve el problema con el método símplex (gran M) y el programa revisa cada decisión.')
          : t('Recorre el árbol de branch and bound: en cada nodo decides si se poda o se ramifica.')}
      </p>
    </div>
    {#if entero}
      <div class="seg" role="radiogroup" aria-label={t('Qué practicar')}>
        <button role="radio" aria-checked={practica.modo === 'simplex'} class:on={practica.modo === 'simplex'} onclick={() => (practica.modo = 'simplex')}>{t('Símplex (relajación lineal)')}</button>
        <button role="radio" aria-checked={practica.modo === 'bnb'} class:on={practica.modo === 'bnb'} onclick={() => (practica.modo = 'bnb')}>{t('Branch and bound')}</button>
      </div>
    {/if}
  </header>
  {#if practica.modo === 'bnb' && entero}
    <BnbPractica />
  {:else}
    {#if entero}<p class="muted small">{t('Las variables enteras se tratan como continuas: es la relajación lineal, el primer nodo de branch and bound.')}</p>{/if}
    <SimplexPractica />
  {/if}
</div>

<style>
  .pv {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .cab {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px 20px;
    flex-wrap: wrap;
  }
  h2 {
    margin: 0 0 2px;
    font-size: 18px;
  }
  .cab p {
    margin: 0;
    font-size: 13px;
  }
  .small {
    font-size: 12.5px;
    margin: 0;
  }
</style>
