<script lang="ts">
  import { ajustes } from '../lib/ajustes.svelte';
  // Banco de trabajo de cualquier módulo descrito por su especificación:
  // editor de datos generado a partir de las hojas del módulo y vistas del informe.
  import UndoRedo from './UndoRedo.svelte';
  import RecientesMenu from './RecientesMenu.svelte';
  import EngineLine from './EngineLine.svelte';
  import DataEditor from './generic/DataEditor.svelte';
  import NewProblemDialog from './generic/NewProblemDialog.svelte';
  import FieldInput from './generic/FieldInput.svelte';
  import BlockView from './blocks/BlockView.svelte';
  import Icon from './Icon.svelte';
  import OptsMenu from './ui/OptsMenu.svelte';
  import ExplicarIA from './ExplicarIA.svelte';
  import ExportMenu from './ExportMenu.svelte';
  import QuePasaBoton from './quepasa/QuePasaBoton.svelte';
  import QuePasaGen from './quepasa/QuePasaGen.svelte';
  import { shell } from '../lib/shell.svelte';
  import { app } from '../lib/state.svelte';
  import { ui } from '../lib/ui.svelte';
  import { t } from '../lib/i18n.svelte';
  import { shown, type Field } from '../lib/gtypes';

  let { id }: { id: string } = $props();
  const st = $derived(shell.get(id));
  $effect(() => {
    st.init();
  });

  const tabs = $derived([{ id: 'datos', title: t('Datos') }, ...(st.report?.sections.map((s) => ({ id: s.id, title: s.title })) ?? [])]);
  const section = $derived(st.report?.sections.find((s) => s.id === st.tab));
  const info = $derived(shell.modules.find((m) => m.id === id));
  const fileName = $derived(st.file ? st.file.split(/[\\/]/).pop() : t('Sin guardar'));

  function optVisible(f: Field) {
    return shown(f, (k) => (k === 'problem_type' ? st.model?.problem_type : st.options[k]));
  }

  // cada vista empieza arriba
  let contentEl: HTMLElement | undefined = $state();
  $effect(() => {
    st.tab;
    contentEl?.scrollTo({ top: 0 });
  });

  const quick = (f: Field) => f.kind.type === 'bool' || f.kind.type === 'choice';
  const barOpts = $derived(st.spec?.options.filter((f) => quick(f) && optVisible(f)) ?? []);
  const panelOpts = $derived(st.spec?.options.filter((f) => !quick(f) && optVisible(f)) ?? []);
  const canCompare = $derived(barOpts.some((f) => f.comparable));
  // el método (opción comparable) queda a la vista; las demás, en el panel de opciones
  const methodOpt = $derived(barOpts.find((f) => f.comparable && f.kind.type === 'choice'));
  const restOpts = $derived(barOpts.filter((f) => f !== methodOpt));
  function valueLabel(f: Field): string {
    const v = st.options[f.key];
    if (f.kind.type === 'choice') return f.kind.options.find((o) => o[0] === v)?.[1] ?? String(v ?? '');
    if (f.kind.type === 'bool') return v ? t('sí') : t('no');
    return String(v ?? '');
  }
  const optsResumen = $derived(
    restOpts
      .filter((f) => f.kind.type === 'bool' ? !!st.options[f.key] : true)
      .map((f) => (f.kind.type === 'bool' ? f.label : valueLabel(f)))
      .join(' · ')
  );

  function keydown(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey;
    // con los Ajustes abiertos o dentro de un panel, los atajos no tocan la hoja de atrás
    if (ajustes.abierta || (e.target instanceof Element && e.target.closest('.capa, .popover, [role="dialog"]'))) return;
    if (mod && e.key === 'Enter') {
      e.preventDefault();
      st.solve();
    } else if (mod && e.key.toLowerCase() === 's') {
      e.preventDefault();
      st.save();
    } else if (mod && e.key.toLowerCase() === 'o') {
      e.preventDefault();
      shell.open((m) => (st.error = m));
    } else if (mod && !e.altKey && (e.key.toLowerCase() === 'z' || e.key.toLowerCase() === 'y') && !(e.target instanceof HTMLTextAreaElement)) {
      // deshacer y rehacer (en el editor de texto, el propio del texto)
      e.preventDefault();
      if (e.key.toLowerCase() === 'y' || e.shiftKey) st.rehacer();
      else st.deshacer();
    }
  }

  // modelo de programación lineal equivalente (transbordo, ruta más corta, agente viajero)
  const lpModel = $derived((st.report?.data as { lp_model?: any } | undefined)?.lp_model ?? null);
  function openAsLp() {
    if (!lpModel) return;
    app.load(structuredClone($state.snapshot(lpModel)), null, 'matrix');
    shell.module = 'lp';
    ui.avisar(t('Modelo abierto en Programación lineal: puedes resolverlo, ver el símplex o guardarlo como .LPP para WinQSB'), 'info');
  }

  const statusText: Record<string, string> = {
    optimal: 'Óptimo',
    feasible: 'Solución factible',
    infeasible: 'No factible',
    unbounded: 'No acotado',
    solved: 'Resuelto'
  };
</script>

<svelte:window onkeydown={keydown} />

<main class="workbench">
  <header class="top">
    <div class="title">
      {#if st.model}
        <input
          class="pname"
          class:long={(st.model.name ?? '').length > 36}
          title={st.model.name}
          aria-label={t('Nombre del problema')}
          value={st.model.name}
          onchange={(e) => {
            st.model.name = (e.currentTarget as HTMLInputElement).value;
            st.changed();
          }}
        />
      {/if}
      <div class="sub">
        {#if st.ptype?.label}<span class="pill" title={info?.solves ?? ''}>{st.ptype.label}</span>{/if}
      </div>
    </div>
    <div class="actions">
      <button class="btn ghost icon" onclick={() => (st.newOpen = true)} title={t('Nuevo problema')} aria-label={t('Nuevo problema')}><Icon name="nuevo" size={16} /></button>
      <button class="btn ghost icon" onclick={() => shell.open((m) => (st.error = m))} title={t('Abrir (Ctrl+O)')} aria-label={t('Abrir')}><Icon name="abrir" size={16} /></button>
      <button class="btn ghost icon" onclick={() => st.save()} title={t('Guardar como .{0} (Ctrl+S)', st.spec?.file_ext ?? '')} aria-label={t('Guardar')}><Icon name="guardar" size={16} /></button>
      <RecientesMenu />
      <UndoRedo historial={st.historial} deshacer={() => st.deshacer()} rehacer={() => st.rehacer()} />
      <span class="sepv" aria-hidden="true"></span>
      {#if canCompare}
        <button class="btn" onclick={() => st.compare()} disabled={st.busy || !st.model} title={t('Resuelve con cada método y los compara')}>
          <Icon name="comparar" size={15} />{t('Comparar')}
        </button>
      {/if}
      <button class="btn primary solve" onclick={() => st.solve()} disabled={st.busy || !st.model} title={t('Ctrl+Intro')}>
        {#if st.busy}<span class="spinner" aria-hidden="true"></span>{t('Resolviendo…')}{:else}<Icon name="jugar" size={14} stroke={2} />{t('Resolver')}{/if}
      </button>
    </div>
  </header>

  <nav class="tabs" aria-label={t('Vistas')}>
    {#each tabs as tb (tb.id)}
      <button class:on={st.tab === tb.id} aria-current={st.tab === tb.id ? 'page' : undefined} onclick={() => (st.tab = tb.id)}>{tb.title}</button>
    {/each}
    <div class="opts">
      {#if methodOpt}
        <label class="opt metodo" title={methodOpt.help ?? methodOpt.label}>
          <span class="ol">{methodOpt.label}</span>
          <FieldInput field={methodOpt} value={st.options[methodOpt.key]} onchange={(v) => (st.options[methodOpt.key] = v)} />
        </label>
      {/if}
      {#if restOpts.length}
        <OptsMenu resumen={optsResumen}>
          {#each restOpts as f (f.key)}
            <label class="opt-p" class:check={f.kind.type === 'bool'}>
              {#if f.kind.type === 'bool'}
                <FieldInput field={f} value={st.options[f.key]} onchange={(v) => (st.options[f.key] = v)} />
                <span>{f.label}</span>
              {:else}
                <span class="opl">{f.label}</span>
                <FieldInput field={f} value={st.options[f.key]} onchange={(v) => (st.options[f.key] = v)} />
              {/if}
              {#if f.help}<span class="oph">{f.help}</span>{/if}
            </label>
          {/each}
        </OptsMenu>
      {/if}
    </div>
  </nav>

  <section class="content" bind:this={contentEl}>
    {#if st.error}
      <div class="alert" role="alert">
        <span class="atxt"><Icon name="alerta" size={18} />{st.error}</span>
        <button class="btn ghost" onclick={() => (st.error = null)}>{t('Cerrar')}</button>
      </div>
    {/if}
    {#if st.stale && st.tab !== 'datos'}
      <div class="stale"><Icon name="recargar" size={17} />{t('Los datos han cambiado desde la última resolución.')} <button class="link" onclick={() => st.solve()}>{t('Resolver de nuevo')}</button></div>
    {/if}

    {#if st.tab === 'datos'}
      {#if st.ptype && st.model}
        <DataEditor pt={st.ptype} model={st.model} onchange={() => st.changed()} modulo={id} report={st.report} stale={st.stale} />
        {#if panelOpts.length}
          <section class="analysis card">
            <h3><Icon name="ajustes" size={17} />{t('Parámetros del análisis')}</h3>
            <div class="agrid">
              {#each panelOpts as f (f.key)}
                <label class="af">
                  <span class="al">{f.label}</span>
                  <FieldInput field={f} value={st.options[f.key]} onchange={(v) => (st.options[f.key] = v)} />
                  {#if f.help}<span class="ah">{f.help}</span>{/if}
                </label>
              {/each}
            </div>
          </section>
        {/if}
      {:else if !st.error}
        <p class="muted">{t('Cargando…')}</p>
      {/if}
    {:else if section && st.report}
      {#if st.tab === st.report.sections[0]?.id}
        {@const fail = st.report.status === 'infeasible' || st.report.status === 'unbounded'}
        <div class="headline card" class:fail>
          <div class="hrow">
            <span class="pill" class:good={!fail} class:bad={fail}><Icon name={fail ? 'alerta' : 'check'} size={14} stroke={2.2} />{t(statusText[st.report.status] ?? st.report.status)}</span>
            <div class="hacc">
              {#if lpModel}
                <button class="btn" onclick={openAsLp} title={t('Abre el modelo matemático en el módulo de programación lineal (y desde ahí se puede guardar como .LPP para WinQSB)')}>
                  <Icon name="lp" size={16} />{t('Abrir como programa lineal')}
                </button>
              {/if}
              <QuePasaBoton modulo={id} />
              <ExportMenu modulo={id} />
              <ExplicarIA modulo={id} />
            </div>
          </div>
          <p>{st.report.headline}</p>
          {#if st.report.engine}<EngineLine engine={st.report.engine} />{/if}
        </div>
        <QuePasaGen {st} />
      {/if}
      <div class="blocks">
        {#each section.blocks as b, k (k)}
          <BlockView block={b} />
        {/each}
      </div>
    {/if}
  </section>

  <footer class="status">
    <span class="file" title={st.file ?? ''}>{#if st.dirty}<span class="dirty" title={t('Cambios sin guardar')}></span>{/if}{fileName}{st.dirty ? t(', con cambios sin guardar') : ''}</span>
    <span class="msg" aria-live="polite">{st.notice ?? (st.report && !st.stale ? st.report.headline : '')}</span>
    <span class="muted">{info?.winqsb ?? ''}</span>
  </footer>
</main>

<NewProblemDialog {st} />

<style>
  .opt {
    display: flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }
  .opt :global(.sel) {
    height: 26px;
    font-size: 12.5px;
    max-width: 260px;
  }
  .ol {
    color: var(--ink-3);
    font-size: 12px;
  }
  .opt-p {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12.5px;
    color: var(--ink-2);
  }
  .opt-p.check {
    flex-direction: row;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    color: var(--ink);
  }
  .opt-p.check .oph {
    flex-basis: 100%;
  }
  .opl {
    font-weight: 600;
  }
  .oph {
    font-size: 11.5px;
    color: var(--ink-3);
  }
  .sepv {
    width: 1px;
    height: 18px;
    background: var(--rule);
    margin: 0 4px;
  }
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
  .headline {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    margin-bottom: 18px;
    padding: 14px 18px;
    background:
      radial-gradient(120% 140% at 100% 0%, var(--good-tint), transparent 55%),
      var(--sheet);
  }
  .hrow {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    width: 100%;
    flex-wrap: wrap;
  }
  .hacc {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
  }
  .headline.fail {
    background:
      radial-gradient(120% 140% at 100% 0%, var(--bad-tint), transparent 55%),
      var(--sheet);
  }
  .headline p {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
    letter-spacing: -0.015em;
    line-height: 1.35;
    max-width: 75ch;
  }
  .analysis {
    margin-top: 22px;
    padding: 18px 20px 20px;
  }
  .analysis h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 15px;
    margin: 0 0 14px;
  }
  .analysis h3 :global(svg) {
    color: var(--solver);
  }
  .agrid {
    display: flex;
    flex-wrap: wrap;
    gap: 14px 22px;
    align-items: flex-start;
  }
  .af {
    display: flex;
    flex-direction: column;
    gap: 5px;
    width: 210px;
  }
  .al {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--ink-2);
  }
  .ah {
    font-size: 12px;
    color: var(--ink-3);
  }
  .blocks {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
</style>
