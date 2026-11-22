<script lang="ts">
  import raw from '../lib/logo.svg?raw';
  let { size = 40, color = 'var(--solver)' }: { size?: number; color?: string } = $props();
  // el logo usa currentColor: hereda el color que se le dé
  const svg = $derived(raw.replace('width="305" height="281"', `width="${size}" height="${Math.round((size * 281) / 305)}"`));
</script>

<span class="logo" style:color aria-hidden="true">{@html svg}</span>

<style>
  .logo {
    display: inline-flex;
    line-height: 0;
    flex: none;
  }
</style>
