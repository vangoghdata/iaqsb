<script lang="ts">
  // Tema de un curso que es solo lección: sus lecciones y la entrada a su guía.
  import Icon from './Icon.svelte';
  import Nativo from './chat/Nativo.svelte';
  import Markdown from './chat/Markdown.svelte';
  import { call } from '../lib/backend';
  import { guia } from '../lib/guia.svelte';
  import { chat } from '../lib/chat.svelte';
  import { t } from '../lib/i18n.svelte';
  import type { Guia } from '../lib/guia.svelte';

  let { id }: { id: string } = $props();
  let g = $state<Guia | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    const m = id;
    g = null;
    error = null;
    call<{ guide: Guia }>('guide', { module: m })
      .then((r) => (g = r.guide))
      .catch((e) => (error = e instanceof Error ? e.message : String(e)));
  });

  const tieneGuia = $derived(!!g && !!(g.que_es?.length || g.historia?.length || g.para_que_sirve?.length));
</script>

<div class="tema">
  {#if error}
    <p class="err">{error}</p>
  {:else if g}
    <header>
      <Nativo toma="explicando" tam={88} titulo="Nativo" />
      <div class="tit">
        {#if g.curso}<span class="kicker">{g.curso.titulo}</span>{/if}
        <h1>{g.titulo}</h1>
        <p class="resumen">{g.resumen}</p>
        <div class="acc">
          {#if tieneGuia}<button class="btn primary" onclick={() => guia.abrir(id)}><Icon name="info" size={15} />{t('Ver la guía completa')}</button>{/if}
          <button class="btn" onclick={() => { chat.abrir(); chat.enviar(t('Explícame el tema «{0}» del curso «{1}»: qué es, para qué sirve y un ejemplo.', g?.titulo ?? '', g?.curso?.titulo ?? '')); }}><Icon name="chat" size={15} />{t('Pregúntale a Nativo')}</button>
        </div>
      </div>
    </header>
    {#each g.lecciones ?? [] as l, i (i)}
      <article class="leccion">
        <Markdown texto={l.markdown} />
      </article>
    {:else}
      {#if !tieneGuia}<p class="muted">{t('Este tema todavía no tiene lecciones.')}</p>{/if}
    {/each}
  {:else}
    <p class="muted">{t('Cargando el tema…')}</p>
  {/if}
</div>

<style>
  .tema {
    flex: 1;
    overflow: auto;
    padding: 24px max(24px, calc(50% - 420px)) 60px;
  }
  header {
    display: flex;
    gap: 18px;
    align-items: center;
    margin-bottom: 18px;
  }
  .kicker {
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--solver);
  }
  h1 {
    margin: 2px 0 4px;
    font-size: 24px;
  }
  .resumen {
    margin: 0 0 10px;
    color: var(--muted, #6b7280);
  }
  .acc {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .leccion {
    border: 0.5px solid var(--rule);
    border-radius: 12px;
    padding: 6px 22px 14px;
    background: var(--sheet);
    margin-bottom: 14px;
    font-size: 14.5px;
    line-height: 1.6;
  }
  .err {
    color: var(--bad, #c0392b);
  }
</style>
