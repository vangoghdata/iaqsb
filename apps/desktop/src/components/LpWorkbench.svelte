<script lang="ts">
  import { ajustes } from '../lib/ajustes.svelte';
  // Módulo LP-ILP: editor y vistas propios (hoja del modelo, símplex, branch and bound, método gráfico).
  import ModelSheet from './ModelSheet.svelte';
  import SolutionView from './SolutionView.svelte';
  import StepsView from './StepsView.svelte';
  import BnbView from './BnbView.svelte';
  import GraphView from './GraphView.svelte';
  import PracticaView from './practica/PracticaView.svelte';
  import { practicable } from '../lib/practica';
  import QuePasaLp from './quepasa/QuePasaLp.svelte';
  import { app, type Tab } from '../lib/state.svelte';
  import { num } from '../lib/format';
  import Icon from './Icon.svelte';
  import OptsMenu from './ui/OptsMenu.svelte';
  import { t } from '../lib/i18n.svelte';
  import AlgebraView from './algebra/AlgebraView.svelte';
  import { algebra } from '../lib/algebra.svelte';
  import UndoRedo from './UndoRedo.svelte';
  import RecientesMenu from './RecientesMenu.svelte';

  // modo algebraico: Resolver, Abrir, Guardar y Nuevo actúan sobre el texto del modelo
  const resolver = () => (algebra.on ? algebra.solve() : app.solve());
  const guardar = () => (algebra.on ? algebra.guardar() : app.save());
  const abrir = () => (algebra.on ? algebra.abrir() : app.open());

  const tabs = $derived(
    [
      { id: 'modelo', label: t('Modelo'), show: true },
      { id: 'solucion', label: t('Solución'), show: !!app.report },
      { id: 'pasos', label: t('Tablas del símplex'), show: !!app.report && app.report.method === 'simplex' },
      { id: 'nodos', label: t('Branch and bound'), show: !!app.report?.branch_and_bound },
      { id: 'grafico', label: t('Método gráfico'), show: !!app.graph },
      { id: 'practica', label: t('Práctica'), show: !app.large && !algebra.on && practicable(app.model) },
      { id: 'quepasa', label: t('¿Qué pasa si…?'), show: !!app.report && !app.large && !algebra.on }
    ].filter((x) => x.show) as { id: Tab; label: string }[]
  );

  function keydown(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey;
    // con los Ajustes abiertos o dentro de un panel, los atajos no tocan la hoja de atrás
    if (ajustes.abierta || (e.target instanceof Element && e.target.closest('.capa, .popover, [role="dialog"]'))) return;
    if (mod && e.key === 'Enter') {
      e.preventDefault();
      resolver();
    } else if (mod && e.key.toLowerCase() === 's') {
      e.preventDefault();
      guardar();
    } else if (mod && e.key.toLowerCase() === 'o') {
      e.preventDefault();
      abrir();
    } else if (mod && !e.altKey && (e.key.toLowerCase() === 'z' || e.key.toLowerCase() === 'y') && !algebra.on && !(e.target instanceof HTMLTextAreaElement)) {
      // deshacer y rehacer (en el editor de texto, el propio del texto)
      e.preventDefault();
      if (e.key.toLowerCase() === 'y' || e.shiftKey) app.rehacer();
      else app.deshacer();
    }
  }

  const archivo = $derived(algebra.on ? algebra.file : app.file);
  const sucio = $derived(algebra.on ? algebra.dirty : app.dirty);
  const fileName = $derived(archivo ? archivo.split(/[\\/]/).pop() : t('Sin guardar'));
  const saveExt = $derived(app.layout === 'mps' ? '.MPS' : app.layout === 'lp' ? '.LP' : app.layout === 'xlsx' ? '.xlsx' : '.LPP');
  const big = $derived(app.large);
</script>

<svelte:window onkeydown={keydown} />

<main class="workbench">
    <header class="top">
      <div class="title">
        <input
          class="pname"
          aria-label={t('Nombre del problema')}
          value={algebra.on ? algebra.name : app.model.name}
          onchange={(e) => {
            const v = (e.currentTarget as HTMLInputElement).value;
            if (algebra.on) {
              algebra.name = v;
              algebra.dirty = true;
              return;
            }
            app.model.name = v;
            app.changed();
          }}
        />
        <div class="sub">
          <div class="seg modo" role="radiogroup" aria-label={t('Forma de escribir el modelo')}>
            <button role="radio" aria-checked={!algebra.on} class:on={!algebra.on} onclick={() => algebra.activar(false)} title={t('La hoja de WinQSB: una columna por variable')}>{t('Hoja')}</button>
            <button role="radio" aria-checked={algebra.on} class:on={algebra.on} onclick={() => algebra.activar(true)} title={t('Conjuntos, índices y restricciones generales, al estilo de AMPL y LINGO')}>{t('Modelo algebraico')}</button>
          </div>
{#if algebra.on}
          {@const st = algebra.stats}
          {#if st}<span class="muted">{t('{0} variables, {1} restricciones', st.variables.toLocaleString(), st.constraints.toLocaleString())}</span>{/if}
{:else}
{#if !big}
          <div class="seg" role="radiogroup" aria-label={t('Criterio del objetivo')}>
            {#each [['max', t('Maximizar')], ['min', t('Minimizar')]] as [v, l]}
              <button
                role="radio"
                aria-checked={app.model.objective === v}
                class:on={app.model.objective === v}
                onclick={() => {
                  app.model.objective = v as 'max' | 'min';
                  app.changed();
                }}>{l}</button
              >
            {/each}
          </div>
          {/if}
          {#if big}
            <span class="muted">{t('{0} variables, {1} restricciones', big.size.cols.toLocaleString(), big.size.rows.toLocaleString())}</span>
          {:else}
            <span class="muted">{t('{0} variables, {1} restricciones', app.model.variables.length, app.model.constraints.length)}</span>
          {/if}
{/if}
        </div>
      </div>

      <div class="actions">
        <button class="btn ghost icon" onclick={() => (algebra.on ? algebra.nuevo() : app.newProblem())} title={t('Nuevo problema')} aria-label={t('Nuevo problema')}><Icon name="nuevo" size={16} /></button>
        <button class="btn ghost icon" onclick={abrir} title={t('Abrir (Ctrl+O)')} aria-label={t('Abrir')}><Icon name="abrir" size={16} /></button>
        <RecientesMenu />
        {#if !algebra.on}<UndoRedo historial={app.historial} deshacer={() => app.deshacer()} rehacer={() => app.rehacer()} />{/if}
        {#if algebra.on}
          <div class="savegrp">
            <button class="btn" onclick={() => algebra.guardar()} title="Ctrl+S"><Icon name="guardar" size={15} />{t('Guardar {0}', algebra.extension)}</button>
            <select aria-label={t('Formato del archivo')} bind:value={algebra.formato} title={t('Formato del archivo')}>
              <option value="iaqsb">{t('Modelo algebraico (.iaqsb)')}</option>
              <option value="mps">{t('MPS (cualquier motor)')}</option>
              <option value="lp">{t('LP (formato de CPLEX)')}</option>
              <option value="lpp">{t('WinQSB: modelo normal')}</option>
            </select>
          </div>
        {:else}
        <div class="savegrp">
          <button class="btn" onclick={() => app.save()} title="Ctrl+S"><Icon name="guardar" size={15} />{t('Guardar {0}', saveExt)}</button>
          <select aria-label={t('Formato del archivo')} bind:value={app.layout} title={t('Formato del archivo')}>
            {#if !big}<option value="matrix">{t('WinQSB: hoja (matriz)')}</option>{/if}
            <option value="normal">{t('WinQSB: modelo normal')}</option>
            <option value="mps">{t('MPS (cualquier motor)')}</option>
            <option value="lp">{t('LP (formato de CPLEX)')}</option>
            {#if !big}<option value="xlsx">{t('Excel (.xlsx)')}</option>{/if}
          </select>
        </div>
        {/if}
        <button class="btn primary solve" onclick={resolver} disabled={app.busy} title={t('Ctrl+Intro')}>
          {#if app.busy}<span class="spinner" aria-hidden="true"></span>{t('Resolviendo…')}{:else}<Icon name="jugar" size={14} stroke={2} />{t('Resolver')}{/if}
        </button>
      </div>
    </header>

    <nav class="tabs" aria-label={t('Vistas')}>
      {#each tabs as tb (tb.id)}
        <button class:on={app.tab === tb.id} aria-current={app.tab === tb.id ? 'page' : undefined} onclick={() => (app.tab = tb.id)}>
          {tb.label}
        </button>
      {/each}
      <div class="opts">
        <OptsMenu resumen={app.showSteps ? t('Con tablas del símplex') : ''}>
          <label class="lpo"><input type="checkbox" bind:checked={app.showSteps} /> {t('Guardar las tablas del símplex')}</label>
          {#if app.isInteger}
            <label class="lpo col">
              <span>{t('Tolerancia entera')}</span>
              <input class="tol" type="number" step="0.001" min="0" bind:value={app.intTolerance} />
            </label>
          {/if}
        </OptsMenu>
      </div>
    </nav>

    <section class="content">
      {#if app.error}
        <div class="alert" role="alert">
          <span class="atxt"><Icon name="alerta" size={18} />{app.error}</span>
          <button class="btn ghost" onclick={() => (app.error = null)}>{t('Cerrar')}</button>
        </div>
      {/if}
      {#if app.stale && app.tab !== 'modelo'}
        <div class="stale"><Icon name="recargar" size={17} />{t('El modelo ha cambiado desde la última resolución.')} <button class="link" onclick={resolver}>{t('Resolver de nuevo')}</button></div>
      {/if}

      {#if app.tab === 'modelo' && algebra.on}
        <AlgebraView />
      {:else if app.tab === 'modelo' && big}
        <div class="grande card">
          <h2><Icon name="motor" size={18} />{t('Modelo grande')}</h2>
          <p>{t('Este modelo tiene {0} variables ({1} enteras), {2} restricciones y {3} coeficientes distintos de cero. Es demasiado grande para la hoja, así que se resuelve directamente con el motor elegido arriba.', big.size.cols.toLocaleString(), big.size.integer.toLocaleString(), big.size.rows.toLocaleString(), big.size.nonzeros.toLocaleString())}</p>
          <p class="muted">{t('Se puede guardar en MPS o LP (para cualquier motor) o en el formato algebraico de WinQSB (.LPP); que WinQSB pueda abrirlo depende de sus propios límites.')}</p>
          <details>
            <summary>{t('Ver el texto del modelo ({0})', big.format.toUpperCase())}</summary>
            <pre>{big.text.length > 200000 ? big.text.slice(0, 200000) + '\n…' : big.text}</pre>
          </details>
        </div>
      {:else if app.tab === 'modelo'}
        <ModelSheet />
      {:else if app.tab === 'solucion' && app.report}
        <SolutionView />
      {:else if app.tab === 'pasos'}
        <StepsView />
      {:else if app.tab === 'nodos'}
        <BnbView />
      {:else if app.tab === 'grafico'}
        <GraphView />
      {:else if app.tab === 'practica'}
        <PracticaView />
      {:else if app.tab === 'quepasa' && app.report && !big}
        <QuePasaLp />
      {/if}
    </section>

    <footer class="status">
      <span class="file" title={archivo ?? ''}>{#if sucio}<span class="dirty" title={t('Cambios sin guardar')}></span>{/if}{fileName}{sucio ? t(', con cambios sin guardar') : ''}</span>
      <span class="msg" aria-live="polite">
        {#if app.notice}
          {app.notice}
        {:else if app.report && !app.stale}
          {app.report.status === 'optimal' ? t('Óptimo: z = {0}', num(app.report.objective_value)) : app.report.status === 'feasible' ? t('Factible: z = {0}', num(app.report.objective_value)) : app.report.message}
        {/if}
      </span>
      <span class="muted">{t('Programación lineal y entera')}</span>
    </footer>
</main>

<style>
  .atxt {
    display: inline-flex;
    align-items: center;
    gap: 10px;
  }
  .atxt :global(svg) {
    color: var(--bad);
    flex: none;
  }
  .stale :global(svg) {
    color: var(--binding);
  }
  .lpo {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
  }
  .lpo.col {
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    color: var(--ink-2);
  }
  .grande {
    padding: 20px 22px;
    max-width: 80ch;
    display: grid;
    gap: 10px;
  }
  .grande h2 {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 17px;
  }
  .grande p {
    margin: 0;
  }
  .grande pre {
    max-height: 420px;
    overflow: auto;
    font-size: 11.5px;
    padding: 10px;
    border: 1px solid var(--rule);
    border-radius: var(--r-sm);
  }
</style>
