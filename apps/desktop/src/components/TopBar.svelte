<script lang="ts">
  import Nativo from './chat/Nativo.svelte';
  import { guia } from '../lib/guia.svelte';
  // Barra de arriba: módulo activo, ejemplos y tema.
  import Icon from './Icon.svelte';
  import SelectorTemaCompacto from './tema/SelectorTemaCompacto.svelte';
  import MotorMenu from './MotorMenu.svelte';
  import { app } from '../lib/state.svelte';
  import { shell } from '../lib/shell.svelte';
  import { ui } from '../lib/ui.svelte';
  import { chat } from '../lib/chat.svelte';
  import { creador } from '../lib/creador.svelte';
  import { t } from '../lib/i18n.svelte';
  import { ajustes } from '../lib/ajustes.svelte';

  const info = $derived(shell.modules.find((m) => m.id === shell.module));
  type Ex = { id: string; label: string; origin?: string; from?: string; ok?: boolean };
  const lpExamples: Ex[] = $derived([
    { id: 'wyndor', label: 'Wyndor Glass' },
    { id: 'dieta', label: t('Dieta mínima') },
    { id: 'produccion_entera', label: t('Producción con lotes enteros') },
    { id: 'mochila', label: t('Mochila binaria') }
  ]);
  const gen = $derived(shell.module === 'lp' ? null : shell.get(shell.module));
  const examples = $derived(
    shell.module === 'lp'
      ? lpExamples
      : (gen?.examples ?? []).map(
          (e): Ex => ({
            id: e.name,
            label: e.model.name || e.name,
            origin: e.source?.origin_label,
            from: e.source?.summary,
            ok: e.source?.verified
          })
        )
  );

  let abierto = $state(false);
  let caja: HTMLElement | undefined = $state();

  function cargar(id: string, label: string) {
    abierto = false;
    if (shell.module === 'lp') app.loadExample(id);
    else {
      gen?.loadExample(id);
    }
    ui.avisar(t('Ejemplo cargado: {0}', label), 'info');
  }

  function fuera(e: MouseEvent) {
    if (abierto && caja && !caja.contains(e.target as Node)) abierto = false;
  }
</script>

<svelte:window onclick={fuera} onkeydown={(e) => e.key === 'Escape' && (abierto = false)} />

<header class="barra">
  {#if !ui.fijado}
    <button
      class="btn ghost icon"
      aria-label={t('Mostrar la barra lateral')}
      title={t('Módulos (lleva el mouse al borde izquierdo o pulsa Ctrl+K)')}
      onclick={() => (ui.lateral ? ui.ocultarLateral(0) : ui.mostrarLateral())}
      onmouseenter={() => ui.mostrarLateral()}
    >
      <Icon name="lateral" size={17} />
    </button>
  {/if}

  <div class="modulo">
    <span class="mico"><Icon name={info?.base ?? shell.module} size={15} /></span>
    <span class="mnom">{info?.name ?? t('Programación lineal y entera')}</span>
    {#if info}
      {#if info.curso}
        <span class="chip" title={t('Tema de un curso instalado')}>{info.curso.titulo}</span>
      {:else}
        <span class="chip" title={t('Módulo de WinQSB y extensión de sus archivos')}>{info.winqsb} · .{info.file_ext}</span>
      {/if}
    {/if}
  </div>

  <div class="der">
    {#if creador.actual}
      <button class="btn volver" title={t('Vuelve al editor del curso que estás armando')} onclick={() => creador.mostrar()}>
        <Icon name="izquierda" size={15} />
        <span class="txt">{t('Volver al curso')}</span>
      </button>
    {/if}
    <button class="btn ghost guiame" class:nueva={!guia.vistas.includes(shell.module)} title={t('Guíame: qué es este módulo, para qué sirve en la vida real, su historia, cómo usarlo y qué se usa hoy en la industria')} onclick={() => guia.abrir(shell.module)}>
      <Icon name="info" size={17} />
      <span class="txt">{t('Guíame')}</span>
    </button>
    <div class="ejemplos" bind:this={caja}>
      <button class="btn ghost" aria-haspopup="menu" aria-expanded={abierto} onclick={() => (abierto = !abierto)} disabled={!examples.length}>
        <Icon name="libro" size={17} />
        <span class="txt">{t('Ejemplos')}</span>
        <Icon name="abajo" size={15} />
      </button>
      {#if abierto}
        <div class="popover lista" role="menu">
          <p class="ptit">{t('Ejemplos de {0}', info?.winqsb ?? 'LP-ILP')}</p>
          {#each examples as ex (ex.id)}
            <button role="menuitem" class="opc" onclick={() => cargar(ex.id, ex.label)} title={ex.from ? `${ex.origin}: ${ex.from}` : ''}>
              <span class="el">{ex.label}</span>
              {#if ex.origin}
                <span class="ef"><span class="eo">{ex.origin}</span>{#if ex.from && ex.from !== ex.origin}{' · '}{ex.from}{/if}{#if ex.ok === false}{' · '}<span class="pv">{t('por verificar')}</span>{/if}</span>
              {/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <MotorMenu />

    <button class="btn ghost asistente" class:on={chat.abierto} aria-pressed={chat.abierto} title={t('Asistente IA: conversa con el modelo que conectes sobre tu problema')} onclick={() => chat.alternar()}>
      <Nativo toma="saludo" tam={22} circulo fijo />
      <span class="txt">{t('Asistente')}</span>
    </button>

    {#if shell.cambiando}
      <span class="traduciendo" role="status" aria-live="polite"><span class="spinner" aria-hidden="true"></span>{t('Cambiando el idioma…')}</span>
    {/if}

    <SelectorTemaCompacto />

    <button class="btn ghost icon" title={t('Ajustes (Ctrl+,)')} aria-label={t('Ajustes')} onclick={() => ajustes.abrir()}>
      <Icon name="engranaje" size={18} />
    </button>
  </div>
</header>

<style>
  .guiame {
    position: relative;
  }
  .guiame.nueva::after {
    content: '';
    position: absolute;
    top: 5px;
    right: 5px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--solver);
    animation: latido 1.8s ease-in-out infinite;
  }
  @keyframes latido {
    0%, 100% { transform: scale(1); opacity: 1; }
    50% { transform: scale(1.5); opacity: 0.5; }
  }
  @media (prefers-reduced-motion: reduce) {
    .guiame.nueva::after { animation: none; }
  }
  .barra {
    position: relative;
    z-index: 20;
    display: flex;
    align-items: center;
    gap: 14px;
    height: var(--bar-h);
    padding: 0 12px 0 12px;
    background: var(--barra-bg);
    -webkit-backdrop-filter: var(--vidrio);
    backdrop-filter: var(--vidrio);
    color: var(--barra-fg);
    border-bottom: 0.5px solid var(--rule);
  }
  .barra :global(.btn.ghost) {
    color: var(--barra-fg-2);
  }
  .barra :global(.btn.ghost:hover) {
    color: var(--barra-fg);
    background: color-mix(in oklab, var(--barra-fg) 8%, transparent);
  }
  .modulo {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .mico {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 6px;
    background: var(--solver-tint);
    color: var(--solver);
    flex: none;
  }
  :global(html[data-personal]) .mico {
    background: color-mix(in oklab, var(--barra-fg) 10%, transparent);
    color: var(--barra-fg);
  }
  .mnom {
    font-weight: 600;
    font-size: 13.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .chip {
    flex: none;
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 999px;
    color: var(--barra-fg-2);
    border: 1px solid color-mix(in oklab, var(--barra-fg) 14%, transparent);
  }
  .der {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .ejemplos {
    position: relative;
  }
  .barra .asistente.on {
    background: var(--solver-tint);
    color: var(--solver);
  }
  .traduciendo {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    color: var(--barra-fg-2);
    white-space: nowrap;
  }
  .lista {
    right: 0;
    top: calc(100% + 6px);
    width: 360px;
    max-height: 70vh;
    overflow: auto;
    padding: 6px;
  }
  .ptit {
    margin: 4px 8px 6px;
    font-size: 10.5px;
    font-weight: 650;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .opc {
    display: block;
    width: 100%;
    text-align: left;
    border: 0;
    background: none;
    padding: 8px 10px;
    border-radius: 9px;
    font-size: 13px;
  }
  .el {
    display: block;
  }
  .ef {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    margin-top: 2px;
    font-size: 11.5px;
    color: var(--ink-3);
    line-height: 1.35;
  }
  .eo {
    font-weight: 600;
    color: var(--ink-2);
  }
  .pv {
    color: color-mix(in srgb, var(--binding) 85%, var(--ink));
  }
  .opc:hover .ef,
  .opc:focus-visible .ef {
    color: inherit;
  }
  .opc:hover,
  .opc:focus-visible {
    background: var(--solver-tint);
    color: var(--solver);
    outline: none;
  }
  @media (max-width: 900px) {
    .barra {
      padding-left: 12px;
    }
  }
  @media (max-width: 640px) {
    .chip,
    .txt {
      display: none;
    }
  }
</style>
