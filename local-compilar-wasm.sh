#!/usr/bin/env bash
# Compila la versión web (wasm) y genera web/pkg/.
# Requisitos: rustup target add wasm32-unknown-unknown
#             cargo install wasm-bindgen-cli --version 0.2.129
set -euo pipefail
cd "$(dirname "$0")"

# `cargo install` deja los binarios en $CARGO_HOME/bin (~/.cargo/bin), que no
# siempre está en el PATH de la shell: se busca ahí además de en el PATH.
WASM_BINDGEN="${WASM_BINDGEN:-}"
if [[ -z "$WASM_BINDGEN" ]]; then
  if command -v wasm-bindgen >/dev/null 2>&1; then
    WASM_BINDGEN="$(command -v wasm-bindgen)"
  else
    for candidato in "${CARGO_HOME:-$HOME/.cargo}/bin/wasm-bindgen" "$HOME/.cargo/bin/wasm-bindgen"; do
      if [[ -x "$candidato" ]]; then
        WASM_BINDGEN="$candidato"
        break
      fi
    done
  fi
fi

if [[ -z "$WASM_BINDGEN" ]]; then
  echo "Falta wasm-bindgen-cli. Instálalo con:" >&2
  echo "  cargo install wasm-bindgen-cli --version 0.2.129" >&2
  exit 1
fi

# La CLI debe coincidir con la versión del crate `wasm-bindgen` (Cargo.lock);
# si no, el error que da más adelante es confuso.
VERSION_CLI="$("$WASM_BINDGEN" --version | awk '{print $2}')"
VERSION_CRATE="$(awk '/^name = "wasm-bindgen"$/ {getline; gsub(/[",]/, "", $3); print $3}' Cargo.lock | head -1)"
if [[ -n "$VERSION_CRATE" && "$VERSION_CLI" != "$VERSION_CRATE" ]]; then
  echo "wasm-bindgen-cli es $VERSION_CLI pero el crate usa $VERSION_CRATE." >&2
  echo "Instala la versión correcta:" >&2
  echo "  cargo install wasm-bindgen-cli --version $VERSION_CRATE --force" >&2
  exit 1
fi

cargo build --release --target wasm32-unknown-unknown
"$WASM_BINDGEN" \
  --target web \
  --no-typescript \
  --out-dir web/pkg \
  --out-name simulador_politica \
  target/wasm32-unknown-unknown/release/simulador_politica.wasm

echo
echo "Listo. Sirve la carpeta web (no abras el HTML con file://):"
echo "  python3 -m http.server -d web 8080"
echo "  y abre http://localhost:8080"
