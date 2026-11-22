// Markdown pequeño y seguro para el chat con Claude.
//
// Se escapa TODO el texto primero (& < > " ') y solo después se agregan las
// etiquetas que este renderizador conoce; por eso el HTML que sale no puede traer
// etiquetas ni atributos escritos por el modelo. Los enlaces solo aceptan http(s).
//
// Soporta: títulos, párrafos, listas (con anidación), citas, bloques de código,
// tablas de GFM (con alineación), líneas horizontales, **negrita**, *cursiva*,
// ~~tachado~~, `código` y enlaces. Un bloque de código sin cerrar (mientras llega
// la respuesta) se muestra igual.

export interface OpcionesMd {
  /** Texto del botón de copiar (ya traducido). */
  copiar?: string;
}

const ESC: Record<string, string> = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' };

export function escapar(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ESC[c]);
}

// marcadores internos (el texto de entrada no puede traerlos: se quitan antes)
const M0 = '\u0000';
const M1 = '\u0001';

/** Texto en línea, YA ESCAPADO, a HTML. */
function enLinea(s: string): string {
  const guardados: string[] = [];
  const guardar = (html: string) => `${M0}${guardados.push(html) - 1}${M1}`;

  // código en línea: su contenido no se interpreta
  s = s.replace(/(`+)([^`]|[^`][\s\S]*?[^`])\1(?!`)/g, (_, _b, c: string) => guardar(`<code>${c.trim() === '' ? c : c.replace(/^ (.*) $/, '$1')}</code>`));
  // enlaces [texto](https://…)
  s = s.replace(/\[([^\]\n]+)\]\((https?:\/\/[^\s)]+)\)/g, (_, txt: string, url: string) => guardar(`<a href="${url}" title="${url}">${txt}</a>`));
  // direcciones sueltas (DOI, páginas): https://…
  s = s.replace(/(^|[\s(])(https?:\/\/(?:(?!&quot;|&#39;|&lt;|&gt;)[^\s<])+(?<![.,;:!?)\]]))/g, (_, pre: string, url: string) => pre + guardar(`<a href="${url}">${url}</a>`));
  // negrita, cursiva y tachado
  s = s.replace(/\*\*(?=\S)([\s\S]*?\S)\*\*/g, '<strong>$1</strong>');
  s = s.replace(/(^|[^\p{L}\p{N}_])__(?=\S)([\s\S]*?\S)__(?![\p{L}\p{N}_])/gu, '$1<strong>$2</strong>');
  s = s.replace(/(^|[^*\p{L}\p{N}])\*(?=[^\s*])([^*\n]*?[^\s*])\*(?!\*)/gu, '$1<em>$2</em>');
  s = s.replace(/(^|[^\p{L}\p{N}_])_(?=[^\s_])([^_\n]*?[^\s_])_(?![\p{L}\p{N}_])/gu, '$1<em>$2</em>');
  s = s.replace(/~~(?=\S)([\s\S]*?\S)~~/g, '<del>$1</del>');
  // vuelve a poner lo guardado
  return s.replace(new RegExp(`${M0}(\\d+)${M1}`, 'g'), (_, k: string) => guardados[Number(k)]);
}

const RE_FENCE = /^ {0,3}(`{3,}|~{3,})\s*([\w+#.-]*)?.*$/;
const RE_TITULO = /^ {0,3}(#{1,6})\s+(.*?)\s*#*\s*$/;
const RE_HR = /^ {0,3}([-*_])(\s*\1){2,}\s*$/;
const RE_ITEM = /^(\s*)([-*+]|\d{1,9}[.)])\s+(.*)$/;
const RE_SEP_TABLA = /^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?\s*$/;
const RE_CITA = /^ {0,3}>\s?(.*)$/;

function celdas(fila: string): string[] {
  let f = fila.trim().replace(/\\\|/g, M0);
  if (f.startsWith('|')) f = f.slice(1);
  if (f.endsWith('|')) f = f.slice(0, -1);
  return f.split('|').map((c) => c.replace(new RegExp(M0, 'g'), '|').trim());
}

function esInicioDeBloque(l: string, sig?: string): boolean {
  return (
    RE_FENCE.test(l) ||
    RE_TITULO.test(l) ||
    RE_HR.test(l) ||
    RE_ITEM.test(l) ||
    RE_CITA.test(l) ||
    (l.includes('|') && sig !== undefined && RE_SEP_TABLA.test(sig) && sig.includes('-'))
  );
}

function bloques(lineas: string[], op: OpcionesMd): string {
  const out: string[] = [];
  let i = 0;
  const copiar = escapar(op.copiar ?? 'Copiar');
  while (i < lineas.length) {
    const l = lineas[i];
    if (!l.trim()) {
      i++;
      continue;
    }
    // bloque de código
    const fence = RE_FENCE.exec(l);
    if (fence) {
      const marca = fence[1];
      const lang = fence[2] ?? '';
      const cuerpo: string[] = [];
      i++;
      const cierre = new RegExp(`^ {0,3}\\${marca[0]}{${marca.length},}\\s*$`);
      while (i < lineas.length && !cierre.test(lineas[i])) {
        cuerpo.push(lineas[i]);
        i++;
      }
      i++; // la marca de cierre
      out.push(
        `<div class="md-code"><div class="md-code-h"><span>${escapar(lang)}</span><button type="button" class="md-copy" data-copy="code">${copiar}</button></div><pre><code>${escapar(cuerpo.join('\n'))}</code></pre></div>`
      );
      continue;
    }
    // título (# … ######): en el chat bajan dos niveles para no gritar
    const tit = RE_TITULO.exec(l);
    if (tit) {
      const n = Math.min(6, tit[1].length + 2);
      out.push(`<h${n}>${enLinea(escapar(tit[2]))}</h${n}>`);
      i++;
      continue;
    }
    if (RE_HR.test(l)) {
      out.push('<hr>');
      i++;
      continue;
    }
    // tabla de GFM
    if (l.includes('|') && i + 1 < lineas.length && RE_SEP_TABLA.test(lineas[i + 1]) && lineas[i + 1].includes('-')) {
      const cab = celdas(l);
      const alin = celdas(lineas[i + 1]).map((c) => (c.startsWith(':') && c.endsWith(':') ? 'center' : c.endsWith(':') ? 'right' : c.startsWith(':') ? 'left' : ''));
      const st = (k: number) => (alin[k] ? ` style="text-align:${alin[k]}"` : '');
      i += 2;
      const filas: string[][] = [];
      while (i < lineas.length && lineas[i].includes('|') && lineas[i].trim()) {
        filas.push(celdas(lineas[i]));
        i++;
      }
      const th = cab.map((c, k) => `<th${st(k)}>${enLinea(escapar(c))}</th>`).join('');
      const tb = filas.map((f) => '<tr>' + cab.map((_, k) => `<td${st(k)}>${enLinea(escapar(f[k] ?? ''))}</td>`).join('') + '</tr>').join('');
      out.push(
        `<div class="md-table"><button type="button" class="md-copy" data-copy="table">${copiar}</button><div class="md-tw"><table><thead><tr>${th}</tr></thead><tbody>${tb}</tbody></table></div></div>`
      );
      continue;
    }
    // cita
    if (RE_CITA.test(l)) {
      const dentro: string[] = [];
      while (i < lineas.length && lineas[i].trim() && (RE_CITA.test(lineas[i]) || !esInicioDeBloque(lineas[i]))) {
        const m = RE_CITA.exec(lineas[i]);
        dentro.push(m ? m[1] : lineas[i]);
        i++;
      }
      out.push(`<blockquote>${bloques(dentro, op)}</blockquote>`);
      continue;
    }
    // lista (con sublistas por sangría)
    if (RE_ITEM.test(l)) {
      const r = lista(lineas, i, op);
      out.push(r.html);
      i = r.fin;
      continue;
    }
    // párrafo: líneas seguidas hasta una vacía o el inicio de otro bloque
    const par: string[] = [];
    while (i < lineas.length && lineas[i].trim() && (par.length === 0 || !esInicioDeBloque(lineas[i], lineas[i + 1]))) {
      par.push(lineas[i].trim());
      i++;
    }
    out.push(`<p>${par.map((x) => enLinea(escapar(x))).join('<br>')}</p>`);
  }
  return out.join('');
}

/** Lista desde la línea `i`; termina donde la sangría baja o aparece otro bloque. */
function lista(lineas: string[], i: number, op: OpcionesMd): { html: string; fin: number } {
  const m0 = RE_ITEM.exec(lineas[i])!;
  const sangria = m0[1].length;
  const ordenada = /\d/.test(m0[2]);
  const inicio = ordenada ? parseInt(m0[2], 10) : 1;
  const items: string[] = [];
  while (i < lineas.length) {
    const m = RE_ITEM.exec(lineas[i]);
    if (!m || m[1].length < sangria || /\d/.test(m[2]) !== ordenada) break;
    if (m[1].length > sangria) break; // no debería pasar: lo toma la sublista
    const texto: string[] = [m[3]];
    let sub = '';
    i++;
    // continuación del ítem y sublistas
    while (i < lineas.length) {
      const l = lineas[i];
      if (!l.trim()) {
        // una línea vacía sigue la lista solo si después viene otro ítem o algo con sangría
        const sig = lineas[i + 1];
        if (sig !== undefined && (RE_ITEM.test(sig) || /^\s{2,}\S/.test(sig))) {
          i++;
          continue;
        }
        break;
      }
      const mi = RE_ITEM.exec(l);
      if (mi && mi[1].length > sangria) {
        const r = lista(lineas, i, op);
        sub += r.html;
        i = r.fin;
        continue;
      }
      if (mi) break;
      const sangriaL = l.length - l.trimStart().length;
      if (sangriaL > sangria && !RE_FENCE.test(l)) {
        texto.push(l.trim());
        i++;
        continue;
      }
      if (sangriaL > sangria && RE_FENCE.test(l)) {
        // bloque de código dentro del ítem
        const cuerpo: string[] = [l.trim()];
        i++;
        while (i < lineas.length && !RE_FENCE.test(lineas[i])) cuerpo.push(lineas[i++].trim());
        if (i < lineas.length) cuerpo.push(lineas[i++].trim());
        sub += bloques(cuerpo, op);
        continue;
      }
      if (esInicioDeBloque(l, lineas[i + 1])) break;
      texto.push(l.trim());
      i++;
    }
    items.push(`<li>${texto.map((x) => enLinea(escapar(x))).join('<br>')}${sub}</li>`);
  }
  const tag = ordenada ? 'ol' : 'ul';
  const st = ordenada && inicio !== 1 ? ` start="${inicio}"` : '';
  return { html: `<${tag}${st}>${items.join('')}</${tag}>`, fin: i };
}

/** Markdown → HTML seguro (todo lo que viene del texto queda escapado). */
export function markdownHtml(src: string, op: OpcionesMd = {}): string {
  const limpio = src.replace(/\r\n?/g, '\n').replace(/[\u0000\u0001]/g, '');
  return bloques(limpio.split('\n'), op);
}
