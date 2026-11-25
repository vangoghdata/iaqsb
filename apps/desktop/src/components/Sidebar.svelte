<script lang="ts">
  import { cursos } from '../lib/cursos.svelte';
  import { creador } from '../lib/creador.svelte';
  import { MARCA } from '../lib/marca';
  // Menú lateral al estilo de IA Nativa: marca, buscador y módulos agrupados con ícono.
  import Logo from './Logo.svelte';
  import Icon from './Icon.svelte';
  import { app } from '../lib/state.svelte';
  import { shell } from '../lib/shell.svelte';
  import { ui } from '../lib/ui.svelte';
  import { t } from '../lib/i18n.svelte';

  const GRUPOS: { titulo: string; ids: string[] }[] = [
    { titulo: 'Optimización', ids: ['lp', 'gp', 'qp', 'nlp', 'dp'] },
    { titulo: 'Transporte y distribución', ids: ['net', 'vrp', 'modo'] },
    { titulo: 'Inventarios y almacenes', ids: ['its', 'inv', 'alm', 'fin', 'kpi'] },
    { titulo: 'Proyectos y producción', ids: ['pert', 'ap', 'pmp', 'mrp', 'mrp2', 'job', 'fll'] },
    { titulo: 'Decisión y pronóstico', ids: ['da', 'mkp', 'fc', 'st'] },
    { titulo: 'Colas', ids: ['qa', 'qss'] },
    { titulo: 'Calidad', ids: ['qcc', 'asa'] }
  ];

  let q = $state('');
  let buscador: HTMLInputElement | undefined = $state();

  const sinTildes = (s: string) => s.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase();
  const disponibles = $derived(shell.modules.filter((m) => m.available));
  const grupos = $derived.by(() => {
    const qn = sinTildes(q.trim());
    const pasa = (m: (typeof disponibles)[number]) =>
      !qn || sinTildes(`${m.name} ${m.winqsb} ${m.solves} ${m.file_ext}`).includes(qn);
    const usados = new Set(GRUPOS.flatMap((g) => g.ids));
    const lista: { titulo: string; mods: typeof disponibles; curso?: boolean }[] = GRUPOS.map((g) => ({
      titulo: g.titulo,
      mods: g.ids.map((id) => disponibles.find((m) => m.id === id)).filter((m) => m && pasa(m)) as typeof disponibles
    }));
    const otros = disponibles.filter((m) => !usados.has(m.id) && !m.curso && pasa(m));
    if (otros.length) lista.push({ titulo: 'Otros', mods: otros });
    // un grupo por curso instalado (.ncourse)
    const porCurso = new Map<string, typeof disponibles>();
    for (const m of disponibles) if (m.curso && pasa(m)) porCurso.set(m.curso.titulo, [...(porCurso.get(m.curso.titulo) ?? []), m]);
    for (const [titulo, mods] of porCurso) lista.push({ titulo, mods, curso: true });
    return lista.filter((g) => g.mods.length);
  });
  const plano = $derived(grupos.flatMap((g) => g.mods));

  function elegir(id: string) {
    shell.module = id;
    creador.ocultar();
    q = '';
    if (!ui.fijado) {
      buscador?.blur();
      ui.ocultarLateral(0);
    }
  }

  function teclas(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      ui.mostrarLateral();
      requestAnimationFrame(() => buscador?.focus());
    } else if (e.key === 'Escape' && ui.lateral && !ui.fijado) {
      ui.ocultarLateral(0);
    }
  }

  function enBuscador(e: KeyboardEvent) {
    if (e.key === 'Enter' && plano.length) elegir(plano[0].id);
    else if (e.key === 'Escape') {
      q = '';
      buscador?.blur();
      if (!ui.fijado) ui.ocultarLateral(0);
    }
  }

  let aside: HTMLElement | undefined = $state();
  function salir() {
    if (ui.fijado) return;
    // no se esconde mientras se escribe en el buscador
    if (aside && aside.contains(document.activeElement) && document.activeElement === buscador) return;
    ui.ocultarLateral();
  }
</script>

<svelte:window onkeydown={teclas} />

<aside
  class="side"
  class:flota={!ui.fijado}
  class:visible={ui.fijado || ui.lateral}
  bind:this={aside}
  onmouseenter={() => !ui.fijado && ui.mostrarLateral()}
  onmouseleave={salir}
  aria-hidden={!ui.fijado && !ui.lateral}
>
  <div class="brand">
    <Logo size={30} color="var(--menu-on-fg)" />
    <div class="brand-txt">
      <div class="word">{MARCA}</div>
      <div class="tag">{t('Hecho por IA Nativa')}</div>
    </div>
  </div>

  <label class="buscar">
    <Icon name="buscar" size={15} />
    <input
      bind:this={buscador}
      bind:value={q}
      onkeydown={enBuscador}
      onblur={() => !ui.fijado && aside && !aside.matches(':hover') && ui.ocultarLateral()}
      placeholder={t('Buscar')}
      aria-label={t('Buscar módulo')}
      spellcheck="false"
    />
    <span class="kbd">Ctrl K</span>
  </label>

  <nav class="mods" aria-label={t('Módulos')}>
    {#each grupos as g (g.titulo)}
      <div class="grupo">
        <h2 class="gtit">{g.curso ? g.titulo : t(g.titulo)}</h2>
        <ul>
          {#each g.mods as m (m.id)}
            <li>
              <button
                class="item mod"
                class:on={m.id === shell.module}
                aria-current={m.id === shell.module ? 'page' : undefined}
                title={m.solves}
                onclick={() => elegir(m.id)}
              >
                <span class="ic"><Icon name={m.curso ? (m.base ?? (m.leccion ? 'libro' : 'tabla')) : m.id} size={16} /></span>
                <span class="nm">{m.name}</span>
                {#if !m.curso}<span class="wq">{m.winqsb}</span>{/if}
              </button>
            </li>
          {/each}
        </ul>
      </div>
    {:else}
      <p class="nada">{t('Ningún módulo coincide con «{0}».', q)}</p>
    {/each}
  </nav>

  <div class="pie">
    <button class="item connect" class:on={creador.abierto} title={t('Arma un curso propio con el asistente (a partir de diapositivas, libros o programas), pruébalo y compártelo como .ncourse')} onclick={() => creador.mostrar()}>
      <span class="ic"><Icon name="nuevo" size={16} /></span>
      <span class="nm">{creador.actual ? t('Curso: {0}', creador.actual.curso?.titulo || creador.actual.id) : t('Crear un curso…')}</span>
    </button>
    <button class="item connect" title={t('Instala un curso (.ncourse) que te compartieron: sus temas aparecen aquí como módulos')} onclick={() => cursos.elegir()} disabled={cursos.instalando}>
      <span class="ic"><Icon name="abrir" size={16} /></span>
      <span class="nm">{cursos.instalando ? t('Instalando…') : t('Instalar un curso…')}</span>
    </button>
    <button class="item connect" title={t('Conectar con Claude')} onclick={() => (app.connectOpen = true)}>
      <span class="ic"><Icon name="chispa" size={16} /></span>
      <span class="nm">{t('Conectar con Claude')}</span>
    </button>
    <button class="item plegar" title={ui.fijado ? t('La barra se esconde y aparece al llevar el mouse al borde izquierdo') : t('Dejar la barra siempre visible')} onclick={() => ui.fijar()}>
      <span class="ic"><Icon name="fijar" size={15} /></span>
      <span class="nm">{ui.fijado ? t('Esconder automáticamente') : t('Fijar la barra lateral')}</span>
    </button>
  </div>
</aside>

<style>
  .side {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 10px 10px;
    background: var(--menu-bg);
    -webkit-backdrop-filter: var(--vidrio);
    backdrop-filter: var(--vidrio);
    color: var(--menu-fg);
    min-height: 0;
    overflow: hidden;
    z-index: 30;
    border-right: 0.5px solid var(--rule);
  }
  .side.flota {
    position: fixed;
    top: 8px;
    bottom: 8px;
    left: 8px;
    width: var(--side-w);
    z-index: 60;
    border: 0.5px solid var(--rule);
    border-radius: 14px;
    box-shadow: var(--shadow-pop);
    transform: translateX(calc(-100% - 20px));
    transition: transform 0.24s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .side.flota.visible {
    transform: none;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 2px 8px 4px;
  }
  .word {
    font-family: var(--font-brand);
    font-weight: 700;
    font-size: 17px;
    letter-spacing: -0.02em;
    line-height: 1.1;
  }
  .tag {
    font-size: 11px;
    color: var(--menu-fg-3);
    white-space: nowrap;
  }
  .buscar {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 6px 0 9px;
    border-radius: 7px;
    background: var(--menu-hover);
    color: var(--menu-fg-3);
  }
  .buscar:focus-within {
    box-shadow: 0 0 0 3px color-mix(in oklab, var(--focus) 35%, transparent);
  }
  .buscar input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: none;
    color: var(--menu-fg);
    font-size: 13px;
  }
  .buscar input::placeholder {
    color: var(--menu-fg-3);
  }
  .mods {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    margin: 0 -4px;
    padding: 0 4px;
  }
  .grupo + .grupo {
    margin-top: 10px;
  }
  .gtit {
    margin: 0 8px 2px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0;
    color: var(--menu-fg-3);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 28px;
    padding: 0 8px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--menu-fg);
    font-size: 13px;
    font-weight: 450;
    text-align: left;
  }
  .item:hover {
    background: var(--menu-hover);
  }
  .item.on {
    background: var(--menu-on-bg);
    font-weight: 550;
  }
  .ic {
    display: grid;
    place-items: center;
    width: 18px;
    color: var(--solver);
    flex: none;
  }
  .nm {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .wq {
    font-size: 10.5px;
    font-weight: 500;
    color: var(--menu-fg-3);
  }
  .nada {
    padding: 8px 10px;
    font-size: 12.5px;
    color: var(--menu-fg-3);
  }
  .pie {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding-top: 8px;
    border-top: 0.5px solid var(--rule);
  }
  .plegar {
    color: var(--menu-fg-3);
    font-size: 12px;
  }
  .plegar .ic {
    color: var(--menu-fg-3);
  }
</style>
