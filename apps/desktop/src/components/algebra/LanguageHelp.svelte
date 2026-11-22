<script lang="ts">
  // Referencia breve del lenguaje algebraico: estructura, palabras clave en los
  // tres idiomas, conjuntos, parámetros, variables y restricciones generales.
  import Icon from '../Icon.svelte';
  import { t } from '../../lib/i18n.svelte';

  let { onclose }: { onclose: () => void } = $props();

  // concepto, español, inglés, portugués (los textos del lenguaje no se traducen)
  const claves: [string, string, string, string][] = $derived([
    [t('Conjunto'), 'conjunto', 'set', 'conjunto'],
    [t('Parámetro'), 'parametro', 'param', 'parametro'],
    [t('Variable'), 'variable', 'var', 'variavel'],
    [t('Maximizar'), 'maximizar', 'maximize, max', 'maximizar'],
    [t('Minimizar'), 'minimizar', 'minimize, min', 'minimizar'],
    [t('Restricciones'), 'sujeto a, s.a.', 'subject to, s.t.', 'sujeito a'],
    [t('Suma'), 'suma, ∑', 'sum', 'soma'],
    [t('Pertenencia'), 'en, ∈', 'in', 'em'],
    [t('Condición'), 'si … entonces … sino', 'if … then … else', 'se … entao … senao'],
    [t('Lógica'), 'y, o, no', 'and, or, not', 'e, ou, nao'],
    [t('Tipos'), 'binaria, entera, libre', 'binary, integer, free', 'binaria, inteira, livre'],
    [t('Valor por omisión'), 'default', 'default', 'padrao'],
    [t('Opción'), 'opcion', 'option', 'opcao']
  ]);

  const generales: [string, string, string][] = $derived([
    [t('Valor absoluto'), 'dev: abs(x - 10) <= 3;', t('También en el objetivo: minimizar e: abs(x - y);')],
    [t('Máximo y mínimo'), 'pico: max(x[1], x[2], x[3]) <= 50;', t('minimizar m: max(x, y); acota el mayor de los dos.')],
    [t('Lineal por tramos'), 'minimizar c: pwl(q; (0, 0), (100, 500), (300, 1100));', t('Puntos (x, f(x)) en orden; sirve para costos con descuentos.')],
    [t('Lógicas (binarias)'), 'c: z = and(a, b);', t('También or(a, b, …) y not(a).')],
    [t('Indicadoras'), 'c{i en I}: si y[i] = 0 entonces x[i] <= 0;', t('Otra forma: y[i] = 1 -> x[i] >= 50;')],
    [t('Disyunciones'), 'orden: s1 + 5 <= s2 o s2 + 3 <= s1;', t('Se cumple al menos una de las relaciones.')],
    [t('Cardinalidad'), 'c: atmost(2, y[A], y[B], y[C]);', t('También atleast(k, …) y exactly(k, …).')],
    [t('Implicación'), 'c: implies(a, b);', t('Si a = 1, entonces b = 1.')]
  ]);
</script>

<aside class="ayuda card" aria-label={t('Ayuda del lenguaje')}>
  <header>
    <h2><Icon name="libro" size={16} />{t('Ayuda del lenguaje')}</h2>
    <button class="btn ghost icon" onclick={onclose} title={t('Cerrar')} aria-label={t('Cerrar la ayuda')}><Icon name="cerrar" size={15} /></button>
  </header>

  <div class="cuerpo">
    <section>
      <h3>{t('Estructura')}</h3>
      <p>{t('Un modelo es una lista de instrucciones que terminan en «;»: conjuntos, parámetros, variables, uno o más objetivos y las restricciones. No importan las mayúsculas ni las tildes, y se pueden mezclar los tres idiomas.')}</p>
      <pre>conjunto I = &#123;A, B, C&#125;;
parametro c&#123;I&#125; = [5, 4, 3];
variable x&#123;I&#125; &gt;= 0;
maximizar z: suma&#123;i en I&#125; c[i] * x[i];
sujeto a
  cap: suma&#123;i en I&#125; x[i] &lt;= 10;</pre>
    </section>

    <section>
      <h3>{t('Palabras clave')}</h3>
      <div class="tw">
        <table>
          <thead><tr><th></th><th>Español</th><th>English</th><th>Português</th></tr></thead>
          <tbody>
            {#each claves as [c, es, en, pt] (c)}
              <tr><td class="c">{c}</td><td><code>{es}</code></td><td><code>{en}</code></td><td><code>{pt}</code></td></tr>
            {/each}
          </tbody>
        </table>
      </div>
      <p class="muted">{t('Estas palabras están reservadas: no sirven como nombres (p. ej. «minimo», «maximo», «suma» o «en»).')}</p>
    </section>

    <section>
      <h3>{t('Conjuntos y parámetros')}</h3>
      <ul class="ej">
        <li><code>conjunto T = 1..12;</code> <span>{t('rango de enteros')}</span></li>
        <li><code>conjunto Arcos = &#123;(A,B), (B,C)&#125;;</code> <span>{t('pares')}</span></li>
        <li><code>parametro d&#123;T&#125; = [100, 120, 90, …];</code> <span>{t('lista en el orden del conjunto')}</span></li>
        <li><code>parametro c&#123;I, J&#125; = [[4, 6], [5, 3]];</code> <span>{t('tabla: una fila por elemento de I')}</span></li>
        <li><code>parametro l&#123;Arcos&#125; = &#123;(A,B): 4, (B,C): 2&#125;;</code> <span>{t('por clave')}</span></li>
        <li><code>parametro cap&#123;t en T&#125; = si t &lt;= 6 entonces 120 sino 140;</code> <span>{t('calculado')}</span></li>
        <li><code>parametro p&#123;I&#125; default 1;</code> <span>{t('valor por omisión')}</span></li>
      </ul>
    </section>

    <section>
      <h3>{t('Variables')}</h3>
      <ul class="ej">
        <li><code>variable x&#123;I, J&#125; &gt;= 0;</code></li>
        <li><code>variable y&#123;I&#125; binaria;</code></li>
        <li><code>variable n &gt;= 0 &lt;= 10 entera;</code></li>
        <li><code>variable z libre;</code></li>
      </ul>
      <p class="muted">{t('Modo rápido: sin ninguna instrucción «variable», los nombres sueltos son variables continuas ≥ 0.')} <code>max 3x + 5y; s.t. c1: x &lt;= 4;</code></p>
    </section>

    <section>
      <h3>{t('Objetivo y restricciones')}</h3>
      <ul class="ej">
        <li><code>minimizar costo: suma&#123;i en I, j en J&#125; c[i,j] * x[i,j];</code></li>
        <li><code>oferta&#123;i en I&#125;: suma&#123;j en J&#125; x[i,j] &lt;= s[i];</code> <span>{t('una por cada i')}</span></li>
        <li><code>bal&#123;t en T: t &gt; 1&#125;: inv[t-1] + p[t] - inv[t] = d[t];</code> <span>{t('con condición')}</span></li>
        <li><code>minimizar f: (x - 2)^2 + (y - 1)^2;</code> <span>{t('objetivo cuadrático')}</span></li>
      </ul>
      <p class="muted">{t('Con varios objetivos se optimizan por prioridad, en el orden en que se escriben. Funciones de datos: card(I), floor, ceil, round, abs, sqrt, exp, log.')}</p>
    </section>

    <section>
      <h3>{t('Restricciones generales')}</h3>
      <dl class="gen">
        {#each generales as [n, ej, nota] (n)}
          <dt>{n}</dt>
          <dd><code>{ej}</code><span>{nota}</span></dd>
        {/each}
      </dl>
      <p class="muted">{t('Se reformulan solas con variables binarias y big-M tomado de las cotas: las variables que intervienen necesitan cotas finitas, o un big-M general con')} <code>opcion bigM = 1e4;</code></p>
    </section>

    <section>
      <h3>{t('Comentarios y atajos')}</h3>
      <ul class="ej">
        <li><code># …</code> <code>// …</code> <code>/* … */</code></li>
        <li><span class="kbd">Ctrl</span> <span class="kbd">Intro</span> {t('resolver')}</li>
        <li><span class="kbd">Ctrl</span> <span class="kbd">/</span> {t('comentar o descomentar las líneas')}</li>
        <li><span class="kbd">Tab</span> / <span class="kbd">Mayús</span> <span class="kbd">Tab</span> {t('sangría')}; <span class="kbd">Esc</span> {t('y luego')} <span class="kbd">Tab</span> {t('sale del editor')}</li>
      </ul>
    </section>
  </div>
</aside>

<style>
  .ayuda {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 10px 8px 16px;
    border-bottom: 0.5px solid var(--rule);
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 14.5px;
  }
  h2 :global(svg) {
    color: var(--solver);
  }
  .cuerpo {
    overflow: auto;
    padding: 6px 16px 18px;
    display: grid;
    gap: 16px;
    font-size: 12.5px;
    line-height: 1.5;
  }
  h3 {
    margin: 8px 0 6px;
    font-size: 12.5px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--ink-2);
  }
  p {
    margin: 0 0 6px;
  }
  code,
  pre {
    font-family: ui-monospace, 'SF Mono', 'Cascadia Mono', Menlo, Consolas, monospace;
    font-size: 11.5px;
  }
  code {
    padding: 0 4px;
    border-radius: 4px;
    background: var(--rule-soft);
    white-space: pre-wrap;
  }
  pre {
    margin: 0;
    padding: 8px 10px;
    border-radius: var(--r-sm);
    background: var(--sheet-2);
    border: 0.5px solid var(--rule);
    overflow: auto;
    line-height: 1.55;
  }
  .tw {
    overflow: auto;
    border: 0.5px solid var(--rule);
    border-radius: var(--r-sm);
    margin-bottom: 6px;
  }
  table {
    border-collapse: collapse;
    width: 100%;
  }
  th,
  td {
    text-align: left;
    padding: 4px 8px;
    border-bottom: 1px solid var(--rule-soft);
    vertical-align: top;
  }
  th {
    font-size: 11.5px;
    color: var(--ink-2);
    background: var(--sheet-2);
  }
  td.c {
    color: var(--ink-2);
    white-space: nowrap;
  }
  td code {
    background: none;
    padding: 0;
  }
  tr:last-child td {
    border-bottom: 0;
  }
  .ej {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 5px;
  }
  .ej span:not(.kbd) {
    color: var(--ink-3);
  }
  .gen {
    margin: 0 0 6px;
    display: grid;
    gap: 2px 0;
  }
  .gen dt {
    font-weight: 600;
    margin-top: 6px;
  }
  .gen dd {
    margin: 0;
    display: grid;
    gap: 2px;
  }
  .gen dd span {
    color: var(--ink-3);
  }
</style>
