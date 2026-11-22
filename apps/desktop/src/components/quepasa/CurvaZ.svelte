<script lang="ts">
  // «Z según el parámetro»: curva exacta por tramos (análisis paramétrico) o
  // muestreada (modelos enteros), con los puntos de quiebre y la tabla de tramos.
  import BlockView from '../blocks/BlockView.svelte';
  import Icon from '../Icon.svelte';
  import { num, parseNum } from '../../lib/format';
  import { t } from '../../lib/i18n.svelte';
  import type { ChartBlock, Series } from '../../lib/gtypes';
  import { textoSolucion, zEn, type Curva, type ParamLp } from '../../lib/quepasa.svelte';

  let {
    p,
    curva,
    cargando,
    error,
    valor,
    from,
    to,
    onrango,
    onsugerido
  }: {
    p: ParamLp;
    curva: Curva | null;
    cargando: boolean;
    error: string | null;
    valor: number;
    from: number;
    to: number;
    onrango: (from: number, to: number) => void;
    onsugerido: () => void;
  } = $props();

  const estadoTxt: Record<string, string> = {
    optimal: 'Óptimo',
    infeasible: 'No factible',
    unbounded: 'No acotado',
    feasible: 'Factible',
    error: 'Error',
    node_limit: 'Límite de nodos',
    iteration_limit: 'Límite de iteraciones',
    time_limit: 'Se acabó el tiempo'
  };
  const est = (s: string) => t(estadoTxt[s] ?? s);

  const chart = $derived.by((): ChartBlock | null => {
    if (!curva) return null;
    const series: Series[] = [{ name: curva.exacta ? t('Z óptimo') : t('Z óptimo (muestreado)'), kind: 'line', tone: 'accent', points: curva.pts }];
    const zb = zEn(curva, p.base);
    if (zb !== null) series.push({ name: t('Valor original ({0})', num(p.base)), kind: 'scatter', tone: 'muted', points: [[p.base, zb]] });
    if (Math.abs(valor - p.base) > 1e-12) {
      const zv = zEn(curva, valor);
      if (zv !== null) series.push({ name: t('Escenario ({0})', num(valor)), kind: 'scatter', tone: 'good', points: [[valor, zv]] });
    }
    return {
      type: 'chart',
      title: t('Z según {0}', p.label),
      x_label: p.label,
      y_label: 'Z',
      series,
      vlines: curva.quiebres.map((q) => ({ value: q, label: curva.exacta ? num(q) : `≈ ${num(q, 2)}`, tone: 'highlight' as const }))
    };
  });

  // casillas del rango: se aplican al salir o con Intro
  let desde = $state('');
  let hasta = $state('');
  $effect(() => {
    desde = num(from, 6);
    hasta = num(to, 6);
  });
  function aplicar() {
    const a = parseNum(desde);
    const b = parseNum(hasta);
    if (a === null || b === null || desde.trim() === '' || hasta.trim() === '' || a === b) {
      desde = num(from, 6);
      hasta = num(to, 6);
      return;
    }
    onrango(Math.min(a, b), Math.max(a, b));
  }
  const enter = (e: KeyboardEvent) => e.key === 'Enter' && aplicar();
  const pendienteTxt = $derived(p.kind === 'cost' ? t('Pendiente (= valor de {0})', p.name) : t('Pendiente (= precio sombra)'));
</script>

<div class="curva">
  <div class="cc">
    <div class="rango" role="group" aria-label={t('Rango de la gráfica')}>
      <label><span>{t('Desde')}</span><input inputmode="decimal" bind:value={desde} onchange={aplicar} onkeydown={enter} /></label>
      <label><span>{t('Hasta')}</span><input inputmode="decimal" bind:value={hasta} onchange={aplicar} onkeydown={enter} /></label>
      <button class="btn ghost" onclick={onsugerido} title={t('El rango del deslizador: el de sensibilidad más un margen')}><Icon name="recargar" size={14} />{t('Rango sugerido')}</button>
    </div>
    {#if cargando}<span class="cargando" role="status"><span class="spinner" aria-hidden="true"></span>{t('Calculando la curva…')}</span>{/if}
  </div>

  {#if error}
    <p class="err" role="alert"><Icon name="alerta" size={15} />{error}</p>
  {:else if curva && !curva.pts.some((x) => Number.isFinite(x[1]))}
    <p class="aviso"><Icon name="alerta" size={15} />{t('En ningún punto de este rango el problema tiene solución óptima. Prueba con otro rango.')}</p>
  {:else if chart && curva}
    <div class="graf" class:atenuada={cargando}>
      <BlockView block={chart} />
    </div>
    <p class="nota">
      {#if curva.exacta}
        {t('Curva exacta: Z es lineal por tramos y cambia de pendiente justo donde cambia la base óptima (los puntos de quiebre marcados).')}
        {p.kind === 'cost'
          ? t('Para un costo, la pendiente de cada tramo es el valor de la variable en la solución de ese tramo.')
          : t('Para un lado derecho, la pendiente de cada tramo es el precio sombra de la restricción en ese tramo.')}
      {:else}
        {t('Modelo entero: no hay rangos de sensibilidad, así que la curva se muestrea en {0} puntos, cada uno resuelto por completo. Entre dos puntos puede haber cambios que no se ven.', curva.pts.filter((x) => Number.isFinite(x[1])).length)}
      {/if}
    </p>

    {#if curva.exacta}
      <div class="tw">
        <table>
          <caption class="sr">{t('Tramos de Z según {0}', p.label)}</caption>
          <thead>
            <tr>
              <th>{t('Desde')}</th>
              <th>{t('Hasta')}</th>
              <th>{pendienteTxt}</th>
              <th>{t('Z al inicio')}</th>
              <th>{t('Z al final')}</th>
              <th class="l" title={p.kind === 'rhs' ? t('Con un lado derecho los valores cambian dentro del tramo; se muestran los del inicio') : ''}>{p.kind === 'cost' ? t('Solución en el tramo') : t('Solución al inicio del tramo')}</th>
            </tr>
          </thead>
          <tbody>
            {#each curva.tramos as s, k (k)}
              {@const aqui = valor >= s.from - 1e-9 && valor <= s.to + 1e-9}
              <tr class:aqui class:mal={s.status !== 'optimal'}>
                <td>{num(s.from)}</td>
                <td>{num(s.to)}</td>
                {#if s.status === 'optimal'}
                  <td class="b">{num(s.slope)}</td>
                  <td>{num(s.objective_from)}</td>
                  <td>{num(s.objective_to)}</td>
                  <td class="l sol">{textoSolucion(s.solution, (v) => num(v))}</td>
                {:else}
                  <td colspan="4" class="l">{est(s.status)}</td>
                {/if}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else if curva.barrido}
      {#if curva.barrido.solution_changes.length}
        <div class="tw">
          <table>
            <caption class="sr">{t('Cambios de solución')}</caption>
            <thead>
              <tr>
                <th class="l">{t('Cambia entre')}</th>
                <th class="l">{t('Variables positivas antes')}</th>
                <th class="l">{t('Variables positivas después')}</th>
              </tr>
            </thead>
            <tbody>
              {#each curva.barrido.solution_changes as c, k (k)}
                <tr>
                  <td class="l">{num(c.between[0])} – {num(c.between[1])}</td>
                  <td class="l sol">{c.before.join(', ') || '–'}</td>
                  <td class="l sol">{c.after.join(', ') || '–'}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else}
        <p class="nota">{t('En todo el rango la solución tiene las mismas variables positivas.')}</p>
      {/if}
      <details class="puntos">
        <summary>{t('Ver los {0} puntos', curva.barrido.points.length)}</summary>
        <div class="tw alto">
          <table>
            <thead>
              <tr><th>{p.label}</th><th class="l">{t('Estado')}</th><th>Z</th><th class="l">{t('Solución')}</th></tr>
            </thead>
            <tbody>
              {#each curva.barrido.points as q, k (k)}
                <tr class:mal={q.status !== 'optimal' && q.status !== 'feasible'}>
                  <td>{num(q.value)}</td>
                  <td class="l">{est(q.status)}</td>
                  <td>{num(q.metric)}</td>
                  <td class="l sol">{textoSolucion(q.solution, (v) => num(v))}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </details>
    {/if}
  {:else if !cargando}
    <p class="muted">{t('Elige un coeficiente para ver cómo cambia Z.')}</p>
  {/if}
</div>

<style>
  .curva {
    display: grid;
    gap: 12px;
    min-width: 0;
  }
  .cc {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px 16px;
  }
  .rango {
    display: flex;
    align-items: flex-end;
    flex-wrap: wrap;
    gap: 10px;
  }
  .rango label {
    display: grid;
    gap: 3px;
    font-size: 12px;
    color: var(--ink-2);
  }
  .rango input {
    width: 96px;
    height: 30px;
    padding: 0 8px;
    text-align: right;
    border: 1px solid var(--rule);
    border-radius: var(--r-xs);
    background: var(--field);
    font-variant-numeric: tabular-nums;
  }
  .cargando {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    color: var(--ink-2);
  }
  .graf {
    transition: opacity 0.15s;
  }
  .graf.atenuada {
    opacity: 0.55;
  }
  .nota {
    margin: 0;
    font-size: 12.5px;
    color: var(--ink-2);
    max-width: 90ch;
  }
  .aviso {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding: 10px 14px;
    border-radius: var(--r-md);
    background: var(--binding-tint);
  }
  .aviso :global(svg) {
    color: var(--binding);
    flex: none;
  }
  .err {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    color: var(--bad);
  }
  .tw {
    width: max-content;
    max-width: 100%;
    overflow: auto;
    background: var(--sheet);
    border: 1px solid var(--rule);
    border-radius: var(--r-md);
  }
  .tw.alto {
    max-height: 320px;
  }
  th,
  td {
    text-align: right;
    padding: 6px 12px;
    border-bottom: 1px solid var(--rule-soft);
    white-space: nowrap;
  }
  th {
    position: sticky;
    top: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--ink-2);
    background: var(--sheet-2);
    border-bottom: 1px solid var(--rule);
  }
  .l {
    text-align: left;
  }
  .b {
    font-weight: 700;
  }
  .sol {
    white-space: normal;
    min-width: 200px;
    max-width: 460px;
  }
  tr.aqui td {
    background: var(--solver-tint);
  }
  tr.aqui td:first-child {
    box-shadow: inset 3px 0 0 var(--solver);
  }
  tr.mal td {
    color: var(--bad);
  }
  .puntos summary {
    cursor: pointer;
    font-size: 12.5px;
    color: var(--ink-2);
    margin-bottom: 8px;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
</style>
