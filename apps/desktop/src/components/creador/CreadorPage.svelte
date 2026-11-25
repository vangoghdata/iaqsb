<script lang="ts">
  // Crear un curso: los borradores, la estructura del curso, el editor de archivos, la
  // validación, las pruebas de los ejemplos, la vista previa y la exportación a .ncourse.
  // El asistente de IA (panel de Nativo, modo creador) trabaja sobre el mismo borrador.
  import Icon from '../Icon.svelte';
  import Nativo from '../chat/Nativo.svelte';
  import Markdown from '../chat/Markdown.svelte';
  import { creador, extension } from '../../lib/creador.svelte';
  import { cursos } from '../../lib/cursos.svelte';
  import { chat } from '../../lib/chat.svelte';
  import { ajustes } from '../../lib/ajustes.svelte';
  import { t } from '../../lib/i18n.svelte';
  import { MARCA } from '../../lib/marca';
  import { onMount } from 'svelte';

  onMount(() => {
    void creador.listar();
    void cursos.cargar();
  });

  let titulo = $state('');
  let desde = $state<'vacio' | 'plantilla'>('vacio');
  let temaTitulo = $state('');
  /** «nuevo:leccion|python|r|algebra», «modulo:<id>» o «c.<curso>.<tema>» (biblioteca). */
  let temaTipo = $state('nuevo:leccion');
  const desdeBiblioteca = $derived(!temaTipo.startsWith('nuevo:'));
  /** Los temas de la biblioteca agrupados por curso (sin el que se está editando). */
  const porCurso = $derived.by(() => {
    const m = new Map<string, typeof creador.biblioteca.temas>();
    for (const x of creador.biblioteca.temas) {
      if (x.curso === b?.instalado_como) continue;
      m.set(x.curso_titulo, [...(m.get(x.curso_titulo) ?? []), x]);
    }
    return [...m];
  });
  let agregandoTema = $state(false);
  let vista = $state(false);
  let selector: HTMLInputElement | undefined = $state();
  let carpetaSubir = $state('');

  const b = $derived(creador.actual);
  const a = $derived(creador.archivo);
  const ext = $derived(a ? extension(a.ruta) : '');
  const imagen = $derived(a?.binario && ['png', 'jpg', 'jpeg', 'webp', 'gif'].includes(ext));
  const mime = $derived(ext === 'jpg' ? 'image/jpeg' : `image/${ext}`);

  /** Archivos de cada tema y los del curso (raíz). */
  const arbol = $derived.by(() => {
    const porTema = new Map<string, { ruta: string; nombre: string; bytes: number }[]>();
    const raiz: { ruta: string; nombre: string; bytes: number }[] = [];
    for (const f of b?.archivos ?? []) {
      const m = f.ruta.match(/^temas\/([^/]+)\/(.+)$/);
      if (m) porTema.set(m[1], [...(porTema.get(m[1]) ?? []), { ruta: f.ruta, nombre: m[2], bytes: f.bytes }]);
      else raiz.push({ ruta: f.ruta, nombre: f.ruta, bytes: f.bytes });
    }
    return { porTema, raiz };
  });

  const TIPOS: Record<string, string> = {
    leccion: 'Lección',
    calculadora: 'Calculadora',
    modelo: 'Modelo',
    modulo: 'Módulo de Nativo'
  };
  const LENGUAJES: Record<string, string> = { python: 'Python', r: 'R', algebra: 'Algebraico' };

  const SUGERENCIAS = $derived([
    t('Arma un tema de MRP II a partir de las diapositivas que te adjunto: lección, guía con historia y bibliografía, y una calculadora.'),
    t('Convierte el programa de Python que te adjunto en una calculadora del curso, con su pantalla de datos y un ejemplo.'),
    t('Escribe la guía (Guíame) de cada tema: qué es, usos reales, historia con fuentes, software de la industria y errores comunes.'),
    t('Revisa el curso: valida, prueba los ejemplos, comprueba los números y dime qué falta.')
  ]);

  function pedir(texto: string) {
    chat.abrir();
    if (chat.tieneClave && !chat.ocupado) void chat.enviar(texto);
    else chat.borrador = texto;
  }

  function teclas(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
      e.preventDefault();
      void creador.guardarArchivo();
    }
    // tabulación dentro del editor
    if (e.key === 'Tab' && !e.shiftKey && creador.archivo) {
      const ta = e.currentTarget as HTMLTextAreaElement;
      e.preventDefault();
      const i = ta.selectionStart;
      creador.archivo.texto = ta.value.slice(0, i) + '  ' + ta.value.slice(ta.selectionEnd);
      requestAnimationFrame(() => (ta.selectionStart = ta.selectionEnd = i + 2));
    }
  }

  function subir(carpeta: string) {
    carpetaSubir = carpeta;
    selector?.click();
  }

  function alSubir(e: Event) {
    const inp = e.currentTarget as HTMLInputElement;
    if (inp.files?.length) void creador.agregarArchivos([...inp.files], carpetaSubir);
    inp.value = '';
  }

  function crearTema() {
    if (desdeBiblioteca) void creador.importarTema(temaTipo, temaTitulo);
    else if (temaTitulo.trim()) void creador.nuevoTema(temaTitulo, temaTipo.slice(6) as 'leccion' | 'python' | 'r' | 'algebra');
    else return;
    temaTitulo = '';
    agregandoTema = false;
  }

  const fecha = (ms: number) => (ms ? new Date(ms).toLocaleString() : '');
</script>

<input bind:this={selector} type="file" multiple hidden onchange={alSubir} />

<div class="creador">
  {#if !b}
    <!-- inicio: crear, abrir o modificar -->
    <div class="inicio">
      <header>
        <Nativo toma="explicando" tam={84} titulo="Nativo" />
        <div>
          <span class="kicker">{t('Cursos de {0}', MARCA)}</span>
          <h1>{t('Crear un curso')}</h1>
          <p class="muted">
            {t('Un curso es un paquete de temas que cualquiera instala y usa como módulos: lecciones, una guía con historia y bibliografía, y calculadoras en Python o R o modelos de optimización. Lo armas con el asistente a partir de tus diapositivas, libros o programas, lo pruebas aquí mismo y lo exportas como .ncourse para tus estudiantes o colegas.')}
          </p>
        </div>
      </header>

      <section class="tarjeta">
        <h2>{t('Curso nuevo')}</h2>
        <form
          class="fila"
          onsubmit={(e) => {
            e.preventDefault();
            void creador.nuevo(titulo || t('Mi curso'), desde);
          }}
        >
          <input class="campo" bind:value={titulo} placeholder={t('Título, por ejemplo «Planeación de la producción»')} />
          <select class="campo corto" bind:value={desde}>
            <option value="vacio">{t('Vacío (lo arma el asistente)')}</option>
            <option value="plantilla">{t('Con temas de ejemplo')}</option>
          </select>
          <button class="btn primary" type="submit" disabled={!!creador.ocupado}><Icon name="mas" size={15} />{t('Crear')}</button>
        </form>
        <div class="fila otras">
          <button class="btn" onclick={() => creador.importar()}><Icon name="abrir" size={15} />{t('Abrir un .ncourse para modificarlo')}</button>
          {#if cursos.lista.length}
            <select
              class="campo corto"
              onchange={(e) => {
                const v = (e.currentTarget as HTMLSelectElement).value;
                if (v) void creador.nuevo('', `curso:${v}`);
                (e.currentTarget as HTMLSelectElement).value = '';
              }}
            >
              <option value="">{t('Modificar un curso instalado…')}</option>
              {#each cursos.lista as c (c.id)}<option value={c.id}>{c.titulo}</option>{/each}
            </select>
          {/if}
        </div>
      </section>

      {#if creador.borradores.length}
        <section class="tarjeta">
          <h2>{t('Tus borradores')}</h2>
          <ul class="borradores">
            {#each creador.borradores as x (x.id)}
              <li>
                <button class="abrir" onclick={() => creador.abrir(x.id)}>
                  <Icon name="libro" size={16} />
                  <span class="nm">{x.titulo || x.id}</span>
                  <span class="muted">{t('{0} temas', x.temas)} · {fecha(x.modificado)}{x.instalado ? ' · ' + t('vista previa instalada') : ''}</span>
                </button>
                <button class="btn ghost icon" title={t('Borrar el borrador')} onclick={() => creador.quitar(x.id)}><Icon name="cerrar" size={14} /></button>
              </li>
            {/each}
          </ul>
          <p class="muted chico">{t('Se guardan en {0}', creador.carpeta)}</p>
        </section>
      {/if}

      <section class="tarjeta">
        <h2>{t('Con Claude Code o Codex')}</h2>
        <p class="muted">
          {t('También puedes pedirle el curso a Claude Code o a Codex con la skill skills/crear-curso-nativo del repositorio, o con la herramienta courses del servidor MCP. El formato está en docs/CURSOS.md.')}
        </p>
      </section>
    </div>
  {:else}
    <!-- un borrador abierto -->
    <div class="barra">
      <div class="tit">
        <span class="kicker">{t('Borrador')}{b.instalado ? ' · ' + t('vista previa instalada') : ''}</span>
        <h1>{b.curso?.titulo || b.id}</h1>
      </div>
      <span class="estado" class:ok={b.validacion.ok} title={[...b.validacion.errores, ...b.validacion.avisos].join('\n')}>
        <Icon name={b.validacion.ok ? 'check' : 'alerta'} size={14} />
        {b.validacion.ok ? t('Válido') : t('{0} errores', b.validacion.errores.length)}{b.validacion.avisos.length ? ' · ' + t('{0} avisos', b.validacion.avisos.length) : ''}
      </span>
      <div class="acciones">
        <button class="btn" class:primary={!chat.abierto} onclick={() => chat.alternar()} title={t('Abre el asistente de creación')}><Icon name="chat" size={14} />{t('Asistente')}</button>
        <button class="btn" onclick={() => creador.probar()} disabled={!!creador.ocupado || !b.validacion.ok} title={t('Instala el curso y resuelve los ejemplos de cada tema')}><Icon name="jugar" size={14} />{t('Probar ejemplos')}</button>
        <button class="btn" onclick={() => creador.vistaPrevia()} disabled={!!creador.ocupado || !b.validacion.ok} title={t('Instala el curso y abre su primer tema, como lo verán los estudiantes')}><Icon name="pantalla" size={14} />{t('Vista previa')}</button>
        <button class="btn" onclick={() => creador.exportar()} disabled={!!creador.ocupado || !b.validacion.ok}><Icon name="exportar" size={14} />{t('Exportar .ncourse')}</button>
        <button class="btn ghost" onclick={() => creador.cerrar()} title={t('Cierra el borrador (queda guardado)')}><Icon name="cerrar" size={14} />{t('Cerrar')}</button>
      </div>
    </div>
    {#if creador.ocupado}<div class="ocupado"><span class="spinner" aria-hidden="true"></span>{creador.ocupado}</div>{/if}

    <div class="cuerpo">
      <!-- estructura -->
      <aside class="estructura">
        <h3>{t('Curso')}</h3>
        <ul class="archivos">
          {#each arbol.raiz as f (f.ruta)}
            <li><button class:sel={a?.ruta === f.ruta} onclick={() => creador.abrirArchivo(f.ruta)}><Icon name="documento" size={13} />{f.nombre}</button></li>
          {/each}
        </ul>

        <h3>{t('Temas')}</h3>
        {#each b.temas as tm (tm.id)}
          <details class="tema" open={a?.ruta.startsWith(`temas/${tm.id}/`)}>
            <summary>
              <span class="nm">{tm.titulo || tm.id}</span>
              <span class="tipo">{t(TIPOS[tm.tipo] ?? tm.tipo)}{tm.lenguaje ? ' · ' + t(LENGUAJES[tm.lenguaje] ?? tm.lenguaje) : ''}{tm.base ? ' · ' + tm.base : ''}{tm.origen?.curso_titulo ? ' · ' + t('de «{0}»', tm.origen.curso_titulo) : ''}</span>
            </summary>
            <ul class="archivos">
              {#each arbol.porTema.get(tm.id) ?? [] as f (f.ruta)}
                <li><button class:sel={a?.ruta === f.ruta} onclick={() => creador.abrirArchivo(f.ruta)}><Icon name="documento" size={13} />{f.nombre}</button></li>
              {/each}
            </ul>
            <div class="mini">
              <button class="btn ghost" onclick={() => subir(`temas/${tm.id}`)} title={t('Agrega imágenes, datos o PDF a este tema')}><Icon name="clip" size={13} />{t('Archivo')}</button>
              {#if b.instalado}<button class="btn ghost" onclick={() => creador.vistaPrevia(tm.id)}><Icon name="pantalla" size={13} />{t('Ver')}</button>{/if}
              <button class="btn ghost" onclick={() => creador.borrarArchivo(`temas/${tm.id}`)} title={t('Borrar el tema')}><Icon name="cerrar" size={13} /></button>
            </div>
          </details>
        {:else}
          <p class="muted chico">{t('Todavía no hay temas. Pídeselos al asistente o agrega uno.')}</p>
        {/each}

        {#if agregandoTema}
          <form
            class="nuevo-tema"
            onsubmit={(e) => {
              e.preventDefault();
              crearTema();
            }}
          >
            <!-- svelte-ignore a11y_autofocus -->
            <select class="campo" bind:value={temaTipo}>
              <optgroup label={t('Tema nuevo')}>
                <option value="nuevo:leccion">{t('Lección')}</option>
                <option value="nuevo:python">{t('Calculadora en Python')}</option>
                <option value="nuevo:r">{t('Calculadora en R')}</option>
                <option value="nuevo:algebra">{t('Modelo de optimización (algebraico)')}</option>
              </optgroup>
              <optgroup label={t('Módulos de Nativo (su motor y su guía)')}>
                {#each creador.biblioteca.modulos as m (m.desde)}<option value={m.desde}>{m.titulo}</option>{/each}
              </optgroup>
              {#each porCurso as [curso, temas] (curso)}
                <optgroup label={t('Copiar de «{0}»', curso)}>
                  {#each temas as x (x.desde)}<option value={x.desde}>{x.titulo}</option>{/each}
                </optgroup>
              {/each}
            </select>
            <!-- svelte-ignore a11y_autofocus -->
            <input class="campo" bind:value={temaTitulo} placeholder={desdeBiblioteca ? t('Título (opcional)') : t('Título del tema')} autofocus />
            {#if desdeBiblioteca}
              <p class="muted chico">
                {temaTipo.startsWith('modulo:')
                  ? t('Usa la pantalla, el motor y la guía del módulo. Le agregas lecciones, ejemplos y, si tu curso usa otra fórmula, un ajuste en Python o R.')
                  : t('Se copia al curso para que lo modifiques; queda el crédito a sus autores.')}
              </p>
            {/if}
            <div class="fila">
              <button class="btn primary" type="submit" disabled={!desdeBiblioteca && !temaTitulo.trim()}>{t('Agregar')}</button>
              <button class="btn ghost" type="button" onclick={() => (agregandoTema = false)}>{t('Cancelar')}</button>
            </div>
          </form>
        {:else}
          <button
            class="btn agregar"
            onclick={() => {
              agregandoTema = true;
              void creador.cargarBiblioteca();
            }}
          ><Icon name="mas" size={14} />{t('Agregar un tema')}</button>
        {/if}

        <h3>{t('Qué puede usar el asistente')}</h3>
        <label class="chk"><input type="checkbox" checked={creador.herramientas.leccion} onchange={(e) => creador.fijarHerramienta('leccion', e.currentTarget.checked)} />{t('Lecciones')}</label>
        <label class="chk"><input type="checkbox" checked={creador.herramientas.python} onchange={(e) => creador.fijarHerramienta('python', e.currentTarget.checked)} />{t('Calculadoras en Python')}</label>
        <label class="chk"><input type="checkbox" checked={creador.herramientas.r} onchange={(e) => creador.fijarHerramienta('r', e.currentTarget.checked)} />{t('Calculadoras en R')}</label>
        <label class="chk"><input type="checkbox" checked={creador.herramientas.algebra} onchange={(e) => creador.fijarHerramienta('algebra', e.currentTarget.checked)} />{t('Modelos de optimización')}</label>
      </aside>

      <!-- editor -->
      <main class="editor">
        {#if a}
          <div class="cab">
            <span class="ruta">{a.ruta}{creador.sinGuardar ? ' •' : ''}</span>
            {#if ext === 'md'}
              <button class="btn ghost" onclick={() => (vista = !vista)}><Icon name={vista ? 'teclado' : 'libro'} size={14} />{vista ? t('Editar') : t('Ver como lección')}</button>
            {/if}
            {#if !a.binario}
              <button class="btn" onclick={() => creador.guardarArchivo()} disabled={!creador.sinGuardar}><Icon name="guardar" size={14} />{t('Guardar')}</button>
            {/if}
            {#if a.ruta !== 'curso.json'}
              <button class="btn ghost icon" title={t('Borrar el archivo')} onclick={() => creador.borrarArchivo(a.ruta)}><Icon name="cerrar" size={14} /></button>
            {/if}
          </div>
          {#if imagen}
            <div class="img"><img src={`data:${mime};base64,${a.texto}`} alt={a.ruta} /></div>
          {:else if a.binario}
            <p class="muted centro">{t('Archivo binario: no se edita aquí.')}</p>
          {:else if vista && ext === 'md'}
            <article class="leccion"><Markdown texto={a.texto} /></article>
          {:else}
            <textarea class="codigo" spellcheck={ext === 'md'} bind:value={creador.archivo!.texto} onkeydown={teclas}></textarea>
          {/if}
        {:else}
          <p class="muted centro">{t('Elige un archivo a la izquierda.')}</p>
        {/if}
      </main>

      <!-- validación, pruebas y asistente -->
      <aside class="revision">
        <div class="asistente">
          <Nativo toma={chat.nativo} tam={56} titulo="Nativo" />
          <div>
            <strong>{t('Asistente de creación')}</strong>
            <p class="muted chico">
              {ajustes.iaLista
                ? t('Adjúntale en el chat tus diapositivas (.pptx), libros o apuntes (PDF), programas (.py, .R) o datos (.xlsx), y dile qué tema quieres.')
                : t('Conecta un modelo de IA en Ajustes para que el asistente arme el curso contigo.')}
            </p>
          </div>
        </div>
        <div class="sugerencias">
          {#each SUGERENCIAS as s (s)}<button onclick={() => pedir(s)}>{s}</button>{/each}
        </div>

        {#if b.validacion.errores.length}
          <h3>{t('Errores')}</h3>
          <ul class="lista mala">{#each b.validacion.errores as e (e)}<li>{e}</li>{/each}</ul>
        {/if}
        {#if b.validacion.avisos.length}
          <h3>{t('Avisos')}</h3>
          <ul class="lista">{#each b.validacion.avisos as e (e)}<li>{e}</li>{/each}</ul>
        {/if}
        {#if creador.pruebas}
          <h3>{t('Prueba de los ejemplos')}</h3>
          <ul class="lista pruebas">
            {#each creador.pruebas as p, i (i)}
              <li class:mala={!p.ok}>
                <Icon name={p.ok ? 'check' : 'alerta'} size={13} />
                <span><strong>{p.tema}{p.ejemplo ? ` · ${p.ejemplo}` : ''}</strong>: {p.ok ? p.headline : p.error}</span>
              </li>
            {:else}
              <li>{t('Ningún tema calcula: no hay ejemplos que probar.')}</li>
            {/each}
          </ul>
        {/if}
      </aside>
    </div>
  {/if}
</div>

<style>
  .creador {
    /* ocupa la barra superior y el área de trabajo de la columna principal */
    grid-row: 1 / -1;
    container: creador / inline-size;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .inicio {
    overflow: auto;
    padding: 24px max(24px, calc(50% - 420px)) 60px;
  }
  header {
    display: flex;
    gap: 18px;
    align-items: center;
    margin-bottom: 18px;
  }
  .kicker {
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--solver);
  }
  h1 {
    margin: 2px 0 4px;
    font-size: 22px;
  }
  h2 {
    font-size: 15px;
    margin: 0 0 10px;
  }
  h3 {
    font-size: 11.5px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--ink-2);
    margin: 16px 0 6px;
  }
  .tarjeta {
    border: 0.5px solid var(--rule);
    border-radius: 12px;
    background: var(--sheet);
    padding: 16px 18px;
    margin-bottom: 14px;
  }
  .fila {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
  }
  .otras {
    margin-top: 10px;
  }
  .campo {
    flex: 1;
    min-width: 200px;
    padding: 7px 10px;
    border: 0.5px solid var(--rule);
    border-radius: var(--r-sm);
    background: var(--field);
    color: var(--ink);
  }
  .campo.corto {
    flex: 0 1 auto;
    min-width: 0;
  }
  .chico {
    font-size: 12px;
  }
  .borradores {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .borradores li {
    display: flex;
    align-items: center;
    gap: 6px;
    border-bottom: 0.5px solid var(--rule-soft);
  }
  .abrir {
    flex: 1;
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 9px 4px;
    background: none;
    border: 0;
    text-align: left;
    cursor: pointer;
    color: var(--ink);
  }
  .abrir:hover .nm {
    color: var(--solver);
  }
  .abrir .nm {
    font-weight: 600;
  }
  .abrir .muted {
    margin-left: auto;
    font-size: 12px;
  }

  .barra {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 18px;
    border-bottom: 0.5px solid var(--rule);
    flex-wrap: wrap;
  }
  .barra h1 {
    font-size: 18px;
    margin: 0;
  }
  .estado {
    display: inline-flex;
    gap: 5px;
    align-items: center;
    font-size: 12.5px;
    padding: 3px 9px;
    border-radius: 999px;
    background: var(--bad-tint);
    color: var(--bad);
  }
  .estado.ok {
    background: var(--good-tint);
    color: var(--good);
  }
  .acciones {
    margin-left: auto;
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .ocupado {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 6px 18px;
    font-size: 12.5px;
    color: var(--ink-2);
    background: var(--solver-tint);
  }
  .cuerpo {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 250px minmax(0, 1fr) 300px;
  }
  /* angosto (por ejemplo, con el asistente abierto): la revisión va debajo del editor */
  @container creador (max-width: 1050px) {
    .cuerpo {
      grid-template-columns: 210px minmax(0, 1fr);
      grid-template-rows: minmax(0, 1fr) auto;
    }
    .estructura {
      grid-row: 1 / span 2;
    }
    .revision {
      grid-column: 2;
      max-height: 32vh;
      padding-bottom: 10px;
      border-left: 0;
      border-top: 0.5px solid var(--rule);
    }
    .revision .asistente,
    .revision .sugerencias {
      display: none;
    }
    .barra h1 {
      font-size: 16px;
    }
  }
  .estructura,
  .revision {
    overflow: auto;
    padding: 4px 12px 30px;
    border-right: 0.5px solid var(--rule);
    font-size: 13px;
  }
  .revision {
    border-right: 0;
    border-left: 0.5px solid var(--rule);
  }
  .archivos {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .archivos button {
    width: 100%;
    display: flex;
    gap: 6px;
    align-items: center;
    padding: 4px 6px;
    border: 0;
    background: none;
    border-radius: var(--r-xs);
    cursor: pointer;
    color: var(--ink);
    text-align: left;
    font-size: 12.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .archivos button:hover {
    background: var(--menu-hover);
  }
  .archivos button.sel {
    background: var(--solver-tint);
    color: var(--solver);
  }
  .tema {
    margin-bottom: 4px;
  }
  .tema summary {
    cursor: pointer;
    padding: 5px 4px;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .tema summary .nm {
    font-weight: 600;
  }
  .tipo {
    font-size: 11.5px;
    color: var(--ink-2);
  }
  .tema .archivos {
    padding-left: 10px;
  }
  .mini {
    display: flex;
    gap: 2px;
    padding-left: 6px;
  }
  .mini .btn {
    font-size: 12px;
    padding: 2px 6px;
  }
  .agregar {
    margin-top: 6px;
    width: 100%;
    justify-content: center;
  }
  .nuevo-tema {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 6px;
  }
  .nuevo-tema .campo {
    min-width: 0;
  }
  .chk {
    display: flex;
    gap: 6px;
    align-items: center;
    padding: 2px 0;
  }

  .editor {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .cab {
    display: flex;
    gap: 6px;
    align-items: center;
    padding: 8px 12px;
    border-bottom: 0.5px solid var(--rule-soft);
  }
  .ruta {
    flex: 1;
    font-family: ui-monospace, 'SF Mono', Consolas, monospace;
    font-size: 12.5px;
    color: var(--ink-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .codigo {
    flex: 1;
    resize: none;
    border: 0;
    outline: none;
    padding: 12px 16px;
    background: var(--sheet);
    color: var(--ink);
    font-family: ui-monospace, 'SF Mono', Consolas, monospace;
    font-size: 13px;
    line-height: 1.55;
    tab-size: 2;
  }
  .leccion {
    flex: 1;
    overflow: auto;
    padding: 6px 24px 30px;
    font-size: 14.5px;
    line-height: 1.6;
  }
  .img {
    flex: 1;
    overflow: auto;
    display: grid;
    place-items: center;
    padding: 16px;
  }
  .img img {
    max-width: 100%;
  }
  .centro {
    margin: auto;
    text-align: center;
  }

  .asistente {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-top: 12px;
  }
  .asistente p {
    margin: 2px 0 0;
  }
  .sugerencias {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 10px;
  }
  .sugerencias button {
    text-align: left;
    padding: 7px 10px;
    border: 0.5px solid var(--rule);
    border-radius: var(--r-md);
    background: var(--sheet);
    color: var(--ink);
    cursor: pointer;
    font-size: 12.5px;
    line-height: 1.4;
  }
  .sugerencias button:hover {
    border-color: var(--solver);
  }
  .lista {
    margin: 0;
    padding-left: 16px;
    font-size: 12.5px;
  }
  .lista li {
    margin-bottom: 4px;
  }
  .lista.mala,
  .lista li.mala {
    color: var(--bad);
  }
  .pruebas {
    list-style: none;
    padding: 0;
  }
  .pruebas li {
    display: flex;
    gap: 6px;
    align-items: flex-start;
  }
  .pruebas li :global(svg) {
    flex: none;
    margin-top: 2px;
    color: var(--good);
  }
  .pruebas li.mala :global(svg) {
    color: var(--bad);
  }
</style>
