<script lang="ts">
  import type { SourcesBlock, Citation } from '../../lib/gtypes';
  import { openExternal } from '../../lib/backend';
  import { locale, t } from '../../lib/i18n.svelte';

  let { s }: { s: SourcesBlock } = $props();
  let copied = $state('');

  /** Parte un texto con *cursivas* en trozos [texto, ¿cursiva?]. */
  const parts = (t: string) => t.split('*').map((p, i) => [p, i % 2 === 1] as const).filter(([p]) => p);
  const plain = (t: string) => t.replaceAll('*', '');

  // en la lista final, cada referencia una sola vez y en orden alfabético (como en APA)
  const all = $derived.by(() => {
    const m = new Map<string, Citation>();
    for (const c of [...(s.data?.refs ?? []), ...s.refs]) if (!m.has(c.id)) m.set(c.id, c);
    return [...m.values()].sort((a, b) => plain(a.apa).localeCompare(plain(b.apa), locale()));
  });
  const pending = $derived(all.filter((c) => !c.verified).length + (s.data && !s.data.verified ? 1 : 0));

  async function copy(what: 'apa' | 'bibtex') {
    const txt = what === 'apa' ? all.map((c) => plain(c.apa)).join('\n\n') : all.map((c) => c.bibtex).join('\n');
    try {
      await navigator.clipboard.writeText(txt);
      copied = what;
      setTimeout(() => (copied = ''), 1800);
    } catch {
      copied = '';
    }
  }
</script>

{#snippet url(u: string)}<a class="link" href={u} onclick={(e) => { e.preventDefault(); openExternal(u); }}>{u}</a>{/snippet}

{#snippet apa(t: string)}
  {#each parts(t) as [p, it]}{#if it}<em>{p}</em>{:else if p.includes('https://')}{p.slice(0, p.indexOf('https://'))}{@render url(p.slice(p.indexOf('https://')).trim())}{:else}{p}{/if}{/each}
{/snippet}

{#snippet badge(ok: boolean, why?: string)}
  <span class="badge" class:ok class:warn={!ok} title={ok ? t('Datos comprobados en el DOI, el editor o un catálogo') : (why ?? '')}>
    {ok ? t('Verificada') : t('Por verificar')}
  </span>
{/snippet}

<div class="src">
  {#if s.data}
    <section class="card">
      <div class="head">
        <h3>{t('Origen de los datos')}</h3>
        <span class="kind">{s.data.origin_label}</span>
        {@render badge(s.data.verified, s.data.pending)}
      </div>
      <p class="sum">{s.data.summary}</p>
      {#if s.data.expected}<p class="muted">{t('Resultado en la fuente:')} <strong>{s.data.expected}</strong></p>{/if}
      {#if s.data.note}<p class="muted">{s.data.note}</p>{/if}
      {#if s.data.pending}<p class="pend">{t('Por verificar: {0}', s.data.pending)}</p>{/if}
    </section>
  {/if}

  {#if s.refs.length}
    <section>
      <h3>{t('Métodos, fórmulas y pasos')}</h3>
      <ul class="uses">
        {#each s.refs as c (c.id)}
          <li>
            <div class="what">
              {#each c.used_for as u}<span>{u}</span>{/each}
            </div>
            <div class="cite">({c.cite}) {#if !c.verified}{@render badge(false, c.pending)}{/if}</div>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  <section>
    <div class="head">
      <h3>{t('Referencias')}</h3>
      <span class="muted small">{t('APA 7.ª ed.')}{#if pending}, {t('{0} por verificar', pending)}{/if}</span>
      <span class="grow"></span>
      <button class="btn" onclick={() => copy('apa')}>{copied === 'apa' ? t('Copiado') : t('Copiar en APA')}</button>
      <button class="btn" onclick={() => copy('bibtex')}>{copied === 'bibtex' ? t('Copiado') : t('Copiar BibTeX')}</button>
    </div>
    <ol class="refs">
      {#each all as c (c.id)}
        <li>
          {@render apa(c.apa)}
          {#if !c.verified} {@render badge(false, c.pending)}{#if c.pending}<span class="pend small"> {c.pending}</span>{/if}{/if}
        </li>
      {/each}
    </ol>
  </section>
</div>

<style>
  .src {
    display: flex;
    flex-direction: column;
    gap: 22px;
    max-width: 92ch;
  }
  h3 {
    font-size: 15px;
    margin: 0;
  }
  .head {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
    margin-bottom: 8px;
  }
  .grow {
    flex: 1;
  }
  .card {
    padding: 14px 16px;
    border: 0.5px solid var(--rule);
    border-radius: var(--r-lg);
    background: var(--sheet);
  }
  .card p {
    margin: 4px 0 0;
  }
  .sum {
    font-weight: 550;
  }
  .kind {
    font-size: 12px;
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--rule-soft);
    color: var(--ink-2);
  }
  .badge {
    font-size: 11.5px;
    padding: 1px 7px;
    border-radius: 999px;
    white-space: nowrap;
  }
  .badge.ok {
    background: color-mix(in srgb, #10b981 16%, transparent);
    color: color-mix(in srgb, #10b981 75%, var(--ink));
  }
  .badge.warn {
    background: var(--binding-tint);
    color: color-mix(in srgb, var(--binding) 80%, var(--ink));
  }
  .uses {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .uses li {
    display: flex;
    gap: 16px;
    justify-content: space-between;
    align-items: baseline;
    border-bottom: 1px solid var(--rule-soft);
    padding-bottom: 6px;
  }
  .what {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .cite {
    color: var(--ink-2);
    white-space: nowrap;
    font-size: 13px;
  }
  .refs {
    margin: 0;
    padding-left: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .refs li {
    padding-left: 2.2em;
    text-indent: -2.2em;
    line-height: 1.5;
  }
  .link {
    cursor: pointer;
    color: var(--solver);
    text-decoration: underline;
    text-underline-offset: 2px;
    word-break: break-all;
  }
  .pend {
    color: color-mix(in srgb, var(--binding) 80%, var(--ink));
  }
  .muted {
    color: var(--ink-2);
  }
  .small {
    font-size: 12.5px;
  }
</style>
