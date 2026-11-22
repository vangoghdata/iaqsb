<script lang="ts">
  // Apariencia: Claro, Oscuro, Automático o Personalizado (temas listos y editor),
  // como la tarjeta de Ajustes de IA Nativa. Todo se ve al instante en la app.
  import Icon from '../Icon.svelte';
  import { ui } from '../../lib/ui.svelte';
  import { t } from '../../lib/i18n.svelte';
  import {
    armarTema,
    DIBUJOS,
    guardarImagenPropia,
    guardarPersonal,
    leerImagenPropia,
    leerPersonal,
    probarPersonal,
    TEMAS_LISTOS,
    VELO_POR_DEFECTO,
    type DisenoTema,
    type PreferenciaTema,
    type TemaListo
  } from '../../lib/tema';
  import type { NombreIcono } from '../../lib/iconos';

  const OPCIONES: { valor: PreferenciaTema; titulo: string; icono: NombreIcono; detalle: string }[] = [
    { valor: 'claro', titulo: 'Claro', icono: 'sol', detalle: 'Se lee mejor con mucha luz o en el proyector del salón.' },
    { valor: 'oscuro', titulo: 'Oscuro', icono: 'luna', detalle: 'Cansa menos la vista de noche.' },
    { valor: 'sistema', titulo: 'Automático', icono: 'pantalla', detalle: 'El mismo que tenga el computador.' },
    { valor: 'personalizado', titulo: 'Personalizado', icono: 'paleta', detalle: 'Tus colores y un fondo: temas listos o el tuyo.' }
  ];

  const inicial = leerPersonal();
  let diseno = $state<DisenoTema>(inicial?.diseno ?? TEMAS_LISTOS[0].diseno);
  let elegido = $state<string>(inicial?.id ?? 'azul');
  let tieneFoto = $state(Boolean(leerImagenPropia()));
  let aviso = $state<{ tipo: 'ok' | 'error'; texto: string } | null>(null);
  let subiendo = $state(false);
  let archivo: HTMLInputElement | undefined = $state();
  let cuadro = 0;
  let guardado = 0;

  function elegirModo(p: PreferenciaTema) {
    if (p === 'personalizado' && !leerPersonal()) {
      const azul = TEMAS_LISTOS.find((t) => t.id === 'azul') ?? TEMAS_LISTOS[0];
      diseno = azul.diseno;
      elegido = azul.id;
      guardarPersonal(armarTema(azul.diseno, azul.id));
      ui.tema = 'personalizado';
      return;
    }
    ui.elegirTema(p);
  }

  // Al arrastrar un color llegan muchos cambios: se pinta uno por cuadro y se guarda al soltar.
  function aplicar(d: DisenoTema, id: string) {
    diseno = d;
    elegido = id;
    const tema = armarTema(d, id);
    cancelAnimationFrame(cuadro);
    cuadro = requestAnimationFrame(() => probarPersonal(tema));
    clearTimeout(guardado);
    guardado = window.setTimeout(() => {
      guardarPersonal(tema);
      ui.tema = 'personalizado';
    }, 250);
  }
  const cambiar = (p: Partial<DisenoTema>) => aplicar({ ...$state.snapshot(diseno), ...p }, 'propio');
  const conDibujo = (dibujo: string): Partial<DisenoTema> => (diseno.dibujo ? { dibujo } : { dibujo, velo: VELO_POR_DEFECTO });

  async function subirFoto(f: File) {
    aviso = null;
    subiendo = true;
    try {
      const url = await comprimir(f);
      if (!guardarImagenPropia(url)) {
        aviso = { tipo: 'error', texto: t('La foto no cabe en este equipo. Prueba con otra más sencilla.') };
        return;
      }
      tieneFoto = true;
      cambiar({ dibujo: 'propia', velo: diseno.velo > 0 ? diseno.velo : VELO_POR_DEFECTO });
      aviso = { tipo: 'ok', texto: t('Listo. La foto quedó guardada en este equipo ({0} KB).', Math.round((url.length * 0.75) / 1024)) };
    } catch {
      aviso = { tipo: 'error', texto: t('No se pudo leer esa imagen.') };
    } finally {
      subiendo = false;
      if (archivo) archivo.value = '';
    }
  }

  function quitarFoto() {
    guardarImagenPropia(null);
    tieneFoto = false;
    if (diseno.dibujo === 'propia') cambiar({ dibujo: '', velo: 0 });
  }

  /** Achica la foto hasta que pese poco (JPEG de ~450 KB como máximo). */
  async function comprimir(f: File): Promise<string> {
    let fuente: CanvasImageSource & { width: number; height: number };
    let liberar = () => {};
    try {
      const b = await createImageBitmap(f, { resizeWidth: 1600, resizeQuality: 'medium' });
      fuente = b;
      liberar = () => b.close();
    } catch {
      const url = URL.createObjectURL(f);
      fuente = await new Promise<HTMLImageElement>((ok, mal) => {
        const i = new Image();
        i.onload = () => ok(i);
        i.onerror = mal;
        i.src = url;
      }).finally(() => URL.revokeObjectURL(url));
    }
    try {
      for (const [lado, calidad] of [
        [1600, 0.72],
        [1280, 0.62],
        [960, 0.55]
      ] as const) {
        const k = Math.min(1, lado / Math.max(fuente.width, fuente.height));
        const c = document.createElement('canvas');
        c.width = Math.max(1, Math.round(fuente.width * k));
        c.height = Math.max(1, Math.round(fuente.height * k));
        const ctx = c.getContext('2d')!;
        ctx.fillStyle = '#ffffff';
        ctx.fillRect(0, 0, c.width, c.height);
        ctx.drawImage(fuente, 0, 0, c.width, c.height);
        const datos = c.toDataURL('image/jpeg', calidad);
        if (datos.length < 600_000 || lado === 960) return datos;
      }
      throw new Error('imagen');
    } finally {
      liberar();
    }
  }

  const grupos: TemaListo['grupo'][] = ['Colores', 'Con fondo'];
  const COLORES: { k: 'menu' | 'barra' | 'acento' | 'fondo'; label: string }[] = [
    { k: 'menu', label: 'Menú' },
    { k: 'barra', label: 'Barra de arriba' },
    { k: 'acento', label: 'Botones' },
    { k: 'fondo', label: 'Fondo' }
  ];

  const claroM = { fondo: '#f4f5f7', barra: '#ffffff', tarjeta: '#ffffff', linea: '#d5d8de', acento: '#4f46e5' };
  const oscuroM = { fondo: '#030712', barra: '#111827', tarjeta: '#111827', linea: '#374151', acento: '#6366f1' };

  function cerrar() {
    ui.apariencia = false;
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && ui.apariencia && cerrar()} />

{#snippet mitad(c: typeof claroM, recorte?: string)}
  <g clip-path={recorte}>
    <rect width="120" height="68" fill={c.fondo} />
    <rect width="22" height="68" fill={c.barra} />
    <rect x="6" y="10" width="10" height="4" rx="2" fill={c.acento} />
    <rect x="6" y="20" width="10" height="3" rx="1.5" fill={c.linea} />
    <rect x="6" y="28" width="10" height="3" rx="1.5" fill={c.linea} />
    <rect x="30" y="10" width="82" height="22" rx="4" fill={c.tarjeta} stroke={c.linea} stroke-width="0.6" />
    <rect x="36" y="16" width="30" height="3" rx="1.5" fill={c.linea} />
    <rect x="36" y="23" width="46" height="3" rx="1.5" fill={c.linea} />
    <rect x="90" y="20" width="16" height="7" rx="3.5" fill={c.acento} />
    <rect x="30" y="38" width="82" height="22" rx="4" fill={c.tarjeta} stroke={c.linea} stroke-width="0.6" />
    <rect x="36" y="44" width="36" height="3" rx="1.5" fill={c.linea} />
    <rect x="36" y="51" width="24" height="3" rx="1.5" fill={c.linea} />
  </g>
{/snippet}

{#snippet muestra(tipo: PreferenciaTema)}
  <svg viewBox="0 0 120 68" class="muestra" aria-hidden="true">
    <defs>
      <clipPath id="tema-mitad-derecha"><polygon points="70,0 120,0 120,68 50,68" /></clipPath>
    </defs>
    {#if tipo === 'personalizado'}
      <rect width="120" height="68" fill="#fde2ef" />
      <rect width="22" height="68" fill="#be185d" />
      <rect x="22" width="98" height="8" fill="#fdf2f8" />
      <circle cx="98" cy="20" r="10" fill="#fff7fb" />
      <path d="M22 60 Q60 44 120 54 V68 H22Z" fill="#f5d0fe" />
      <rect x="30" y="24" width="56" height="30" rx="4" fill="#ffffff" />
      <rect x="36" y="30" width="30" height="3" rx="1.5" fill="#f9a8d4" />
      <rect x="36" y="37" width="40" height="3" rx="1.5" fill="#fbcfe8" />
      <rect x="64" y="45" width="16" height="6" rx="3" fill="#db2777" />
      <rect x="6" y="10" width="10" height="4" rx="2" fill="#fdf2f8" />
      <rect x="6" y="20" width="10" height="3" rx="1.5" fill="#f9a8d4" />
    {:else if tipo === 'oscuro'}
      {@render mitad(oscuroM)}
    {:else}
      {@render mitad(claroM)}
    {/if}
    {#if tipo === 'sistema'}{@render mitad(oscuroM, 'url(#tema-mitad-derecha)')}{/if}
  </svg>
{/snippet}

{#snippet miniatura(d: DisenoTema)}
  {@const dib = DIBUJOS.find((x) => x.id === d.dibujo)}
  <span class="mini" aria-hidden="true">
    <span class="mini-menu" style:background={d.menu}></span>
    <span class="mini-der">
      <span class="mini-barra" style:background={d.barra}></span>
      <span
        class="mini-fondo"
        style:background-color={d.fondo}
        style:background-image={dib ? `url("${dib.archivo}")` : undefined}
        style:background-size={dib?.mosaico ? `${Math.round(dib.mosaico / 3)}px` : 'cover'}
      >
        <span class="mini-tarjeta" style:background={d.base === 'claro' ? '#ffffff' : '#111827'}>
          <span class="mini-boton" style:background={d.acento}></span>
        </span>
      </span>
    </span>
  </span>
{/snippet}

{#if ui.apariencia}
  <div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && cerrar()}>
    <div class="dialog apariencia" role="dialog" aria-modal="true" aria-labelledby="apariencia-tit">
      <header class="cab">
        <div>
          <h2 id="apariencia-tit">{t('Apariencia')}</h2>
          <p class="muted">{t('Se guarda en este equipo: cada computador puede tener la suya.')}</p>
        </div>
        <button class="btn ghost icon" aria-label={t('Cerrar')} onclick={cerrar}><Icon name="cerrar" /></button>
      </header>

      <div class="modos" role="radiogroup" aria-label={t('Tema de la aplicación')}>
        {#each OPCIONES as o (o.valor)}
          {@const on = ui.tema === o.valor}
          <button role="radio" aria-checked={on} class="modo" class:on onclick={() => elegirModo(o.valor)}>
            {@render muestra(o.valor)}
            <span class="mtit">
              <Icon name={o.icono} size={16} />
              <span>{t(o.titulo)}</span>
              {#if on}<span class="activo">{t('Activo')}</span>{/if}
            </span>
            <span class="mdet">{t(o.detalle)}</span>
          </button>
        {/each}
      </div>

      {#if ui.tema === 'personalizado'}
        <div class="editor">
          {#each grupos as g (g)}
            <div>
              <p class="gtit">{t(g)}</p>
              <div class="listos">
                {#each TEMAS_LISTOS.filter((x) => x.grupo === g) as tl (tl.id)}
                  <button type="button" class="listo" class:on={elegido === tl.id} aria-pressed={elegido === tl.id} onclick={() => aplicar(tl.diseno, tl.id)}>
                    {@render miniatura(tl.diseno)}
                    <span class="lnom">{t(tl.nombre)}</span>
                  </button>
                {/each}
              </div>
            </div>
          {/each}

          <div class="ajuste">
            <p class="atit">{t('Ajústalo a tu gusto')}</p>
            <p class="adet">{t('Los textos se ponen claros u oscuros solos para que siempre se lean.')}</p>

            <div class="colores">
              {#each COLORES as c (c.k)}
                <label class="color">
                  <span class="bola" style:background={diseno[c.k]}>
                    <input type="color" value={diseno[c.k]} aria-label={t(c.label)} oninput={(e) => cambiar({ [c.k]: (e.currentTarget as HTMLInputElement).value })} />
                  </span>
                  <span>{t(c.label)}</span>
                </label>
              {/each}
            </div>

            <p class="stit">{t('Tarjetas')}</p>
            <div class="seg">
              {#each [['claro', 'Claras', 'sol'], ['oscuro', 'Oscuras', 'luna']] as [b, l, i] (b)}
                <button type="button" class:on={diseno.base === b} aria-pressed={diseno.base === b} onclick={() => cambiar({ base: b as 'claro' | 'oscuro' })}>
                  <Icon name={i} size={14} />
                  {t(l)}
                </button>
              {/each}
            </div>

            <p class="stit">{t('Dibujo de fondo')}</p>
            <div class="chips">
              <button type="button" class="chip" class:on={!diseno.dibujo} onclick={() => cambiar({ dibujo: '', velo: 0 })}>{t('Solo color')}</button>
              {#each DIBUJOS as x (x.id)}
                <button type="button" class="chip" class:on={diseno.dibujo === x.id} onclick={() => cambiar(conDibujo(x.id))}>{t(x.nombre)}</button>
              {/each}
              {#if tieneFoto}
                <button type="button" class="chip" class:on={diseno.dibujo === 'propia'} onclick={() => cambiar(conDibujo('propia'))}>{t('Mi foto')}</button>
              {/if}
            </div>

            <div class="foto">
              <input bind:this={archivo} type="file" accept="image/png,image/jpeg,image/webp" hidden onchange={(e) => {
                const f = (e.currentTarget as HTMLInputElement).files?.[0];
                if (f) subirFoto(f);
              }} />
              <button type="button" class="btn" disabled={subiendo} onclick={() => archivo?.click()}>
                <Icon name="imagen" size={16} />
                {subiendo ? t('Preparando…') : tieneFoto ? t('Cambiar mi foto') : t('Usar una foto mía')}
              </button>
              {#if tieneFoto}
                <button type="button" class="btn ghost" onclick={quitarFoto}>{t('Quitar foto')}</button>
              {/if}
            </div>
            <p class="adet">{t('Se achica y se guarda solo en este equipo.')}</p>

            {#if diseno.dibujo}
              <label class="velo">
                <span>{t('Suavizar el fondo')} <b>{Math.round(diseno.velo * 100)} %</b></span>
                <input type="range" min="0" max="0.9" step="0.05" value={diseno.velo} oninput={(e) => cambiar({ velo: Number((e.currentTarget as HTMLInputElement).value) })} />
              </label>
            {/if}

            {#if aviso}
              <p class="aviso" class:error={aviso.tipo === 'error'}>{aviso.texto}</p>
            {/if}
          </div>
        </div>
      {/if}

      <footer class="fin">
        <button class="btn primary" onclick={cerrar}>{t('Listo')}</button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .apariencia {
    width: min(820px, 100%);
    padding: 26px;
  }
  .cab {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
  }
  .cab h2 {
    font-size: 20px;
  }
  .cab p {
    margin: 4px 0 0;
    font-size: 13px;
  }
  .modos {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
    margin-top: 18px;
  }
  .modo {
    display: flex;
    flex-direction: column;
    text-align: left;
    padding: 10px;
    border-radius: 18px;
    border: 1px solid var(--rule);
    background: var(--rule-soft);
    transition: background-color 0.15s, border-color 0.15s;
  }
  .modo:hover {
    background: var(--rule);
  }
  .modo.on {
    border-color: var(--accent-btn);
    background: var(--solver-tint);
  }
  .muestra {
    display: block;
    width: 100%;
    border-radius: 11px;
    overflow: hidden;
  }
  .mtit {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-top: 10px;
    font-size: 13.5px;
    font-weight: 650;
  }
  .mtit :global(svg) {
    flex: none;
  }
  .activo {
    margin-left: auto;
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--accent-btn);
    color: var(--accent-btn-ink);
    font-size: 10.5px;
    font-weight: 650;
  }
  .mdet {
    margin-top: 4px;
    font-size: 12px;
    line-height: 1.45;
    color: var(--ink-2);
  }
  .editor {
    display: flex;
    flex-direction: column;
    gap: 20px;
    margin-top: 22px;
  }
  .gtit {
    margin: 0 0 8px;
    font-size: 10.5px;
    font-weight: 650;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--ink-3);
  }
  .listos {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(88px, 1fr));
    gap: 8px;
  }
  .listo {
    padding: 6px;
    border-radius: 16px;
    border: 1px solid var(--rule);
    background: none;
    text-align: left;
  }
  .listo:hover {
    background: var(--rule-soft);
  }
  .listo.on {
    border-color: var(--accent-btn);
    background: var(--solver-tint);
  }
  .lnom {
    display: block;
    margin: 6px 2px 0;
    font-size: 12px;
    font-weight: 550;
  }
  .mini {
    display: flex;
    height: 60px;
    border-radius: 10px;
    overflow: hidden;
    border: 1px solid rgba(0, 0, 0, 0.1);
  }
  .mini-menu {
    width: 20%;
  }
  .mini-der {
    flex: 1;
    display: flex;
    flex-direction: column;
  }
  .mini-barra {
    height: 11px;
  }
  .mini-fondo {
    position: relative;
    flex: 1;
    background-position: center;
  }
  .mini-tarjeta {
    position: absolute;
    inset: 8px 8px 7px;
    border-radius: 5px;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.15);
  }
  .mini-boton {
    position: absolute;
    right: 5px;
    bottom: 5px;
    width: 18px;
    height: 7px;
    border-radius: 99px;
  }
  .ajuste {
    padding: 18px;
    border-radius: 18px;
    border: 1px solid var(--rule);
    background: var(--rule-soft);
  }
  .atit {
    margin: 0;
    font-weight: 650;
  }
  .adet {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--ink-3);
  }
  .colores {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
    margin-top: 14px;
  }
  .color {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 8px;
    border-radius: 12px;
    border: 1px solid var(--rule);
    background: var(--sheet);
    cursor: pointer;
    font-size: 12.5px;
    font-weight: 550;
  }
  .bola {
    position: relative;
    flex: none;
    width: 30px;
    height: 30px;
    border-radius: 9px;
    border: 1px solid var(--rule);
    overflow: hidden;
  }
  .bola input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: pointer;
  }
  .stit {
    margin: 18px 0 8px;
    font-size: 12px;
    font-weight: 600;
    color: var(--ink-2);
  }
  .seg button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    height: 30px;
    padding: 0 13px;
    border-radius: 999px;
    border: 1px solid var(--rule);
    background: var(--rule-soft);
    font-size: 12.5px;
    font-weight: 550;
    color: var(--ink-2);
  }
  .chip:hover {
    color: var(--ink);
  }
  .chip.on {
    border-color: transparent;
    background: var(--accent-btn);
    color: var(--accent-btn-ink);
  }
  .foto {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 14px;
  }
  .velo {
    display: block;
    margin-top: 16px;
    font-size: 12px;
    font-weight: 600;
    color: var(--ink-2);
  }
  .velo span {
    display: flex;
    justify-content: space-between;
  }
  .velo b {
    font-variant-numeric: tabular-nums;
    color: var(--ink-3);
  }
  .velo input {
    width: 100%;
    margin-top: 8px;
  }
  .aviso {
    margin: 12px 0 0;
    font-size: 12.5px;
    color: var(--good);
  }
  .aviso.error {
    color: var(--bad);
  }
  .fin {
    display: flex;
    justify-content: flex-end;
    margin-top: 22px;
  }
  @media (max-width: 720px) {
    .modos,
    .colores {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
