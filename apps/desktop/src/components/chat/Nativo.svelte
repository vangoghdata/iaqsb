<script lang="ts">
  // Nativo animado (webp con transparencia). Cambiar de toma cambia la imagen; con «reducir
  // movimiento» o con las animaciones apagadas en Ajustes queda la imagen quieta.
  import { ajustes } from '../../lib/ajustes.svelte';
  import { prefiereQuieto, srcNativo, type TomaNativo } from '../../lib/nativo';

  let { toma = 'reposo', tam = 40, circulo = false, titulo = '', fijo = false }: { toma?: TomaNativo; tam?: number; circulo?: boolean; titulo?: string; fijo?: boolean } = $props();
  const quieto = $derived(fijo || !ajustes.nativo || prefiereQuieto());
  // si una animación no carga (instalación incompleta), se queda con la quieta
  let fallo = $state(false);
</script>

<span class="nativo" class:circulo style:--tam="{tam}px" title={titulo || undefined}>
  {#key toma + String(quieto || fallo)}
    <img src={srcNativo(toma, quieto || fallo)} alt={titulo} width={tam} height={tam} draggable="false" onerror={() => (fallo = true)} />
  {/key}
</span>

<style>
  .nativo {
    display: inline-block;
    width: var(--tam);
    height: var(--tam);
    flex: none;
    line-height: 0;
  }
  .nativo img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    user-select: none;
    pointer-events: none;
  }
  .circulo {
    border-radius: 50%;
    overflow: hidden;
    background: radial-gradient(circle at 50% 35%, #ffffff, #e2e7ff 75%);
    box-shadow: 0 0 0 1.5px color-mix(in srgb, var(--solver, #2541f5) 35%, transparent);
  }
</style>
