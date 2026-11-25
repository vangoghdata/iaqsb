<script lang="ts">
  import { MARCA } from '../../lib/marca';
  // Ajustes de IAQSB: idioma, nivel, apariencia, números, motor, archivos,
  // informes, inteligencia artificial, atajos y «acerca de» (cómo citar, licencia).
  import Icon from '../Icon.svelte';
  import SelectorTemaCompacto from '../tema/SelectorTemaCompacto.svelte';
  import ConexionIA from './ConexionIA.svelte';
  import { ajustes, VERSION } from '../../lib/ajustes.svelte';
  import { motor, type Metodo } from '../../lib/motor.svelte';
  import { shell } from '../../lib/shell.svelte';
  import { app } from '../../lib/state.svelte';
  import { ui } from '../../lib/ui.svelte';
  import { sesion } from '../../lib/sesion.svelte';
  import { recientes, nombreArchivo } from '../../lib/recientes.svelte';
  import { inTauri } from '../../lib/backend';
  import { i18n, IDIOMAS, t, type Lang } from '../../lib/i18n.svelte';
  import { num } from '../../lib/format';
  import type { NombreIcono } from '../../lib/iconos';

  type Seccion = { id: string; titulo: string; icono: NombreIcono };
  const SECCIONES: Seccion[] = $derived([
    { id: 'general', titulo: t('General'), icono: 'globo' },
    { id: 'apariencia', titulo: t('Apariencia'), icono: 'paleta' },
    { id: 'numeros', titulo: t('Números'), icono: 'tabla' },
    { id: 'motor', titulo: t('Motor de optimización'), icono: 'motor' },
    { id: 'archivos', titulo: t('Archivos'), icono: 'abrir' },
    { id: 'informes', titulo: t('Informes'), icono: 'documento' },
    { id: 'ia', titulo: t('Inteligencia artificial'), icono: 'chispa' },
    { id: 'atajos', titulo: t('Atajos de teclado'), icono: 'teclado' },
    { id: 'acerca', titulo: t('Acerca de IAQSB'), icono: 'info' }
  ]);

  const DESCRIPCION_IDIOMA: Record<Lang, string> = {
    es: 'Interfaz, informes, ejemplos y fuentes en español.',
    en: 'Interface, reports, examples and sources in English.',
    pt: 'Interface, relatórios, exemplos e fontes em português do Brasil.'
  };

  function cerrar() {
    // lo escrito en un campo que todavía tiene el foco también se guarda
    ajustes.guardar();
    ajustes.abierta = false;
  }

  function guardar() {
    ajustes.guardar();
  }

  function elegirIdioma(l: Lang) {
    void shell.cambiarIdioma(l);
  }

  const METODOS: { id: Metodo; label: string }[] = $derived([
    { id: 'auto', label: t('Automático') },
    { id: 'dual_simplex', label: t('Símplex dual') },
    { id: 'primal_simplex', label: t('Símplex primal') },
    { id: 'barrier', label: t('Punto interior (barrera)') },
    { id: 'pdlp', label: t('PDLP (primer orden, modelos enormes)') }
  ]);

  function numero(v: string, entero = false): number | null {
    const x = entero ? parseInt(v, 10) : parseFloat(v.replace(',', '.'));
    return Number.isFinite(x) && x > 0 ? x : null;
  }

  const ATAJOS: [string, string][] = $derived([
    ['Ctrl + Intro', t('Resolver')],
    ['Ctrl + S', t('Guardar')],
    ['Ctrl + O', t('Abrir')],
    ['Ctrl + Z', t('Deshacer')],
    ['Ctrl + Y · Ctrl + Mayús + Z', t('Rehacer')],
    ['Ctrl + V', t('Pegar una tabla de Excel en la hoja (desde la celda elegida)')],
    ['Ctrl + K', t('Mostrar los módulos')],
    ['Ctrl + ,', t('Abrir los ajustes')],
    ['Ctrl + /', t('Comentar líneas (modelo algebraico)')],
    ['Intro · ↑ ↓', t('Moverse entre celdas, como en una hoja de cálculo')],
    ['Esc', t('Cerrar menús y diálogos')]
  ]);

  // APA 7: el título va en su idioma original; «Versión» sí se traduce
  const CITA_APA = $derived(`IA Nativa. (2026). *${MARCA}: para aprender y aplicar la ingeniería industrial con IA* (${t('Versión')} ${VERSION}) [Software].`);
  const CITA_BIB = `@software{iaqsb2026,\n  author = {{IA Nativa}},\n  year = {2026},\n  version = {${VERSION}},\n  title = {{${MARCA}}: para aprender y aplicar la ingeniería industrial con {IA}},\n  note = {Licencia MIT}\n}`;

  async function copiar(texto: string) {
    try {
      await navigator.clipboard.writeText(texto.replace(/\*/g, ''));
      ui.avisar(t('Copiado'), 'ok');
    } catch {
      ui.avisar(t('No se pudo copiar'), 'error');
    }
  }

  function cuando(ts: number | null): string {
    if (!ts) return t('todavía no');
    return new Date(ts).toLocaleTimeString(i18n.lang === 'en' ? 'en-US' : i18n.lang === 'pt' ? 'pt-BR' : 'es-CO', { timeStyle: 'short' });
  }

  function teclas(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === ',') {
      e.preventDefault();
      ajustes.abierta ? cerrar() : ajustes.abrir();
    } else if (e.key === 'Escape' && ajustes.abierta && !ui.apariencia) {
      cerrar();
    }
  }
</script>

<svelte:window onkeydown={teclas} />

{#if ajustes.abierta}
  <div class="capa" role="dialog" aria-modal="true" aria-labelledby="ajustes-tit">
    <div class="pagina">
      <nav class="lado" aria-label={t('Secciones de los ajustes')}>
        <h2 id="ajustes-tit">{t('Ajustes')}</h2>
        {#each SECCIONES as s (s.id)}
          <button class="item" class:on={ajustes.seccion === s.id} aria-current={ajustes.seccion === s.id ? 'page' : undefined} onclick={() => (ajustes.seccion = s.id)}>
            <Icon name={s.icono} size={16} />{s.titulo}
          </button>
        {/each}
      </nav>

      <section class="contenido">
        <header class="cab">
          <h3>{SECCIONES.find((s) => s.id === ajustes.seccion)?.titulo}</h3>
          <button class="btn ghost icon" onclick={cerrar} aria-label={t('Cerrar')} title={t('Cerrar (Esc)')}><Icon name="cerrar" size={18} /></button>
        </header>

        {#if ajustes.seccion === 'general'}
          <div class="grupo">
            <h4>{t('Idioma')}</h4>
            <p class="ayuda">{t('Cambia toda la app: la interfaz, los informes, los ejemplos, las fuentes (APA 7) y las gráficas que se descargan. El problema abierto se vuelve a resolver en el idioma nuevo.')}</p>
            <div class="tarjetas" role="radiogroup" aria-label={t('Idioma')}>
              {#each IDIOMAS as l (l.id)}
                <button role="radio" aria-checked={i18n.lang === l.id} class="tarjeta" class:on={i18n.lang === l.id} lang={l.id} onclick={() => elegirIdioma(l.id)}>
                  <span class="tt">{l.nombre}</span>
                  <span class="td">{DESCRIPCION_IDIOMA[l.id]}</span>
                </button>
              {/each}
            </div>
            {#if shell.cambiando}<p class="estado"><span class="spinner" aria-hidden="true"></span>{t('Trayendo los textos en el idioma nuevo…')}</p>{/if}
          </div>
          <div class="grupo">
            <h4>{t('Nivel')}</h4>
            <p class="ayuda">{t('Ajusta cómo explica el asistente de IA: en pregrado, paso a paso y en lenguaje sencillo; en posgrado, técnico y conciso.')}</p>
            <div class="tarjetas" role="radiogroup" aria-label={t('Nivel')}>
              <button role="radio" aria-checked={ajustes.nivel === 'pregrado'} class="tarjeta" class:on={ajustes.nivel === 'pregrado'} onclick={() => { ajustes.nivel = 'pregrado'; guardar(); }}>
                <span class="tt">{t('Pregrado')}</span><span class="td">{t('Explicaciones paso a paso, cada término definido y el sentido de cada número.')}</span>
              </button>
              <button role="radio" aria-checked={ajustes.nivel === 'posgrado'} class="tarjeta" class:on={ajustes.nivel === 'posgrado'} onclick={() => { ajustes.nivel = 'posgrado'; guardar(); }}>
                <span class="tt">{t('Posgrado e investigación')}</span><span class="td">{t('Terminología precisa (dualidad, condiciones de optimalidad, complejidad), supuestos y literatura.')}</span>
              </button>
            </div>
          </div>
          <label class="check"><input type="checkbox" bind:checked={ajustes.pasos} onchange={() => { app.showSteps = ajustes.pasos; guardar(); }} />{t('Guardar el paso a paso al resolver (tablas del símplex, iteraciones)')}</label>
        {:else if ajustes.seccion === 'apariencia'}
          <div class="grupo">
            <h4>{t('Tema')}</h4>
            <p class="ayuda">{t('Claro, oscuro, el del computador o tus propios colores y fondo.')}</p>
            <div class="fila-tema"><SelectorTemaCompacto /><button class="btn" onclick={() => (ui.apariencia = true)}><Icon name="paleta" size={15} />{t('Personalizar colores y fondo…')}</button></div>
          </div>
          <label class="check"><input type="checkbox" checked={ui.fijado} onchange={(e) => ui.fijar(e.currentTarget.checked)} />{t('Barra lateral de módulos siempre visible')}</label>
          <label class="check"><input type="checkbox" bind:checked={ajustes.nativo} onchange={() => guardar()} />{t('Animaciones de Nativo, el robot del asistente (piensa, calcula, explica y celebra)')}</label>
        {:else if ajustes.seccion === 'numeros'}
          <div class="grupo">
            <h4>{t('Decimales')}</h4>
            <p class="ayuda">{t('Cuántos decimales se muestran en los resultados (los cálculos usan siempre toda la precisión).')}</p>
            <div class="fila">
              <input type="range" min="0" max="8" bind:value={ajustes.decimales} oninput={guardar} aria-label={t('Decimales')} />
              <span class="valor">{ajustes.decimales}</span>
              <span class="muestra">{t('Ejemplo')}: {num(2 / 3)} · {num(1234.56789)}</span>
            </div>
          </div>
          <div class="grupo">
            <h4>{t('Separador decimal')}</h4>
            <div class="tarjetas" role="radiogroup" aria-label={t('Separador decimal')}>
              {#each [['auto', t('Punto en la pantalla (como WinQSB)'), '3.5'], ['coma', t('Coma (Colombia, Brasil y casi toda Latinoamérica)'), '3,5'], ['punto', t('Punto siempre, también en los informes'), '3.5']] as [v, d, m] (v)}
                <button role="radio" aria-checked={ajustes.separador === v} class="tarjeta" class:on={ajustes.separador === v} onclick={() => { ajustes.separador = v as typeof ajustes.separador; guardar(); }}>
                  <span class="tt">{m}</span><span class="td">{d}</span>
                </button>
              {/each}
            </div>
            <p class="ayuda">{t('Al escribir o pegar datos se acepta tanto la coma como el punto.')}</p>
          </div>
        {:else if ajustes.seccion === 'motor'}
          <div class="grupo">
            <p class="ayuda">{t('Vale para todos los módulos. HiGHS viene incluido; Gurobi, CPLEX, SCIP, CBC y GLPK se usan si están instalados en este equipo (con su propia licencia).')}</p>
            <div class="form">
              <label>{t('Motor')}
                <select bind:value={motor.solver} onchange={() => motor.guardar()} onfocus={() => motor.init()}>
                  {#if !motor.solvers.length}<option value="highs">HiGHS</option>{/if}
                  {#each motor.solvers as s (s.id)}<option value={s.id}>{s.name}</option>{/each}
                </select>
              </label>
              <label>{t('Tiempo máximo (s)')}
                <input type="number" min="0" step="1" placeholder={t('sin límite')} value={motor.tiempo ?? ''} oninput={(e) => { motor.tiempo = numero(e.currentTarget.value); motor.guardar(); }} />
              </label>
              <label>{t('Brecha MIP (%)')}
                <input type="number" min="0" step="0.01" placeholder={t('la del motor')} value={motor.brecha ?? ''} oninput={(e) => { const v = e.currentTarget.value.trim(); motor.brecha = v === '' ? null : Math.max(0, parseFloat(v.replace(',', '.')) || 0); motor.guardar(); }} />
              </label>
              <label>{t('Hilos')}
                <input type="number" min="1" step="1" placeholder={t('automático')} value={motor.hilos ?? ''} oninput={(e) => { motor.hilos = numero(e.currentTarget.value, true); motor.guardar(); }} />
              </label>
              <label>{t('Método (LP)')}
                <select bind:value={motor.metodo} onchange={() => motor.guardar()}>
                  {#each METODOS as m (m.id)}<option value={m.id}>{m.label}</option>{/each}
                </select>
              </label>
            </div>
            <label class="check"><input type="checkbox" bind:checked={motor.registro} onchange={() => motor.guardar()} />{t('Mostrar el registro del motor')}</label>
            <p class="ayuda">{t('Para ajustar los parámetros automáticamente a un modelo, usa «Ajustar para este modelo» en el botón del motor de la barra de arriba.')}</p>
            <button class="btn" onclick={() => motor.restablecer()} disabled={!motor.personalizado}>{t('Restablecer')}</button>
          </div>
        {:else if ajustes.seccion === 'archivos'}
          <div class="grupo">
            <h4>{t('Autoguardado')}</h4>
            <label class="check"><input type="checkbox" bind:checked={ajustes.autoguardado} onchange={() => { guardar(); if (ajustes.autoguardado) sesion.programar(0); }} />{t('Guardar el trabajo en este equipo mientras trabajas y recuperarlo al abrir IAQSB')}</label>
            <p class="ayuda">{t('Último autoguardado: {0}. No reemplaza a «Guardar»: tus archivos de WinQSB, Excel o IAQSB se guardan solo cuando tú lo pides.', cuando(sesion.ultimo))}</p>
            <button class="btn" onclick={() => { sesion.borrar(); ui.avisar(t('Se borró el trabajo autoguardado'), 'info'); }}>{t('Borrar el trabajo autoguardado')}</button>
          </div>
          <div class="grupo">
            <h4>{t('Archivos recientes')}</h4>
            {#if !inTauri}<p class="ayuda">{t('La lista de archivos recientes está en la aplicación de escritorio.')}</p>{/if}
            {#each recientes.lista as r (r.path)}
              <div class="reciente"><span title={r.path}>{nombreArchivo(r.path)}</span><span class="ruta">{r.path}</span><button class="btn ghost icon" aria-label={t('Quitar de la lista')} onclick={() => recientes.quitar(r.path)}><Icon name="cerrar" size={14} /></button></div>
            {:else}
              <p class="ayuda">{t('Todavía no hay archivos recientes.')}</p>
            {/each}
            {#if recientes.lista.length}<button class="btn" onclick={() => recientes.limpiar()}>{t('Borrar la lista')}</button>{/if}
          </div>
        {:else if ajustes.seccion === 'informes'}
          <div class="grupo">
            <h4>{t('Portada de los informes')}</h4>
            <p class="ayuda">{t('Datos que aparecen en la portada de los informes de Word, PDF y HTML (se pueden cambiar al exportar).')}</p>
            <div class="form">
              <label>{t('Autor o grupo')}<input bind:value={ajustes.autor} oninput={guardar} placeholder={t('p. ej. Grupo 8')} /></label>
              <label>{t('Curso')}<input bind:value={ajustes.curso} oninput={guardar} placeholder={t('p. ej. Taller de Producción y Logística')} /></label>
              <label>{t('Institución')}<input bind:value={ajustes.institucion} oninput={guardar} placeholder={t('p. ej. Universidad Nacional de Colombia')} /></label>
            </div>
            <p class="ayuda">{t('Las tablas y figuras se numeran al estilo APA 7 y las referencias van con sangría francesa.')}</p>
          </div>
        {:else if ajustes.seccion === 'ia'}
          <div class="grupo">
            <h4>{t('Conectar la API del asistente')}</h4>
            <p class="ayuda">{t('El asistente funciona con el modelo que elijas: Claude, ChatGPT, Gemini, Qwen, DeepSeek, Llama, Mistral, Grok o cualquier otro, con OpenRouter, con la API de cada empresa, con un modelo en tu propio equipo (Ollama, LM Studio) o con el servidor de tu universidad. Con los modelos que usan herramientas, resuelve, compara y explica con el mismo motor de IAQSB.')}</p>
            <ConexionIA />
          </div>
          <div class="grupo">
            <h4>{t('Claude Desktop y otras IA (MCP)')}</h4>
            <p class="ayuda">{t('IAQSB trae un servidor MCP: Claude Desktop (u otra IA compatible) puede plantear y resolver problemas con IAQSB, escribir modelos en el lenguaje algebraico y exportar informes.')}</p>
            <button class="btn" onclick={() => { app.connectOpen = true; }}><Icon name="chispa" size={15} />{t('Conectar con Claude Desktop…')}</button>
          </div>
        {:else if ajustes.seccion === 'atajos'}
          <table class="atajos">
            <tbody>
              {#each ATAJOS as [k, d] (k)}
                <tr><td><kbd>{k}</kbd></td><td>{d}</td></tr>
              {/each}
            </tbody>
          </table>
          <p class="ayuda">{t('En macOS, Cmd en lugar de Ctrl.')}</p>
        {:else if ajustes.seccion === 'acerca'}
          <div class="grupo">
            <h4>{MARCA} {VERSION}</h4>
            <p>{t('Investigación de operaciones nativa para IA. Reimplementa los 19 módulos de WinQSB, abre y guarda sus archivos y agrega módulos de logística, un motor moderno (HiGHS, con conectores para Gurobi, CPLEX, SCIP, CBC y GLPK), un lenguaje de modelado algebraico y un servidor MCP para la IA.')}</p>
            <p>{t('Hecho por IA Nativa. Código abierto con licencia MIT: puedes usarlo, estudiarlo, modificarlo y compartirlo en tu universidad, de pregrado a doctorado.')}</p>
          </div>
          <div class="grupo">
            <h4>{t('Cómo citar IAQSB')}</h4>
            <p class="cita">{@html CITA_APA.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/\*([^*]+)\*/g, '<em>$1</em>')}</p>
            <div class="fila">
              <button class="btn" onclick={() => copiar(CITA_APA)}>{t('Copiar en APA')}</button>
              <button class="btn" onclick={() => copiar(CITA_BIB)}>{t('Copiar BibTeX')}</button>
            </div>
            <p class="ayuda">{t('Cada informe cita además los métodos y los datos que usa (APA 7, con su DOI cuando existe).')}</p>
          </div>
        {/if}
      </section>
    </div>
  </div>
{/if}

<style>
  .capa {
    position: fixed;
    inset: 0;
    z-index: 70;
    display: grid;
    place-items: center;
    padding: 24px;
    background: color-mix(in oklab, #000 35%, transparent);
    -webkit-backdrop-filter: blur(6px);
    backdrop-filter: blur(6px);
    animation: subir 0.14s ease-out;
  }
  .pagina {
    display: grid;
    grid-template-columns: 220px 1fr;
    width: min(980px, 100%);
    height: min(680px, 100%);
    background: var(--sheet);
    color: var(--ink);
    border: 0.5px solid var(--rule);
    border-radius: var(--r-lg, 16px);
    box-shadow: var(--shadow-pop);
    overflow: hidden;
  }
  .lado {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 18px 10px;
    background: color-mix(in oklab, var(--ink) 3%, var(--sheet));
    border-right: 0.5px solid var(--rule);
    overflow: auto;
  }
  .lado h2 {
    font-size: 18px;
    margin: 0 10px 12px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 8px 10px;
    border: 0;
    border-radius: 9px;
    background: none;
    text-align: left;
    font-size: 13.5px;
    color: var(--ink-2);
  }
  .item:hover {
    background: color-mix(in oklab, var(--ink) 6%, transparent);
    color: var(--ink);
  }
  .item.on {
    background: var(--solver-tint);
    color: var(--solver);
    font-weight: 600;
  }
  .contenido {
    overflow: auto;
    padding: 18px 26px 28px;
  }
  .cab {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }
  .cab h3 {
    font-size: 20px;
  }
  .grupo {
    display: grid;
    gap: 8px;
    padding: 14px 0 18px;
    border-bottom: 0.5px solid var(--rule);
  }
  .grupo p {
    margin: 0;
  }
  .grupo:last-child {
    border-bottom: 0;
  }
  h4 {
    margin: 0;
    font-size: 14.5px;
  }
  .ayuda {
    margin: 0;
    font-size: 12.5px;
    color: var(--ink-3);
    line-height: 1.45;
    max-width: 70ch;
  }
  .tarjetas {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
    gap: 8px;
  }
  .tarjeta {
    display: grid;
    gap: 3px;
    text-align: left;
    padding: 11px 13px;
    border: 1px solid var(--rule);
    border-radius: 12px;
    background: var(--sheet);
  }
  .tarjeta:hover {
    border-color: color-mix(in oklab, var(--solver) 45%, var(--rule));
  }
  .tarjeta.on {
    border-color: var(--solver);
    box-shadow: 0 0 0 1px var(--solver) inset;
    background: color-mix(in oklab, var(--solver) 6%, var(--sheet));
  }
  .tt {
    font-weight: 600;
    font-size: 14px;
  }
  .td {
    font-size: 12px;
    color: var(--ink-3);
    line-height: 1.35;
  }
  .estado {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 12.5px;
    color: var(--ink-2);
  }
  .check {
    display: flex;
    gap: 9px;
    align-items: flex-start;
    font-size: 13.5px;
    padding: 6px 0;
  }
  .check input {
    margin-top: 3px;
  }
  .fila {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .fila-tema {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    --barra-fg: var(--ink);
    --barra-fg-2: var(--ink-2);
  }
  .valor {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    min-width: 2ch;
  }
  .muestra {
    color: var(--ink-3);
    font-size: 12.5px;
    font-variant-numeric: tabular-nums;
  }
  .form {
    display: grid;
    gap: 10px;
    max-width: 520px;
  }
  .form label {
    display: grid;
    gap: 4px;
    font-size: 12.5px;
    color: var(--ink-2);
  }
  .form input,
  .form select {
    height: 32px;
    padding: 0 9px;
    border: 1px solid var(--rule);
    border-radius: var(--r-sm);
    background: var(--sheet);
    font-size: 13.5px;
    color: var(--ink);
  }
  .reciente {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 10px;
    font-size: 13px;
  }
  .ruta {
    color: var(--ink-3);
    font-size: 11.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .atajos {
    border-collapse: collapse;
    font-size: 13.5px;
  }
  .atajos td {
    padding: 7px 14px 7px 0;
    border-bottom: 0.5px solid var(--rule);
  }
  kbd {
    font-family: inherit;
    font-size: 12px;
    padding: 2px 7px;
    border: 1px solid var(--rule);
    border-bottom-width: 2px;
    border-radius: 6px;
    background: color-mix(in oklab, var(--ink) 3%, var(--sheet));
    white-space: nowrap;
  }
  .cita {
    margin: 0;
    padding: 10px 12px;
    border-left: 3px solid var(--solver);
    background: color-mix(in oklab, var(--ink) 3%, var(--sheet));
    font-size: 13px;
  }
  @media (max-width: 760px) {
    .pagina {
      grid-template-columns: 1fr;
      grid-template-rows: auto 1fr;
    }
    .lado {
      flex-direction: row;
      flex-wrap: wrap;
      border-right: 0;
      border-bottom: 0.5px solid var(--rule);
    }
    .lado h2 {
      width: 100%;
    }
  }
</style>
