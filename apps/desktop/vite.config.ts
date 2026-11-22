import { defineConfig, type Plugin } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { writeFileSync } from 'node:fs';
import { SCRIPT_TEMA } from './src/lib/tema';

// El tema se aplica antes de pintar con un script pequeño. Va en un archivo
// aparte (no en línea) porque la política de seguridad de Tauri solo deja
// cargar scripts propios. Se genera desde src/lib/tema.ts para no repetirlo.
function temaInicial(): Plugin {
  return {
    name: 'iaqsb-tema-inicial',
    buildStart() {
      writeFileSync(new URL('./public/tema-inicial.js', import.meta.url), SCRIPT_TEMA + '\n');
    }
  };
}

// Tauri espera un puerto fijo y no debe abrir el navegador
export default defineConfig({
  plugins: [temaInicial(), svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // en modo navegador (sin Tauri), las llamadas a la API van al servidor de desarrollo
    proxy: { '/api': 'http://127.0.0.1:1421' }
  },
  build: { target: 'es2022', outDir: 'dist', emptyOutDir: true }
});
