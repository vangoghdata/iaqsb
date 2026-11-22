#!/usr/bin/env bash
# Compila IAQSB en Linux o macOS y genera los instaladores de esa plataforma.
# Uso: scripts/build.sh            (solo los instaladores)
#      scripts/build.sh --pruebas  (pruebas y luego los instaladores)
set -euo pipefail
cd "$(dirname "$0")/.."
if [ "${1:-}" = "--pruebas" ]; then
  # perfil «pruebas»: optimizado y sin LTO (enlaza mucho más rápido que --release)
  cargo test --profile pruebas
fi
cargo build --release -p iaqsb -p iaqsb-mcp
triple=$(rustc --print host-tuple)
bin=apps/desktop/src-tauri/binaries
mkdir -p "$bin"
cp target/release/iaqsb-mcp "$bin/iaqsb-mcp-$triple"
cp target/release/iaqsb "$bin/iaqsb-$triple"
cd apps/desktop
npm install
# en Windows el instalador es solo NSIS (tauri.conf.json); aquí, los formatos de esta plataforma
case "$(uname -s)" in
  Darwin) bundles="app,dmg" ;;
  *) bundles="deb,appimage" ;;
esac
npx tauri build --bundles "$bundles"
echo "Instaladores en target/release/bundle/"
