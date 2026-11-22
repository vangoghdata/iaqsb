<script module lang="ts">
  // vista elegida (tablas o editor visual) por módulo y tipo de problema
  const recordadas = new Map<string, 'tablas' | 'visual'>();
</script>

<script lang="ts">
  import { shown, type Field, type ProblemType, type Report, type Widget } from '../../lib/gtypes';
  import { detectarGrafo, detectarMapa } from '../../lib/grafo';
  import { t } from '../../lib/i18n.svelte';
  import FieldInput from './FieldInput.svelte';
  import MatrixGrid from './MatrixGrid.svelte';
  import RecordsGrid from './RecordsGrid.svelte';
  import NetworkEditor from './NetworkEditor.svelte';
  import MapaRuteo from './MapaRuteo.svelte';

  let {
    pt,
    model,
    onchange,
    modulo = '',
    report = null,
    stale = false
  }: { pt: ProblemType; model: any; onchange: () => void; modulo?: string; report?: Report | null; stale?: boolean } = $props();

  function visible(f: Field, obj: any) {
    return shown(f, (k) => obj?.[k] ?? model[k]);
  }

  // editor visual: red de nodos y arcos, o mapa real si los puntos están en latitud y longitud
  const grafo = $derived(detectarGrafo(modulo, pt));
  const mapa = $derived(detectarMapa(pt, model));
  const clave = $derived(`${modulo}/${pt.id}`);
  let elegida = $state<{ clave: string; v: 'tablas' | 'visual' } | null>(null);
  const vista = $derived.by(() => {
    if (!grafo && !mapa) return 'tablas';
    if (elegida?.clave === clave) return elegida.v;
    return recordadas.get(clave) ?? (mapa ? 'visual' : 'tablas');
  });
  function elegir(v: 'tablas' | 'visual') {
    recordadas.set(clave, v);
    elegida = { clave, v };
  }
  // el editor visual va después de los parámetros
  const despues = $derived.by(() => {
    const k = pt.layout.findIndex((w) => w.type !== 'params');
    return k < 0 ? pt.layout.length : k;
  });
  /** Hojas que el editor visual reemplaza (la matriz de arcos y los datos de los nodos). */
  function oculta(w: Widget): boolean {
    if (vista !== 'visual' || !grafo) return false;
    if (w.type === 'matrix') return grafo.matrices.includes(w) && (w.rows.path === grafo.matriz.rows.path || w.rows.path === grafo.matriz.cols.path);
    if (w.type === 'records') return grafo.registros.includes(w);
    return false;
  }
</script>

{#snippet visual()}
  <section class="visual">
    {#if grafo}
      <NetworkEditor g={grafo} {model} {modulo} {onchange} {report} {stale} />
    {:else if mapa}
      <MapaRuteo w={mapa} {model} {onchange} {report} {stale} />
    {/if}
  </section>
{/snippet}

<div class="editor">
  {#if grafo || mapa}
    <div class="vistas" role="group" aria-label={t('Vista de los datos')}>
      <button class:on={vista === 'tablas'} aria-pressed={vista === 'tablas'} onclick={() => elegir('tablas')}>{t('Tablas')}</button>
      <button class:on={vista === 'visual'} aria-pressed={vista === 'visual'} onclick={() => elegir('visual')}>{mapa ? t('Mapa') : t('Editor visual')}</button>
    </div>
  {/if}
  {#each pt.layout as w, k (k)}
    {#if vista === 'visual' && k === despues}{@render visual()}{/if}
    {#if oculta(w)}
      <!-- en el editor visual -->
    {:else if w.type === 'params'}
      {@const obj = w.path ? model[w.path] : model}
      <section class="params">
        {#if w.title}<h3 class="wt">{w.title}</h3>{/if}
        <div class="pgrid">
          {#each w.fields.filter((f) => visible(f, obj)) as f (f.key)}
            <label class="pf" class:inline={f.kind.type === 'bool'} class:wide={f.kind.type === 'expr'}>
              <span class="pl">{f.label}</span>
              <FieldInput field={f} value={obj[f.key]} onchange={(v) => { obj[f.key] = v; onchange(); }} />
              {#if f.help}<span class="ph">{f.help}</span>{/if}
            </label>
          {/each}
        </div>
      </section>
    {:else if w.type === 'matrix'}
      <section><MatrixGrid {w} {model} {onchange} siblings={pt.layout.filter((x) => x.type === 'matrix') as any} /></section>
    {:else if w.type === 'records'}
      <section><RecordsGrid {w} {model} {onchange} /></section>
    {/if}
  {/each}
  {#if vista === 'visual' && despues >= pt.layout.length}{@render visual()}{/if}
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: 22px;
  }
  .vistas {
    display: inline-flex;
    align-self: flex-start;
    padding: 2px;
    gap: 2px;
    background: var(--sheet-2);
    border: 1px solid var(--rule);
    border-radius: var(--r-sm);
    margin-bottom: -8px;
  }
  .vistas button {
    border: 0;
    background: none;
    padding: 4px 14px;
    border-radius: 5px;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--ink-2);
  }
  .vistas button.on {
    background: var(--sheet);
    color: var(--ink);
    box-shadow: var(--shadow);
  }
  .wt {
    font-size: 15px;
    margin: 0 0 8px;
  }
  .pgrid {
    display: flex;
    flex-wrap: wrap;
    gap: 14px 22px;
    align-items: flex-end;
  }
  .pf {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 180px;
    max-width: 320px;
  }
  .pf.wide {
    flex: 1 1 520px;
    max-width: 860px;
  }
  .pf.wide :global(.inp) {
    font-family: var(--font-mono, ui-monospace, monospace);
  }
  .pf.inline {
    flex-direction: row-reverse;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    min-width: 0;
    padding-bottom: 7px;
  }
  .pl {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--ink-2);
  }
  .ph {
    font-size: 12px;
    color: var(--ink-3);
  }
</style>
