<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { bound, num } from '../lib/format';
  import Icon from './Icon.svelte';
  import SourcesView from './blocks/SourcesView.svelte';
  import EngineLine from './EngineLine.svelte';
  import ExplicarIA from './ExplicarIA.svelte';
  import ExportMenu from './ExportMenu.svelte';
  import { t } from '../lib/i18n.svelte';
  import { copiable } from '../lib/excel';
  import AlgebraSolucion from './algebra/AlgebraSolucion.svelte';
  import type { AlgReport } from '../lib/algebra.svelte';

  const r = $derived(app.report!);
  // modelo algebraico: tablas por índices; sus notas de reformulación van agrupadas allí
  const alg = $derived((app.report as AlgReport | null)?.algebra ? (app.report as AlgReport) : null);
  const notas = $derived(alg?.algebra ? (r.notes ?? []).filter((n) => !alg.algebra!.reformulations.includes(n)) : (r.notes ?? []));
  const fuentes = $derived(r.sections?.find((x) => x.id === 'fuentes')?.blocks[0]);
  const dirLabel = { '<=': '≤', '>=': '≥', '=': '=' } as const;
  const statusText = $derived({
    optimal: t('Óptimo'),
    infeasible: t('No factible'),
    unbounded: t('No acotado'),
    iteration_limit: t('Límite de iteraciones'),
    node_limit: t('Límite de nodos'),
    feasible: t('Factible (sin probar que es óptima)'),
    time_limit: t('Se acabó el tiempo'),
    error: t('Error del motor')
  });
  const ok = $derived(r.status === 'optimal' || r.status === 'feasible');
  const conSolucion = $derived(r.variables.length > 0 && !['infeasible', 'unbounded', 'error', 'time_limit'].includes(r.status));

  // modelos grandes: no se dibujan decenas de miles de filas
  const TOPE = 1000;
  let soloNoCero = $state(true);
  const muchas = $derived(r.variables.length > 200);
  const vars = $derived((muchas && soloNoCero ? r.variables.filter((v) => Math.abs(v.value) > 1e-9) : r.variables).slice(0, TOPE));
  const varsOcultas = $derived((muchas && soloNoCero ? r.variables.filter((v) => Math.abs(v.value) > 1e-9).length : r.variables.length) - vars.length);
  const muchasR = $derived(r.constraints.length > 200);
  const cons = $derived((muchasR && soloNoCero ? r.constraints.filter((c) => Math.abs(c.slack_or_surplus) < 1e-9) : r.constraints).slice(0, TOPE));
  const consOcultas = $derived((muchasR && soloNoCero ? r.constraints.filter((c) => Math.abs(c.slack_or_surplus) < 1e-9).length : r.constraints.length) - cons.length);
</script>

<div class="sol">
  <header class="head card" class:fail={!ok}>
    <div>
      <span class="pill" class:good={ok} class:bad={!ok}><Icon name={ok ? 'check' : 'alerta'} size={14} stroke={2.2} />{statusText[r.status] ?? r.status}</span>
      {#if r.objective_value !== null}
        <div class="zline">
          <span class="zlab">{r.objective === 'max' ? t('Máximo') : t('Mínimo')}</span>
          <span class="zval">{num(r.objective_value)}</span>
        </div>
      {/if}
    </div>
    <div class="hder">
      {#if r.status !== 'optimal' || r.alternate_optima}<p class="msg">{r.message}</p>{/if}
      {#if r.engine}<EngineLine engine={r.engine} />{/if}
    </div>
    <div class="hia"><ExportMenu modulo="lp" /><ExplicarIA modulo="lp" /></div>
  </header>

  {#if r.diagnosis}
    <section class="diag">
      <h2>{t('Por qué no es factible')}</h2>
      <p>{r.diagnosis.summary}</p>
      {#if r.diagnosis.conflict.length}
        <h3>{t('Conflicto mínimo (basta relajar una para romperlo)')}</h3>
        <ul>
          {#each r.diagnosis.conflict as c, i (i)}<li><code>{c.text}</code></li>{/each}
        </ul>
      {/if}
      {#if r.diagnosis.relaxation.length}
        <h3>{t('Cambio mínimo que lo hace factible')}{#if r.diagnosis.relaxation_total != null}{' '}({t('total {0}', num(r.diagnosis.relaxation_total))}){/if}</h3>
        <ul>
          {#each r.diagnosis.relaxation as c, i (i)}<li>{c.text}</li>{/each}
        </ul>
      {/if}
    </section>
  {:else if r.status === 'infeasible'}
    <p class="hint">{t('Revisa las restricciones que se contradicen y las cotas de las variables. Una restricción ≥ con un lado derecho demasiado alto suele ser la causa.')}</p>
  {:else if r.status === 'unbounded'}
    <p class="hint">{t('Falta una restricción que limite {0}. Comprueba que no se ha olvidado un límite de capacidad o una cota superior.', r.unbounded_variable ?? t('alguna variable'))}</p>
  {/if}

  {#if r.objective_values?.length}
    <section>
      <h2>{t('Objetivos')}</h2>
      <ul class="objs">
        {#each r.objective_values as [n, v] (n)}<li><span>{n}</span><b>{num(v)}</b></li>{/each}
      </ul>
    </section>
  {/if}

  {#if alg}<AlgebraSolucion r={alg} />{/if}

  {#if conSolucion}
    {#if muchas || muchasR}
      <label class="filtro">
        <input type="checkbox" bind:checked={soloNoCero} />
        <span>{t('Mostrar solo las variables distintas de cero y las restricciones activas')}</span>
      </label>
    {/if}
    <section>
      <h2>{t('Variables de decisión')}</h2>
      <div class="tw" use:copiable>
        <table>
          <thead>
            <tr>
              <th class="l">{t('Variable')}</th>
              <th>{t('Valor')}</th>
              <th>c(j)</th>
              <th>{t('Contribución')}</th>
              <th>{t('Coste reducido')}</th>
              <th class="l">{t('Estado')}</th>
              {#if r.sensitivity}<th>{t('c(j) mínimo')}</th><th>{t('c(j) máximo')}</th>{/if}
            </tr>
          </thead>
          <tbody>
            {#each vars as v (v.name)}
              <tr class:basic={v.basis_status === 'basic'}>
                <td class="l name">{v.name}</td>
                <td class="val">{num(v.value)}</td>
                <td>{num(v.cost)}</td>
                <td>{num(v.contribution)}</td>
                <td>{num(v.reduced_cost)}</td>
                <td class="l">{v.basis_status === 'basic' ? t('Básica') : t('En su cota')}</td>
                {#if r.sensitivity}<td>{bound(v.cost_min)}</td><td>{bound(v.cost_max)}</td>{/if}
              </tr>
            {/each}
          </tbody>
          {#if varsOcultas > 0}<tbody><tr><td class="l muted" colspan={r.sensitivity ? 8 : 6}>{t('… y {0} variables más (guarda el modelo o usa la API para verlas todas)', varsOcultas.toLocaleString())}</td></tr></tbody>{/if}
          <tfoot>
            <tr>
              <td class="l" colspan="3">{t('Valor de la función objetivo')}</td>
              <td class="tot">{num(r.objective_value)}</td>
              <td colspan={r.sensitivity ? 4 : 2}></td>
            </tr>
          </tfoot>
        </table>
      </div>
    </section>

    <section>
      <h2>{t('Restricciones')}</h2>
      <div class="tw" use:copiable>
        <table>
          <thead>
            <tr>
              <th class="l">{t('Restricción')}</th>
              <th>{t('Lado izquierdo')}</th>
              <th class="c">{t('Sentido')}</th>
              <th>{t('Lado derecho')}</th>
              <th>{t('Holgura o exceso')}</th>
              {#if r.sensitivity}<th>{t('Precio sombra')}</th><th>{t('Lado der. mínimo')}</th><th>{t('Lado der. máximo')}</th>{/if}
            </tr>
          </thead>
          <tbody>
            {#each cons as c (c.name)}
              <tr class:binding={Math.abs(c.slack_or_surplus) < 1e-9}>
                <td class="l name">{c.name}</td>
                <td>{num(c.lhs)}</td>
                <td class="c">{dirLabel[c.direction]}</td>
                <td>{num(c.rhs)}</td>
                <td>{num(c.slack_or_surplus)}</td>
                {#if r.sensitivity}<td class="sp">{num(c.shadow_price)}</td><td>{bound(c.rhs_min)}</td><td>{bound(c.rhs_max)}</td>{/if}
              </tr>
            {/each}
            {#if consOcultas > 0}<tr><td class="l muted" colspan={r.sensitivity ? 8 : 5}>{t('… y {0} restricciones más', consOcultas.toLocaleString())}</td></tr>{/if}
          </tbody>
        </table>
      </div>
    </section>

    <div class="notes">
      {#if r.alternate_optima}<p>{t('Hay soluciones óptimas alternativas: otras combinaciones de variables alcanzan el mismo valor del objetivo.')}</p>{/if}
      {#if r.method === 'branch_and_bound'}
        <p>
          {t(
            'Resuelto con branch and bound en {0} nodos. El óptimo de la relajación lineal es {1}. En problemas enteros no se calculan precios sombra ni rangos.',
            r.branch_and_bound?.nodes_explored,
            num(r.branch_and_bound?.relaxed_objective)
          )}
        </p>
      {:else if r.method === 'branch_and_cut'}
        <p>{t('Resuelto con ramificación y corte (branch and cut) del motor. En problemas enteros no se calculan precios sombra ni rangos.')}</p>
      {:else if r.sensitivity}
        <p>
          {t(
            'El precio sombra es lo que cambia el objetivo por cada unidad más de lado derecho, válido dentro de su rango. Los rangos de c(j) indican cuánto puede cambiar cada coeficiente sin que cambie la solución.'
          )}
        </p>
      {/if}
      {#if r.method === 'simplex'}<p class="muted">{t('Método símplex, {0} iteraciones.', r.iterations)}</p>{/if}
    </div>
  {/if}
  {#if notas.length}
    <div class="notes">
      {#each notas as n, i (i)}<p class="muted">{n}</p>{/each}
    </div>
  {/if}
  {#if r.log}
    <details class="log">
      <summary>{t('Registro del motor')}</summary>
      <pre>{r.log}</pre>
    </details>
  {/if}
  {#if fuentes}
    <section>
      <h2>{t('Fuentes')}</h2>
      <SourcesView s={fuentes} />
    </section>
  {/if}
</div>

<style>
  .sol {
    display: flex;
    flex-direction: column;
    gap: 26px;
  }
  .head {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 28px;
    align-items: end;
    padding: 20px 22px;
    background:
      radial-gradient(120% 140% at 100% 0%, var(--good-tint), transparent 55%),
      var(--sheet);
  }
  .head.fail {
    background:
      radial-gradient(120% 140% at 100% 0%, var(--bad-tint), transparent 55%),
      var(--sheet);
  }
  .zline {
    display: flex;
    align-items: baseline;
    gap: 12px;
    margin-top: 10px;
  }
  .zlab {
    color: var(--ink-2);
  }
  .zval {
    font-family: var(--font-brand);
    font-size: 46px;
    font-weight: 700;
    letter-spacing: -0.02em;
    line-height: 1.05;
    color: var(--solver);
    font-variant-numeric: tabular-nums;
  }
  .hder {
    display: grid;
    gap: 4px;
  }
  .head {
    grid-template-columns: auto 1fr auto;
  }
  .hia {
    align-self: start;
    display: flex;
    gap: 8px;
  }
  .diag {
    padding: 16px 18px;
    border-left: 3px solid var(--bad);
    background: var(--bad-tint);
    border-radius: 0 var(--r-md) var(--r-md) 0;
  }
  .diag p {
    margin: 0 0 8px;
    max-width: 75ch;
  }
  .diag h3 {
    font-size: 13.5px;
    margin: 10px 0 4px;
  }
  .diag ul {
    margin: 0;
    padding-left: 20px;
  }
  .diag code {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 12.5px;
  }
  .objs {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 8px 22px;
  }
  .objs b {
    margin-left: 8px;
    font-variant-numeric: tabular-nums;
  }
  .filtro {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 13px;
    color: var(--ink-2);
  }
  .log pre {
    max-height: 340px;
    overflow: auto;
    font-size: 11.5px;
    padding: 10px;
    background: var(--sheet);
    border: 1px solid var(--rule);
    border-radius: var(--r-sm);
  }
  .msg {
    margin: 0;
    max-width: 60ch;
    color: var(--ink-2);
  }
  .hint {
    margin: 0;
    max-width: 70ch;
    padding: 12px 14px;
    border-left: 3px solid var(--bad);
    background: var(--bad-tint);
    border-radius: 0 var(--r-sm) var(--r-sm) 0;
  }
  h2 {
    font-size: 17px;
    margin-bottom: 10px;
  }
  .tw {
    width: max-content;
    max-width: 100%;
    overflow: auto;
    background: var(--sheet);
    border: 1px solid var(--rule);
    border-radius: var(--r-md);
  }
  th,
  td {
    text-align: right;
    padding: 8px 14px;
    border-bottom: 1px solid var(--rule-soft);
    white-space: nowrap;
  }
  th {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--ink-2);
    background: var(--sheet-2);
    border-bottom: 1px solid var(--rule);
  }
  .l {
    text-align: left;
  }
  .c {
    text-align: center;
  }
  .name {
    font-weight: 650;
  }
  .val {
    font-weight: 700;
  }
  tr.basic .val {
    color: var(--solver);
  }
  tr.binding td {
    background: var(--binding-tint);
  }
  tr.binding td:first-child {
    box-shadow: inset 3px 0 0 var(--binding);
  }
  .sp {
    font-weight: 650;
  }
  tfoot td {
    border-bottom: 0;
    font-weight: 600;
  }
  .tot {
    color: var(--solver);
    font-weight: 750;
  }
  .notes p {
    margin: 0 0 6px;
    max-width: 75ch;
  }
</style>
