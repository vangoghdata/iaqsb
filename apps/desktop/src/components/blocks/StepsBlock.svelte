<script lang="ts">
  import type { StepsBlock } from '../../lib/gtypes';
  import BlockView from './BlockView.svelte';
  import { t } from '../../lib/i18n.svelte';

  let { s }: { s: StepsBlock } = $props();
  let k = $state(0);
  $effect(() => {
    s;
    k = 0;
  });
  const cur = $derived(s.steps[Math.min(k, s.steps.length - 1)]);
  const last = $derived(k >= s.steps.length - 1);

  function keys(e: KeyboardEvent) {
    if (e.key === 'ArrowRight') k = Math.min(k + 1, s.steps.length - 1);
    if (e.key === 'ArrowLeft') k = Math.max(k - 1, 0);
  }
</script>

{#if s.steps.length}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div class="steps" tabindex="0" onkeydown={keys} role="group" aria-label={s.title ?? t('Paso a paso')}>
    {#if s.title}<h2 class="stitle">{s.title}</h2>{/if}
    <div class="bar">
      <button class="btn" disabled={k === 0} onclick={() => (k = 0)}>{t('Inicio')}</button>
      <button class="btn" disabled={k === 0} onclick={() => k--}>{t('Anterior')}</button>
      <div class="where"><strong>{t('Paso {0}', k + 1)}</strong> <span class="muted">{t('de {0}', s.steps.length)}</span></div>
      <button class="btn" disabled={last} onclick={() => k++}>{t('Siguiente')}</button>
      <button class="btn" disabled={last} onclick={() => (k = s.steps.length - 1)}>{t('Final')}</button>
    </div>
    <ol class="dots" aria-hidden="true">
      {#each s.steps as st, i}
        <li><button class:on={i === k} class:done={i < k} title={st.title} tabindex="-1" onclick={() => (k = i)}></button></li>
      {/each}
    </ol>
    <h3>{cur.title}</h3>
    {#if cur.description}<p class="desc">{cur.description}</p>{/if}
    <div class="inner">
      {#each cur.blocks as b, i (`${k}-${i}`)}
        <BlockView block={b} />
      {/each}
    </div>
    <p class="muted small">{t('Usa las flechas ← y → para avanzar.')}</p>
  </div>
{/if}

<style>
  .steps {
    display: flex;
    flex-direction: column;
    gap: 12px;
    outline: none;
  }
  .bar {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  .where {
    min-width: 130px;
    text-align: center;
  }
  .where strong {
    font-family: var(--font-brand);
    font-size: 17px;
  }
  .dots {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .dots button {
    width: 22px;
    height: 6px;
    border: 0;
    border-radius: 3px;
    background: var(--rule);
    padding: 0;
  }
  .dots button.done {
    background: var(--solver);
    opacity: 0.45;
  }
  .dots button.on {
    background: var(--solver);
  }
  h3 {
    font-size: 18px;
    margin-top: 4px;
  }
  .desc {
    margin: 0;
    max-width: 80ch;
  }
  .inner {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .small {
    font-size: 12.5px;
    margin: 0;
  }
  .stitle {
    font-size: 17px;
    margin: 0 0 12px;
    padding-top: 6px;
    color: var(--solver);
  }
</style>
