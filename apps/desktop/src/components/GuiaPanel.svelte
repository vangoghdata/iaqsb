<script lang="ts">
  // «Guíame»: qué es el módulo, para qué sirve en la vida real, historia con fuentes, qué datos
  // necesita, cómo usarlo en IAQSB, cómo lo resuelve el motor, cómo leer los resultados, qué se
  // usa hoy en la industria, alternativas, errores comunes, sección para el profesor y glosario.
  import Icon from './Icon.svelte';
  import Nativo from './chat/Nativo.svelte';
  import { guia } from '../lib/guia.svelte';
  import { chat } from '../lib/chat.svelte';
  import { shell } from '../lib/shell.svelte';
  import { app } from '../lib/state.svelte';
  import { ui } from '../lib/ui.svelte';
  import { t } from '../lib/i18n.svelte';

  const g = $derived(guia.datos);
  const info = $derived(shell.modules.find((m) => m.id === guia.modulo));

  const SECCIONES = $derived(
    [
      { id: 'que', titulo: t('Qué es'), hay: !!g?.que_es?.length },
      { id: 'sirve', titulo: t('Para qué sirve'), hay: !!g?.para_que_sirve?.length },
      { id: 'historia', titulo: t('Historia'), hay: !!g?.historia?.length },
      { id: 'datos', titulo: t('Qué necesitas'), hay: !!g?.necesitas?.length },
      { id: 'pasos', titulo: t('Cómo usarlo'), hay: !!g?.pasos?.length },
      { id: 'motor', titulo: t('Cómo lo resuelve'), hay: !!g?.como_resuelve?.length },
      { id: 'leer', titulo: t('Leer los resultados'), hay: !!g?.interpretar?.length },
      { id: 'industria', titulo: t('En la industria'), hay: !!g?.industria?.length || !!g?.alternativas?.length },
      { id: 'errores', titulo: t('Errores comunes'), hay: !!g?.errores_comunes?.length },
      { id: 'profesor', titulo: t('Para el profesor'), hay: !!g?.profesor },
      { id: 'glosario', titulo: t('Glosario'), hay: !!g?.glosario?.length },
      { id: 'refs', titulo: t('Referencias'), hay: !!g?.referencias_apa?.length }
    ].filter((s) => s.hay)
  );

  let cuerpo: HTMLElement | undefined = $state();
  let activa = $state('que');

  function ir(id: string) {
    activa = id;
    cuerpo?.querySelector(`#guia-${id}`)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }

  function alDesplazar() {
    if (!cuerpo) return;
    const arriba = cuerpo.getBoundingClientRect().top;
    let actual = SECCIONES[0]?.id ?? 'que';
    for (const s of SECCIONES) {
      const el = cuerpo.querySelector(`#guia-${s.id}`);
      if (el && el.getBoundingClientRect().top - arriba < 120) actual = s.id;
    }
    activa = actual;
  }

  const TIPO: Record<string, string> = $derived({ software: t('Software'), libreria: t('Librería'), servicio: t('Servicio'), metodologia: t('Metodología') });

  async function cargarEjemplo() {
    const m = guia.modulo;
    shell.module = m;
    if (m === 'lp') {
      app.loadExample('wyndor');
    } else {
      const st = shell.get(m);
      await st.init();
      const ex = st.examples?.[0];
      if (!ex) return;
      st.loadExample(ex.name);
    }
    guia.cerrar();
    ui.avisar(t('Ejemplo cargado: ahora pulsa Resolver y revisa el paso a paso'), 'info');
  }

  function preguntar() {
    const nombre = info ? `${info.name} (${info.winqsb})` : guia.modulo;
    guia.cerrar();
    chat.abrir();
    chat.enviar(t('Guíame por el módulo {0}: qué es, para qué sirve en una empresa, un ejemplo pequeño resuelto con IAQSB paso a paso y cómo se hace hoy en la industria.', nombre));
  }
</script>

<svelte:window onkeydown={(e) => guia.abierta && e.key === 'Escape' && guia.cerrar()} />

{#if guia.abierta}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="fondo" onclick={() => guia.cerrar()}></div>
  <div class="guia" role="dialog" aria-modal="true" aria-labelledby="guia-tit">
    <header>
      <Nativo toma={guia.cargando ? 'pensando' : 'explicando'} tam={64} titulo="Nativo" />
      <div class="tit">
        <span class="kicker">{t('Guíame')} · {info?.winqsb ?? guia.modulo}</span>
        <h2 id="guia-tit">{g?.titulo ?? info?.name ?? ''}</h2>
        {#if g?.resumen}<p class="resumen">{g.resumen}</p>{/if}
      </div>
      <div class="acc">
        <button class="btn primary" onclick={cargarEjemplo}><Icon name="jugar" size={15} />{t('Probar con un ejemplo')}</button>
        <button class="btn" onclick={preguntar}><Icon name="chat" size={15} />{t('Pregúntale a Nativo')}</button>
        <button class="btn ghost icon" aria-label={t('Cerrar')} title={t('Cerrar')} onclick={() => guia.cerrar()}><Icon name="cerrar" size={16} /></button>
      </div>
    </header>

    {#if guia.cargando}
      <p class="estado">{t('Cargando la guía…')}</p>
    {:else if guia.error}
      <p class="estado err">{guia.error}</p>
    {:else if g}
      <div class="cuerpo-guia">
        <nav aria-label={t('Secciones de la guía')}>
          {#each SECCIONES as s (s.id)}
            <button class:on={activa === s.id} onclick={() => ir(s.id)}>{s.titulo}</button>
          {/each}
        </nav>
        <div class="texto" bind:this={cuerpo} onscroll={alDesplazar}>
          {#if g.que_es?.length}
            <section id="guia-que">
              <h3>{t('Qué es')}</h3>
              {#each g.que_es as p, i (i)}<p>{p}</p>{/each}
              {#if g.analogia}<blockquote>{g.analogia}</blockquote>{/if}
              {#if g.supuestos?.length}
                <h4>{t('Supuestos y límites')}</h4>
                <ul>{#each g.supuestos as x, i (i)}<li>{x}</li>{/each}</ul>
              {/if}
            </section>
          {/if}
          {#if g.para_que_sirve?.length}
            <section id="guia-sirve">
              <h3>{t('Para qué sirve en la vida real')}</h3>
              <div class="tarjetas">
                {#each g.para_que_sirve as x, i (i)}
                  <div class="tarjeta"><strong>{x.sector}</strong><p>{x.ejemplo}</p></div>
                {/each}
              </div>
            </section>
          {/if}
          {#if g.historia?.length}
            <section id="guia-historia">
              <h3>{t('Historia')}</h3>
              <ol class="linea">
                {#each g.historia as x, i (i)}
                  <li>
                    <span class="anio">{x.anio}</span>
                    <span>{x.hecho}{#if x.citas?.length}{' '}<span class="cita">({x.citas.map((c) => c.cita).join('; ')})</span>{/if}</span>
                  </li>
                {/each}
              </ol>
            </section>
          {/if}
          {#if g.necesitas?.length}
            <section id="guia-datos">
              <h3>{t('Qué necesitas')}</h3>
              <dl>
                {#each g.necesitas as x, i (i)}<dt>{x.dato}</dt><dd>{x.para_que}</dd>{/each}
              </dl>
            </section>
          {/if}
          {#if g.pasos?.length}
            <section id="guia-pasos">
              <h3>{t('Cómo usarlo en IAQSB')}</h3>
              <ol class="pasos">
                {#each g.pasos as x, i (i)}<li><strong>{x.titulo}</strong><p>{x.texto}</p></li>{/each}
              </ol>
            </section>
          {/if}
          {#if g.como_resuelve?.length}
            <section id="guia-motor">
              <h3>{t('Cómo lo resuelve IAQSB')}</h3>
              {#each g.como_resuelve as p, i (i)}<p>{p}</p>{/each}
            </section>
          {/if}
          {#if g.interpretar?.length}
            <section id="guia-leer">
              <h3>{t('Cómo leer los resultados')}</h3>
              <ul>{#each g.interpretar as x, i (i)}<li>{x}</li>{/each}</ul>
            </section>
          {/if}
          {#if g.industria?.length || g.alternativas?.length}
            <section id="guia-industria">
              <h3>{t('En la industria hoy')}</h3>
              {#if g.industria?.length}
                <div class="tarjetas">
                  {#each g.industria as x, i (i)}
                    <div class="tarjeta"><span class="tipo">{TIPO[x.tipo] ?? x.tipo}</span><strong>{x.nombre}</strong><p>{x.uso}</p></div>
                  {/each}
                </div>
              {/if}
              {#if g.alternativas?.length}
                <h4>{t('Alternativas y enfoques modernos')}</h4>
                <ul>{#each g.alternativas as x, i (i)}<li>{x}</li>{/each}</ul>
              {/if}
            </section>
          {/if}
          {#if g.errores_comunes?.length}
            <section id="guia-errores">
              <h3>{t('Errores comunes')}</h3>
              <ul>{#each g.errores_comunes as x, i (i)}<li>{x}</li>{/each}</ul>
            </section>
          {/if}
          {#if g.profesor}
            <section id="guia-profesor">
              <h3>{t('Para el profesor')}</h3>
              {#if g.profesor.objetivos?.length}<h4>{t('Objetivos de aprendizaje')}</h4><ul>{#each g.profesor.objetivos as x, i (i)}<li>{x}</li>{/each}</ul>{/if}
              {#if g.profesor.actividades?.length}<h4>{t('Actividades de clase')}</h4><ul>{#each g.profesor.actividades as x, i (i)}<li>{x}</li>{/each}</ul>{/if}
              {#if g.profesor.preguntas?.length}<h4>{t('Preguntas de evaluación')}</h4><ul>{#each g.profesor.preguntas as x, i (i)}<li>{x}</li>{/each}</ul>{/if}
            </section>
          {/if}
          {#if g.glosario?.length}
            <section id="guia-glosario">
              <h3>{t('Glosario')}</h3>
              <dl>{#each g.glosario as x, i (i)}<dt>{x.termino}</dt><dd>{x.definicion}</dd>{/each}</dl>
            </section>
          {/if}
          {#if g.referencias_apa?.length}
            <section id="guia-refs">
              <h3>{t('Referencias (APA 7)')}</h3>
              <ul class="refs">
                {#each g.referencias_apa as r (r.id)}
                  <li>{r.apa.replace(/\*/g, '')}{#if !r.verificada} <span class="pv">{t('por verificar')}</span>{/if}</li>
                {/each}
              </ul>
            </section>
          {/if}
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .fondo {
    position: fixed;
    inset: 0;
    background: rgba(10, 14, 28, 0.38);
    z-index: 80;
  }
  .guia {
    position: fixed;
    z-index: 81;
    inset: 4vh max(16px, calc(50vw - 520px));
    display: flex;
    flex-direction: column;
    background: var(--paper, var(--sheet, #fff));
    color: var(--ink);
    border: 0.5px solid var(--rule);
    border-radius: 14px;
    box-shadow: 0 24px 64px rgba(0, 0, 0, 0.28);
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 18px;
    border-bottom: 0.5px solid var(--rule);
    background: linear-gradient(135deg, var(--solver-tint, #eef1ff), transparent 70%);
  }
  .tit {
    flex: 1;
    min-width: 0;
  }
  .kicker {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--solver);
  }
  h2 {
    margin: 2px 0 2px;
    font-size: 19px;
  }
  .resumen {
    margin: 0;
    font-size: 13px;
    color: var(--muted, #6b7280);
  }
  .acc {
    display: flex;
    gap: 6px;
    align-items: center;
    flex-wrap: wrap;
    justify-content: flex-end;
  }
  .estado {
    padding: 24px;
  }
  .err {
    color: var(--bad, #c0392b);
  }
  .cuerpo-guia {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 190px 1fr;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 12px 8px;
    border-right: 0.5px solid var(--rule);
    overflow: auto;
  }
  nav button {
    text-align: left;
    background: none;
    border: 0;
    border-radius: 8px;
    padding: 7px 10px;
    font: inherit;
    font-size: 13px;
    color: var(--ink);
    cursor: pointer;
  }
  nav button:hover {
    background: var(--hover, rgba(127, 127, 127, 0.12));
  }
  nav button.on {
    background: var(--solver-tint);
    color: var(--solver);
    font-weight: 600;
  }
  .texto {
    overflow: auto;
    padding: 6px 28px 40px;
    font-size: 14px;
    line-height: 1.6;
  }
  section {
    padding-top: 14px;
    max-width: 78ch;
  }
  h3 {
    font-size: 16px;
    margin: 10px 0 8px;
  }
  h4 {
    font-size: 13.5px;
    margin: 14px 0 4px;
  }
  blockquote {
    margin: 10px 0;
    padding: 10px 14px;
    border-left: 3px solid var(--solver);
    background: var(--solver-tint);
    border-radius: 0 8px 8px 0;
  }
  .tarjetas {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: 10px;
  }
  .tarjeta {
    border: 0.5px solid var(--rule);
    border-radius: 10px;
    padding: 10px 12px;
    background: var(--sheet);
  }
  .tarjeta p {
    margin: 4px 0 0;
    font-size: 13px;
  }
  .tipo {
    display: inline-block;
    font-size: 10.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--solver);
    margin-bottom: 2px;
  }
  .tarjeta strong {
    display: block;
  }
  .linea {
    list-style: none;
    padding: 0;
    margin: 0;
    border-left: 2px solid var(--solver-tint);
  }
  .linea li {
    display: grid;
    grid-template-columns: 112px 1fr;
    gap: 8px;
    padding: 6px 0 6px 12px;
    position: relative;
  }
  .linea li::before {
    content: '';
    position: absolute;
    left: -6px;
    top: 13px;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--solver);
  }
  .anio {
    font-weight: 700;
    color: var(--solver);
  }
  .cita {
    color: var(--muted, #6b7280);
    font-size: 12.5px;
  }
  dl {
    margin: 0;
  }
  dt {
    font-weight: 600;
    margin-top: 8px;
  }
  dd {
    margin: 2px 0 0;
  }
  .pasos li {
    margin-bottom: 8px;
  }
  .pasos p {
    margin: 2px 0 0;
  }
  .refs li {
    margin-bottom: 6px;
    font-size: 13px;
  }
  .pv {
    font-size: 11px;
    color: var(--warn, #b7791f);
    border: 0.5px solid currentColor;
    border-radius: 4px;
    padding: 0 4px;
  }
  @media (max-width: 1000px) {
    header {
      flex-wrap: wrap;
    }
    .acc {
      width: 100%;
      justify-content: flex-start;
    }
  }
  @media (max-width: 760px) {
    .cuerpo-guia {
      grid-template-columns: 1fr;
    }
    nav {
      display: none;
    }
    .guia {
      inset: 0;
      border-radius: 0;
    }
  }
</style>
