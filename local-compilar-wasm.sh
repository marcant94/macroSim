#!/usr/bin/env bash
# Compila la versión web (wasm) y genera web/pkg/.
# Requisitos: rustup target add wasm32-unknown-unknown
#             cargo install wasm-bindgen-cli --version 0.2.129
set -euo pipefail
cd "$(dirname "$0")"

if ! command -v wasm-bindgen >/dev/null 2>&1; then
  echo "Falta wasm-bindgen-cli. Instálalo con:" >&2
  echo "  cargo install wasm-bindgen-cli --version 0.2.129" >&2
  exit 1
fi

cargo build --release --target wasm32-unknown-unknown
wasm-bindgen \
  --target web \
  --no-typescript \
  --out-dir web/pkg \
  --out-name simulador_politica \
  target/wasm32-unknown-unknown/release/simulador_politica.wasm

echo
echo "Listo. Sirve la carpeta web (no abras el HTML con file://):"
echo "  python3 -m http.server -d web 8080"
echo "  y abre http://localhost:8080"
