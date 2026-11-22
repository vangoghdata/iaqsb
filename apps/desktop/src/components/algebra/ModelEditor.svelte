<script lang="ts">
  // Editor de código del lenguaje algebraico, sin dependencias: un <textarea>
  // transparente sobre un <pre> resaltado con la misma tipografía (el scroll
  // se sincroniza), con números de línea, sangría automática, Tab = 2 espacios,
  // Ctrl+/ comenta y Ctrl+Intro resuelve. El error de la revisión se subraya
  // en su lugar y se marca su línea.
  import { onDestroy, onMount } from 'svelte';
  import { offsetDe, resaltar } from '../../lib/algebra-resaltado';
  import { t } from '../../lib/i18n.svelte';

  let {
    value,
    oninput,
    error = null,
    onsubmit,
    jump = null,
    onjump,
    memoria,
    label
  }: {
    value: string;
    oninput: (v: string) => void;
    error?: { line: number; col: number; message: string } | null;
    onsubmit?: () => void;
    jump?: { line: number; col: number; n: number } | null;
    /** Se llama al atender un salto (para no repetirlo al volver a montar el editor). */
    onjump?: () => void;
    /** Desplazamiento y selección que se conservan al cambiar de pestaña. */
    memoria?: { top: number; left: number; a: number; b: number };
    label?: string;
  } = $props();

  const LH = 20; // alto de línea (px): el mismo en el textarea, el <pre> y el margen
  const PAD = 10; // relleno superior

  let ta: HTMLTextAreaElement | undefined = $state();
  let arriba = $state(0);
  let izq = $state(0);
  // tras Esc, el siguiente Tab sale del editor (para quien usa solo el teclado)
  let salir = false;

  const lineas = $derived(value.split('\n').length);
  const errOff = $derived(error && error.line > 0 ? offsetDe(value, error.line, error.col) : null);
  const html = $derived(resaltar(value, errOff));
  const errLinea = $derived(error && error.line > 0 ? Math.min(error.line, lineas) : 0);
  const ancho = $derived(String(lineas).length);

  function scroll() {
    if (!ta) return;
    arriba = ta.scrollTop;
    izq = ta.scrollLeft;
  }

  // inserta texto como si se tecleara (así Ctrl+Z lo deshace)
  function insertar(txt: string) {
    if (!ta) return;
    ta.focus();
    if (!document.execCommand('insertText', false, txt)) {
      ta.setRangeText(txt, ta.selectionStart, ta.selectionEnd, 'end');
      oninput(ta.value);
    }
  }

  /** Inicio y fin de las líneas que toca la selección. */
  function lineasSel(): [number, number] {
    const v = ta!.value;
    const a = ta!.selectionStart;
    let b = ta!.selectionEnd;
    // una selección que termina justo al empezar una línea no la incluye
    if (b > a && v[b - 1] === '\n') b--;
    const ini = v.lastIndexOf('\n', a - 1) + 1;
    let fin = v.indexOf('\n', b);
    if (fin < 0) fin = v.length;
    return [ini, fin];
  }

  /** Reemplaza las líneas seleccionadas por `f(líneas)` y las deja seleccionadas. */
  function cambiarLineas(f: (ls: string[]) => string[]) {
    if (!ta) return;
    const [ini, fin] = lineasSel();
    const nuevo = f(ta.value.slice(ini, fin).split('\n')).join('\n');
    ta.setSelectionRange(ini, fin);
    insertar(nuevo);
    ta.setSelectionRange(ini, ini + nuevo.length);
  }

  function comentar() {
    cambiarLineas((ls) => {
      const llenas = ls.filter((l) => l.trim());
      const todas = llenas.length > 0 && llenas.every((l) => /^\s*#/.test(l));
      if (todas) return ls.map((l) => l.replace(/^(\s*)# ?/, '$1'));
      const sangria = Math.min(...llenas.map((l) => l.length - l.trimStart().length));
      return ls.map((l) => (l.trim() ? l.slice(0, sangria) + '# ' + l.slice(sangria) : l));
    });
  }

  function keydown(e: KeyboardEvent) {
    if (!ta) return;
    const mod = e.ctrlKey || e.metaKey;
    if (e.key === 'Escape') {
      salir = true;
      return;
    }
    if (e.key === 'Tab' && !mod && !e.altKey) {
      if (salir) {
        salir = false;
        return;
      }
      e.preventDefault();
      const multi = ta.value.slice(ta.selectionStart, ta.selectionEnd).includes('\n');
      if (e.shiftKey) cambiarLineas((ls) => ls.map((l) => l.replace(/^ {1,2}|^\t/, '')));
      else if (multi) cambiarLineas((ls) => ls.map((l) => (l ? '  ' + l : l)));
      else insertar('  ');
      return;
    }
    salir = false;
    if (mod && e.key === 'Enter') {
      e.preventDefault();
      // el atajo de la ventana no debe resolver dos veces
      e.stopPropagation();
      onsubmit?.();
      return;
    }
    if (mod && (e.key === '/' || e.code === 'Slash')) {
      e.preventDefault();
      comentar();
      return;
    }
    if (e.key === 'Enter' && !mod && !e.shiftKey && !e.altKey && !e.isComposing) {
      e.preventDefault();
      const v = ta.value;
      const a = ta.selectionStart;
      const ini = v.lastIndexOf('\n', a - 1) + 1;
      const antes = v.slice(ini, a);
      let sangria = /^[ \t]*/.exec(antes)![0];
      // sin el comentario del final, ¿la línea queda abierta?
      const codigo = antes.replace(/(#|\/\/).*$/, '').trimEnd();
      const st = /^\s*(sujeto\s+a|sujeita\s+a|sujeito\s+a|subject\s+to|s\.[ta]\.)\s*$/i.test(codigo);
      if (st || /[{([:=]$/.test(codigo) || /(entonces|then|entao|então|->)$/i.test(codigo)) sangria += '  ';
      insertar('\n' + sangria);
      return;
    }
  }

  // al volver a la pestaña, el texto queda donde estaba
  onMount(() => {
    if (!ta || !memoria || jump) return;
    const area = ta;
    const m = memoria;
    queueMicrotask(() => {
      area.setSelectionRange(m.a, m.b);
      area.scrollTop = m.top;
      area.scrollLeft = m.left;
      scroll();
    });
  });
  onDestroy(() => {
    if (ta && memoria) Object.assign(memoria, { top: ta.scrollTop, left: ta.scrollLeft, a: ta.selectionStart, b: ta.selectionEnd });
  });

  // salto pedido desde fuera (clic en un error, carga de un archivo)
  $effect(() => {
    const j = jump;
    if (!j || !ta) return;
    onjump?.();
    const area = ta;
    queueMicrotask(() => {
      const off = offsetDe(area.value, j.line, j.col);
      const m = /^[\p{L}\p{N}_'.]+/u.exec(area.value.slice(off));
      area.focus({ preventScroll: true });
      area.setSelectionRange(off, off + (j.line > 1 || j.col > 1 ? (m?.[0].length ?? 0) : 0));
      const y = (j.line - 1) * LH;
      if (y < area.scrollTop || y > area.scrollTop + area.clientHeight - LH * 2) area.scrollTop = Math.max(0, y - area.clientHeight / 3);
      if (j.line === 1 && j.col === 1) area.scrollTop = 0;
      area.scrollLeft = 0;
      scroll();
    });
  });
</script>

<div class="ed" style:--lh={`${LH}px`} style:--pad={`${PAD}px`} style:--gw={`${ancho + 2.4}ch`}>
  <div class="margen" aria-hidden="true">
    <div class="num" style:transform={`translateY(${-arriba}px)`}>
      {#each { length: lineas } as _, i (i)}
        <div class:bad={i + 1 === errLinea}>{i + 1}</div>
      {/each}
    </div>
  </div>
  <div class="area">
    <div class="capa" style:transform={`translate(${-izq}px, ${-arriba}px)`} aria-hidden="true">
      {#if errLinea}<div class="banda" style:top={`${PAD + (errLinea - 1) * LH}px`}></div>{/if}
      <!-- el HTML sale de resaltar(), que escapa el texto -->
      <pre class="hl">{@html html}</pre>
    </div>
    <textarea
      bind:this={ta}
      {value}
      oninput={(e) => oninput((e.currentTarget as HTMLTextAreaElement).value)}
      onscroll={scroll}
      onkeydown={keydown}
      wrap="off"
      spellcheck="false"
      autocomplete="off"
      autocapitalize="off"
      aria-label={label ?? t('Texto del modelo algebraico')}
      aria-invalid={!!error}
    ></textarea>
  </div>
</div>

<style>
  .ed {
    /* colores del resaltado (oscuro); el tema claro los cambia abajo */
    --c-kw: #ff7ab2;
    --c-fn: #6bd3c4;
    --c-num: #d9c97c;
    --c-str: #ff8170;
    --c-com: #7f8c98;
    --c-set: #c49cff;
    --c-par: #6cc7ff;
    --c-var: #ffa65c;
    --c-name: #9edb8f;
    --c-op: var(--ink-2);
    --mono: ui-monospace, 'SF Mono', 'Cascadia Mono', 'JetBrains Mono', Menlo, Consolas, 'DejaVu Sans Mono', 'Liberation Mono', monospace;
    position: relative;
    display: grid;
    grid-template-columns: var(--gw) minmax(0, 1fr);
    height: 100%;
    min-height: 0;
    background: var(--sheet);
    border: 0.5px solid var(--rule);
    border-radius: var(--r-lg);
    overflow: hidden;
    font-family: var(--mono);
    font-size: 13px;
    line-height: var(--lh);
  }
  :global(html[data-tema='claro']) .ed {
    --c-kw: #ad3da4;
    --c-fn: #3e8087;
    --c-num: #1c00cf;
    --c-str: #c41a16;
    --c-com: #6b7885;
    --c-set: #6c36a9;
    --c-par: #0b5f8f;
    --c-var: #b45309;
    --c-name: #23743a;
  }
  .ed:focus-within {
    border-color: color-mix(in oklab, var(--focus) 55%, transparent);
    box-shadow: 0 0 0 3px color-mix(in oklab, var(--focus) 18%, transparent);
  }
  .margen {
    position: relative;
    overflow: hidden;
    background: var(--sheet-2);
    border-right: 0.5px solid var(--rule);
    color: var(--ink-3);
    font-size: 11.5px;
    text-align: right;
    user-select: none;
  }
  .num {
    padding: var(--pad) 0.8ch var(--pad) 0;
    will-change: transform;
  }
  .num div {
    height: var(--lh);
    font-variant-numeric: tabular-nums;
  }
  .num div.bad {
    color: var(--bad);
    font-weight: 700;
    box-shadow: inset 2px 0 0 var(--bad);
  }
  .area {
    position: relative;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }
  .capa {
    position: absolute;
    top: 0;
    left: 0;
    min-width: 100%;
    pointer-events: none;
    will-change: transform;
  }
  .banda {
    position: absolute;
    left: 0;
    right: 0;
    height: var(--lh);
    background: var(--bad-tint);
  }
  pre.hl,
  textarea {
    margin: 0;
    border: 0;
    padding: var(--pad) 16px var(--pad) 12px;
    font: inherit;
    line-height: var(--lh);
    letter-spacing: 0;
    tab-size: 2;
    font-variant-ligatures: none;
    font-feature-settings: 'liga' 0, 'calt' 0;
    white-space: pre;
    word-wrap: normal;
    overflow-wrap: normal;
  }
  pre.hl {
    position: relative;
    color: var(--ink);
    background: none;
  }
  textarea {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    resize: none;
    overflow: auto;
    background: transparent;
    color: transparent;
    caret-color: var(--ink);
    outline: none;
    border-radius: 0;
  }
  textarea::selection {
    background: color-mix(in oklab, var(--solver) 32%, transparent);
    color: transparent;
  }
  .hl :global(.c-kw) {
    color: var(--c-kw);
  }
  .hl :global(.c-fn) {
    color: var(--c-fn);
  }
  .hl :global(.c-num) {
    color: var(--c-num);
  }
  .hl :global(.c-str) {
    color: var(--c-str);
  }
  .hl :global(.c-com) {
    color: var(--c-com);
    font-style: italic;
  }
  .hl :global(.c-set) {
    color: var(--c-set);
  }
  .hl :global(.c-par) {
    color: var(--c-par);
  }
  .hl :global(.c-var) {
    color: var(--c-var);
  }
  .hl :global(.c-name) {
    color: var(--c-name);
  }
  .hl :global(.c-op),
  .hl :global(.c-pun) {
    color: var(--c-op);
  }
  .hl :global(.c-err) {
    text-decoration: underline wavy var(--bad);
    text-decoration-skip-ink: none;
    text-underline-offset: 3px;
  }
  .hl :global(.c-fin) {
    display: inline-block;
    width: 1ch;
    border-bottom: 2px solid var(--bad);
    text-decoration: none;
  }
</style>
