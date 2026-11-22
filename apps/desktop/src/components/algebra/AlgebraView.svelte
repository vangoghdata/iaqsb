<script lang="ts">
  // Vista «Modelo» del LP-ILP en modo algebraico: barra de herramientas,
  // editor con revisión en vivo y ayuda del lenguaje.
  import ModelEditor from './ModelEditor.svelte';
  import AlgebraStats from './AlgebraStats.svelte';
  import EjemplosMenu from './EjemplosMenu.svelte';
  import LanguageHelp from './LanguageHelp.svelte';
  import Icon from '../Icon.svelte';
  import { algebra } from '../../lib/algebra.svelte';
  import { app } from '../../lib/state.svelte';
  import { t } from '../../lib/i18n.svelte';

  const chk = $derived(algebra.check);
  const err = $derived(chk && !chk.ok ? (chk.errors[0] ?? null) : null);
  const vacia = $derived(app.model.variables.length === 0 && !app.large);
</script>

<div class="alg" class:con-ayuda={algebra.ayuda}>
  <div class="barra" role="toolbar" aria-label={t('Herramientas del modelo algebraico')}>
    <EjemplosMenu actual={algebra.example} onpick={(id) => algebra.cargarEjemplo(id)} />
    <span class="sep" aria-hidden="true"></span>
    <button class="btn" onclick={() => algebra.aHoja()} disabled={algebra.ocupado || !chk?.ok} title={t('Expande los índices en la hoja de LP-ILP: una columna por variable y una fila por restricción')}>
      <Icon name="tabla" size={15} />{t('Pasar a la hoja')}
    </button>
    <button class="btn" onclick={() => algebra.deHoja()} disabled={algebra.ocupado || vacia} title={t('Escribe la hoja actual en el lenguaje (sin índices)')}>
      <Icon name="copiar" size={15} />{t('Desde la hoja')}
    </button>
    <span class="esp"></span>
    <button class="btn ghost" class:on={algebra.ayuda} aria-pressed={algebra.ayuda} onclick={() => (algebra.ayuda = !algebra.ayuda)}>
      <Icon name="info" size={15} />{t('Ayuda del lenguaje')}
    </button>
  </div>

  {#if algebra.pendiente}
    {@const p = algebra.pendiente}
    <div class="confirma" role="alertdialog" aria-label={t('Confirmar')}>
      <Icon name="alerta" size={16} />
      <span>{p.mensaje}</span>
      <span class="esp"></span>
      <button class="btn" onclick={() => (algebra.pendiente = null)}>{t('Cancelar')}</button>
      <button
        class="btn primary"
        onclick={() => {
          // se toma la acción antes de cerrar el aviso (p deja de existir al cerrarlo)
          const accion = algebra.pendiente?.accion;
          algebra.pendiente = null;
          accion?.();
        }}>{p.boton}</button
      >
    </div>
  {/if}

  <div class="cuerpo">
    <div class="col">
      <div class="editor">
        <ModelEditor value={algebra.text} oninput={(v) => algebra.editar(v)} error={err} onsubmit={() => algebra.solve()} jump={algebra.salto} onjump={() => (algebra.salto = null)} memoria={algebra.memoria} />
      </div>

      <div class="estado" class:mal={!!err} aria-live="polite">
        {#if err}
          <button class="err" onclick={() => algebra.ir(err.line, err.col)} title={t('Ir al error')}>
            <Icon name="alerta" size={16} />
            {#if err.line > 0}<span class="donde">{t('Línea {0}, columna {1}', err.line, err.col)}</span>{/if}
            <span class="txt">{err.message}</span>
          </button>
        {:else if chk?.ok}
          <div class="ok">
            <span class="pill good"><Icon name="check" size={13} stroke={2.2} />{t('Modelo correcto')}</span>
            {#if algebra.checking}<span class="muted rev">{t('Revisando…')}</span>{/if}
            <span class="esp"></span>
            <span class="muted atajo"><span class="kbd">Ctrl</span> <span class="kbd">Intro</span> {t('resuelve')}</span>
          </div>
          <AlgebraStats s={chk.stats} />
        {:else}
          <span class="muted">{algebra.checking ? t('Revisando…') : t('Escribe el modelo: se revisa mientras escribes.')}</span>
        {/if}
      </div>
    </div>

    {#if algebra.ayuda}
      <LanguageHelp onclose={() => (algebra.ayuda = false)} />
    {/if}
  </div>
</div>

<style>
  .alg {
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: 100%;
    min-height: 460px;
  }
  .barra {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .sep {
    width: 1px;
    height: 20px;
    background: var(--rule);
    margin: 0 2px;
  }
  .esp {
    flex: 1;
  }
  .btn.ghost.on {
    background: var(--solver-tint);
    color: var(--solver);
  }
  .confirma {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    padding: 8px 10px 8px 14px;
    border-radius: var(--r-md);
    background: var(--binding-tint);
    border: 1px solid color-mix(in oklab, var(--binding) 35%, transparent);
  }
  .confirma > :global(svg) {
    color: var(--binding);
    flex: none;
  }
  .cuerpo {
    flex: 1;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 14px;
    min-height: 0;
  }
  .con-ayuda .cuerpo {
    grid-template-columns: minmax(0, 1fr) minmax(300px, 400px);
  }
  @media (max-width: 1100px) {
    /* en pantallas angostas la ayuda va debajo y la vista se desplaza */
    .con-ayuda .cuerpo {
      flex: none;
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: minmax(440px, 62vh) 440px;
    }
  }
  .col {
    display: grid;
    grid-template-rows: minmax(260px, 1fr) auto;
    gap: 10px;
    min-height: 0;
    min-width: 0;
  }
  .editor {
    min-height: 0;
  }
  .estado {
    display: grid;
    gap: 10px;
    padding: 12px 14px;
    border-radius: var(--r-lg);
    background: var(--sheet);
    border: 0.5px solid var(--rule);
    max-height: 40vh;
    overflow: auto;
  }
  .estado.mal {
    background: var(--bad-tint);
    border-color: color-mix(in oklab, var(--bad) 40%, transparent);
    padding: 0;
  }
  .err {
    display: flex;
    align-items: baseline;
    gap: 10px;
    width: 100%;
    padding: 11px 14px;
    border: 0;
    background: none;
    text-align: left;
    color: var(--ink);
    font-size: 13px;
    border-radius: var(--r-lg);
  }
  .err :global(svg) {
    color: var(--bad);
    flex: none;
    align-self: center;
  }
  .err:hover .donde {
    text-decoration: underline;
  }
  .donde {
    flex: none;
    font-weight: 700;
    color: var(--bad);
  }
  .ok {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .rev,
  .atajo {
    font-size: 12px;
  }
</style>
