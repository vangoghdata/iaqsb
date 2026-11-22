<script lang="ts">
  import { ICONOS, type NombreIcono } from '../lib/iconos';
  let { name, size = 18, stroke = 1.75 }: { name: NombreIcono | string; size?: number; stroke?: number } = $props();
  const d = $derived((ICONOS as Record<string, string>)[name] ?? ICONOS.chispa);
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"><path {d} /></svg
>
