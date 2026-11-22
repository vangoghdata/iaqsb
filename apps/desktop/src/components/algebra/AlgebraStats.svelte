<script lang="ts">
  // Tamaño y forma de un modelo algebraico: variables, enteras, restricciones,
  // conjuntos con su tamaño, familias de variables y reformulaciones hechas.
  import { agruparNotas, type AlgStats } from '../../lib/algebra.svelte';
  import { t } from '../../lib/i18n.svelte';

  let { s, notas = true, compacto = false }: { s: AlgStats; notas?: boolean; compacto?: boolean } = $props();

  const n = (x: number) => x.toLocaleString();
  const grupos = $derived(agruparNotas(s.reformulations ?? []));
  const continuas = $derived(s.variables - s.integer);
  // las familias con índices van una por una; las variables sueltas, si son muchas, en un solo chip
  const MAX_SUELTAS = 10;
  const conIdx = $derived(s.families.filter((f) => f.indices.length > 0));
  const sueltas = $derived(s.families.filter((f) => f.indices.length === 0));
</script>

<div class="st" class:compacto>
  <dl class="cifras">
    <div><dt>{t('Variables')}</dt><dd>{n(s.variables)}</dd></div>
    <div title={t('Enteras o binarias')}><dt>{t('Enteras')}</dt><dd>{n(s.integer)}</dd></div>
    {#if !compacto}<div><dt>{t('Continuas')}</dt><dd>{n(continuas)}</dd></div>{/if}
    <div><dt>{t('Restricciones')}</dt><dd>{n(s.constraints)}</dd></div>
    <div title={t('Coeficientes distintos de cero')}><dt>{t('No ceros')}</dt><dd>{n(s.nonzeros)}</dd></div>
    {#if s.aux > 0}<div title={t('Variables que agregó la reformulación')}><dt>{t('Auxiliares')}</dt><dd>{n(s.aux)}</dd></div>{/if}
    {#if s.objectives > 1}<div title={t('Se optimizan en orden de prioridad')}><dt>{t('Objetivos')}</dt><dd>{s.objectives}</dd></div>{/if}
    {#if s.quadratic}<div><dt>{t('Objetivo')}</dt><dd class="q">{t('cuadrático')}</dd></div>{/if}
  </dl>

  {#if s.sets.length || s.families.length}
    <div class="filas">
      {#if s.sets.length}
        <div class="fila">
          <span class="lab">{t('Conjuntos')}</span>
          <span class="chips">
            {#each s.sets as c (c.name)}<span class="chip set"><b>{c.name}</b> {n(c.size)}</span>{/each}
          </span>
        </div>
      {/if}
      {#if s.families.length}
        <div class="fila">
          <span class="lab">{t('Variables')}</span>
          <span class="chips">
            {#each conIdx as f (f.name)}
              <span class="chip var"><b>{f.name}</b>[{f.indices.map((x) => x || '·').join(', ')}]<span class="cnt">{' ' + n(f.count)}</span></span>
            {/each}
            {#if sueltas.length > MAX_SUELTAS}
              <span class="chip var" title={sueltas.map((f) => f.name).join(', ')}>{t('{0} sin índices', n(sueltas.length))}</span>
            {:else}
              {#each sueltas as f (f.name)}<span class="chip var"><b>{f.name}</b></span>{/each}
            {/if}
          </span>
        </div>
      {/if}
    </div>
  {/if}

  {#if notas && grupos.length}
    <div class="ref">
      <p class="lab">{s.aux > 0 ? t('Reformulaciones') : t('Notas')}</p>
      {#if !compacto && s.aux > 0}
        <p class="muted expl">
          {t('Las restricciones generales (abs, max, min, pwl, lógicas, indicadoras, disyunciones) se convierten en restricciones lineales con variables auxiliares, casi siempre binarias, y un big-M que sale de las cotas de las variables.')}
        </p>
      {/if}
      <ul>
        {#each grupos as g, i (i)}
          <li>
            {#if g.familia}<code>{g.familia}</code>{g.veces > 1 ? ' ' + t('({0} restricciones)', g.veces) : ''}{': '}{/if}{g.texto}
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>

<style>
  .st {
    display: grid;
    gap: 10px;
    font-size: 12.5px;
  }
  .cifras {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 22px;
    margin: 0;
  }
  .cifras div {
    display: grid;
    gap: 1px;
  }
  dt {
    font-size: 11px;
    color: var(--ink-3);
    font-weight: 550;
  }
  dd {
    margin: 0;
    font-size: 16px;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.01em;
  }
  .compacto dd {
    font-size: 14px;
  }
  dd.q {
    font-size: 13px;
  }
  .filas {
    display: grid;
    gap: 6px;
  }
  .fila {
    display: flex;
    gap: 10px;
    align-items: baseline;
  }
  .lab {
    flex: none;
    width: 92px;
    margin: 0;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--ink-2);
  }
  .compacto .lab {
    width: auto;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }
  .chip {
    display: inline-flex;
    align-items: baseline;
    gap: 2px;
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--rule-soft);
    border: 0.5px solid var(--rule);
    font-family: ui-monospace, 'SF Mono', 'Cascadia Mono', Menlo, Consolas, monospace;
    font-size: 11.5px;
    color: var(--ink-2);
  }
  .chip b {
    font-weight: 650;
    color: var(--ink);
  }
  .chip.set b {
    color: var(--accent-2);
  }
  .chip.var b {
    color: var(--binding);
  }
  .cnt {
    color: var(--ink-3);
    font-variant-numeric: tabular-nums;
  }
  .ref {
    display: grid;
    gap: 4px;
  }
  .ref .lab {
    width: auto;
  }
  .expl {
    margin: 0;
    max-width: 80ch;
    font-size: 12px;
  }
  .ref ul {
    margin: 0;
    padding-left: 18px;
    display: grid;
    gap: 2px;
    max-height: 160px;
    overflow: auto;
  }
  .ref code {
    font-family: ui-monospace, 'SF Mono', 'Cascadia Mono', Menlo, Consolas, monospace;
    font-size: 11.5px;
    font-weight: 600;
  }
</style>
