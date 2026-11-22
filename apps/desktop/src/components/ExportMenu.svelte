<script lang="ts">
  // Botón «Exportar»: el informe que se ve, para entregar (Word, PDF, HTML, Excel
  // o Markdown), con portada (autor, curso, institución) y, si se quiere, el paso
  // a paso. También guarda el modelo como libro de Excel.
  import Icon from './Icon.svelte';
  import { app } from '../lib/state.svelte';
  import { shell } from '../lib/shell.svelte';
  import { ajustes } from '../lib/ajustes.svelte';
  import { motor } from '../lib/motor.svelte';
  import { ui } from '../lib/ui.svelte';
  import { t } from '../lib/i18n.svelte';
  import { saveXlsx } from '../lib/backend';
  import { comaInformes, exportarInforme, FORMATOS, type ArgsInforme, type Formato } from '../lib/excel';

  let { modulo }: { modulo: string } = $props();

  const CLAVE = 'iaqsb.exportar';
  function leerPref(): { formato: Formato; pasos: boolean | null } {
    try {
      const v = JSON.parse(localStorage.getItem(CLAVE) ?? '{}');
      return { formato: FORMATOS.some((f) => f.id === v.formato) ? v.formato : 'docx', pasos: typeof v.pasos === 'boolean' ? v.pasos : null };
    } catch {
      return { formato: 'docx', pasos: null };
    }
  }
  function guardarPref() {
    try {
      localStorage.setItem(CLAVE, JSON.stringify({ formato, pasos }));
    } catch {
      /* solo esta sesión */
    }
  }

  const esLp = $derived(modulo === 'lp');
  const st = $derived(esLp ? null : shell.get(modulo));
  const informe = $derived(esLp ? app.report : st?.report);
  const viejo = $derived(esLp ? app.stale : !!st?.stale);
  const ocupadoResolver = $derived(esLp ? app.busy : !!st?.busy);

  const pref = leerPref();
  let formato = $state<Formato>(pref.formato);
  let pasos = $state(pref.pasos ?? ajustes.pasos);
  let coma = $state(comaInformes());
  let autor = $state(ajustes.autor);
  let curso = $state(ajustes.curso);
  let institucion = $state(ajustes.institucion);
  let abierto = $state(false);
  let ocupado = $state(false);
  let guardandoModelo = $state(false);
  let caja: HTMLElement | undefined = $state();
  let pos = $state({ top: 0, right: 0, max: 600 });

  const actual = $derived(FORMATOS.find((f) => f.id === formato)!);
  const conPortada = $derived(formato === 'docx' || formato === 'pdf' || formato === 'html');

  function alternar(e: MouseEvent) {
    if (abierto) {
      abierto = false;
      return;
    }
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    pos = { top: r.bottom + 6, right: Math.max(8, window.innerWidth - r.right), max: Math.max(260, window.innerHeight - r.bottom - 16) };
    // al abrir, la portada y el separador vienen de los Ajustes
    autor = ajustes.autor;
    curso = ajustes.curso;
    institucion = ajustes.institucion;
    coma = comaInformes();
    abierto = true;
  }

  function fuera(e: MouseEvent) {
    if (abierto && caja && !caja.contains(e.target as Node)) abierto = false;
  }

  function guardarPortada() {
    if (ajustes.autor === autor.trim() && ajustes.curso === curso.trim() && ajustes.institucion === institucion.trim()) return;
    ajustes.autor = autor.trim();
    ajustes.curso = curso.trim();
    ajustes.institucion = institucion.trim();
    ajustes.guardar();
  }

  function plano<T>(x: T): T {
    return JSON.parse(JSON.stringify(x));
  }

  /** El modelo tal como lo recibe el núcleo. */
  function modelo(): unknown {
    if (esLp) return app.large ? { name: app.model.name, format: app.large.format, text: app.large.text } : plano(app.model);
    return plano(st?.model);
  }

  /** Los argumentos de export_report: el informe que se ve o, si le faltan los pasos, el modelo para resolverlo con ellos. */
  function argumentos(): ArgsInforme {
    const base = { module: modulo, format: formato, steps: pasos, author: autor.trim(), course: curso.trim(), institution: institucion.trim(), decimal_comma: coma };
    const rep = plano(informe) as any;
    const m = modelo();
    if (pasos && !viejo) {
      if (esLp && rep?.method === 'simplex' && !rep?.simplex_trace) {
        return { ...base, model: m, options: { trace: true, bnb_trace: true, integer_tolerance: app.intTolerance, ...motor.params() } };
      }
      if (!esLp && st?.options && st.options.steps === false) {
        const p = motor.params();
        return { ...base, model: m, options: { ...plano(st.options), steps: true, ...(Object.keys(p).length ? { engine: p } : {}) } };
      }
    }
    return { ...base, report: rep, ...(esLp ? { model: m } : {}) };
  }

  async function exportar() {
    if (ocupado || !informe) return;
    guardarPortada();
    guardarPref();
    const nombre = String((informe as any).problem || (esLp ? app.model.name : st?.model?.name) || t('informe'));
    ocupado = true;
    try {
      const r = await exportarInforme(argumentos(), nombre);
      if (r) {
        abierto = false;
        ui.avisar(t('Informe exportado: {0}', r.split(/[\\/]/).pop()));
      }
    } catch (e) {
      ui.avisar(t('No se pudo exportar el informe: {0}', e instanceof Error ? e.message : String(e)), 'error');
    } finally {
      ocupado = false;
    }
  }

  async function modeloExcel() {
    if (guardandoModelo) return;
    guardandoModelo = true;
    try {
      const file = esLp ? app.file : st?.file;
      const r = await saveXlsx(modulo, modelo(), file ?? undefined);
      if (r) {
        abierto = false;
        ui.avisar(t('Modelo guardado en Excel: {0}', r.split(/[\\/]/).pop()));
      }
    } catch (e) {
      ui.avisar(t('No se pudo guardar el modelo en Excel: {0}', e instanceof Error ? e.message : String(e)), 'error');
    } finally {
      guardandoModelo = false;
    }
  }

  function resolver() {
    if (esLp) app.solve();
    else st?.solve();
  }
</script>

<svelte:window onclick={fuera} onkeydown={(e) => e.key === 'Escape' && abierto && (abierto = false)} />

<div class="exp" bind:this={caja}>
  <button class="btn" class:on={abierto} aria-haspopup="dialog" aria-expanded={abierto} onclick={alternar} title={t('Exportar el informe a Word, PDF, HTML, Excel o Markdown')}>
    <Icon name="exportar" size={15} />{t('Exportar')}<Icon name="abajo" size={12} />
  </button>

  {#if abierto}
    <div class="popover panel" role="dialog" aria-label={t('Exportar el informe')} style:top={`${pos.top}px`} style:right={`${pos.right}px`} style:max-height={`${pos.max}px`}>
      <p class="ptit">{t('Exportar el informe')}</p>

      <div class="fmts" role="radiogroup" aria-label={t('Formato')}>
        {#each FORMATOS as f (f.id)}
          <button role="radio" aria-checked={formato === f.id} class="fmt" class:on={formato === f.id} onclick={() => (formato = f.id)}>
            <span class="badge b-{f.id}" aria-hidden="true">{f.ext.toUpperCase()}</span>
            <span class="ft">
              <span class="fn">{f.nombre} <span class="fx">.{f.ext}</span></span>
              <span class="fd">{t(f.desc)}</span>
            </span>
            {#if formato === f.id}<span class="ok"><Icon name="check" size={15} stroke={2.2} /></span>{/if}
          </button>
        {/each}
      </div>

      <label class="check">
        <input type="checkbox" bind:checked={pasos} />
        <span>{t('Incluir el paso a paso')}<span class="sub">{t('Iteraciones, tablas y nodos del método')}</span></span>
      </label>
      {#if conPortada}
        <label class="check">
          <input type="checkbox" bind:checked={coma} />
          <span>{t('Coma decimal')} <span class="ej">({coma ? '3,5' : '3.5'})</span></span>
        </label>

        <fieldset class="portada">
          <legend>{t('Portada')}</legend>
          <label class="fila"><span>{t('Autor')}</span><input bind:value={autor} onchange={guardarPortada} placeholder={t('Nombre y apellido')} autocomplete="name" /></label>
          <label class="fila"><span>{t('Curso')}</span><input bind:value={curso} onchange={guardarPortada} placeholder={t('Investigación de operaciones I')} /></label>
          <label class="fila"><span>{t('Institución')}</span><input bind:value={institucion} onchange={guardarPortada} placeholder={t('Universidad')} /></label>
        </fieldset>
      {:else}
        <p class="nota">{formato === 'xlsx' ? t('El libro trae una hoja de resumen y una por cada tabla, con los números como números.') : t('El Markdown es texto plano: sin portada ni figuras.')}</p>
      {/if}

      {#if viejo}
        <div class="aviso">
          <Icon name="alerta" size={15} />
          <span>{t('El modelo cambió desde la última resolución: se exporta el informe anterior.')} <button class="link" onclick={resolver} disabled={ocupadoResolver}>{t('Resolver de nuevo')}</button></span>
        </div>
      {/if}

      <button class="btn primary ir" onclick={exportar} disabled={ocupado || !informe}>
        {#if ocupado}<span class="spinner" aria-hidden="true"></span>{t('Generando…')}{:else}<Icon name="exportar" size={15} />{t('Exportar a {0}', actual.nombre)}{/if}
      </button>

      <div class="modelo">
        <span class="ml">{t('Modelo')}</span>
        <button class="btn ghost" onclick={modeloExcel} disabled={guardandoModelo} title={t('Una hoja por tabla del editor; se vuelve a abrir en IAQSB sin perder nada')}>
          <Icon name="tabla" size={15} />{t('Guardar el modelo en Excel (.xlsx)')}
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .exp {
    position: relative;
  }
  .btn.on {
    background: var(--rule-soft);
  }
  .panel {
    position: fixed;
    background: var(--sheet);
    width: 372px;
    max-width: calc(100vw - 16px);
    overflow: auto;
    padding: 12px 14px 12px;
    display: grid;
    gap: 10px;
  }
  .ptit {
    margin: 0;
    font-size: 10.5px;
    font-weight: 650;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .fmts {
    display: grid;
    gap: 2px;
  }
  .fmt {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 10px;
    padding: 5px 8px;
    border: 1px solid transparent;
    border-radius: var(--r-sm);
    background: none;
    text-align: left;
    color: var(--ink);
  }
  .fmt:hover {
    background: var(--rule-soft);
  }
  .fmt.on {
    background: var(--solver-tint);
    border-color: color-mix(in oklab, var(--solver) 35%, transparent);
  }
  .fmt:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -1px;
  }
  .badge {
    --c: #8e8e93;
    display: grid;
    place-items: center;
    width: 38px;
    height: 26px;
    border-radius: 6px;
    font-size: 9.5px;
    font-weight: 750;
    letter-spacing: 0.02em;
    color: var(--c);
    background: color-mix(in oklab, var(--c) 16%, transparent);
    border: 0.5px solid color-mix(in oklab, var(--c) 35%, transparent);
  }
  .b-docx {
    --c: #6ea8ff;
  }
  .b-pdf {
    --c: #ff6b61;
  }
  .b-html {
    --c: #ffa24c;
  }
  .b-xlsx {
    --c: #4cc77f;
  }
  .b-md {
    --c: #a1a1a6;
  }
  :global(html[data-tema='claro']) .b-docx {
    --c: #2b579a;
  }
  :global(html[data-tema='claro']) .b-pdf {
    --c: #c5221f;
  }
  :global(html[data-tema='claro']) .b-html {
    --c: #c25400;
  }
  :global(html[data-tema='claro']) .b-xlsx {
    --c: #1d6f42;
  }
  :global(html[data-tema='claro']) .b-md {
    --c: #6e6e73;
  }
  .ft {
    display: grid;
    min-width: 0;
  }
  .fn {
    font-weight: 600;
    font-size: 13px;
  }
  .fx {
    font-weight: 500;
    color: var(--ink-3);
    font-size: 12px;
  }
  .fd {
    font-size: 11.5px;
    color: var(--ink-2);
    line-height: 1.3;
  }
  .ok {
    color: var(--solver);
    display: grid;
  }
  .check {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    font-size: 13px;
  }
  .check input {
    margin-top: 2px;
  }
  .sub {
    display: block;
    font-size: 11.5px;
    color: var(--ink-3);
  }
  .ej {
    color: var(--ink-3);
    font-variant-numeric: tabular-nums;
  }
  .portada {
    border: 0.5px solid var(--rule);
    border-radius: var(--r-md);
    margin: 0;
    padding: 6px 10px 10px;
    display: grid;
    gap: 6px;
  }
  legend {
    padding: 0 4px;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--ink-2);
  }
  .fila {
    display: grid;
    grid-template-columns: 82px 1fr;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    color: var(--ink-2);
  }
  .fila input {
    width: 100%;
    height: 28px;
    padding: 0 8px;
    border: 1px solid var(--rule);
    border-radius: var(--r-sm);
    background: var(--field);
    color: var(--ink);
    font-size: 13px;
  }
  .fila input:focus {
    outline: none;
    border-color: var(--focus);
    box-shadow: 0 0 0 3px var(--solver-tint);
  }
  .nota {
    margin: 0;
    font-size: 12px;
    color: var(--ink-3);
    line-height: 1.4;
  }
  .aviso {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    padding: 8px 10px;
    border-radius: var(--r-sm);
    background: var(--binding-tint);
    font-size: 12px;
    line-height: 1.4;
  }
  .aviso :global(svg) {
    color: var(--binding);
    flex: none;
    margin-top: 1px;
  }
  .link {
    border: 0;
    background: none;
    padding: 0;
    color: var(--solver);
    font-weight: 600;
    font-size: inherit;
  }
  .ir {
    height: 34px;
    width: 100%;
    font-size: 13.5px;
    font-weight: 600;
  }
  .modelo {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding-top: 8px;
    border-top: 0.5px solid var(--rule);
  }
  .ml {
    font-size: 10.5px;
    font-weight: 650;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .modelo .btn {
    font-size: 12.5px;
  }
</style>
