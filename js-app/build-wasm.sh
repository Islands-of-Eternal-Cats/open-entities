#!/bin/sh
set -e
# Resolve script dir to absolute path so copy target is always js-app/public
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WASM_PKG_DIR="$REPO_ROOT/wasm-bindings/pkg"
PUBLIC_DIR="$SCRIPT_DIR/public"
SRC="$WASM_PKG_DIR/wasm_bindings_bg.wasm"
DST="$PUBLIC_DIR/wasm_bindings_bg.wasm"

cd "$REPO_ROOT/wasm-bindings" && wasm-pack build --target web

if [ ! -f "$SRC" ]; then
  echo "Error: wasm-pack did not produce $SRC" >&2
  exit 1
fi

mkdir -p "$PUBLIC_DIR"
cp "$SRC" "$DST"
if ! cmp -s "$SRC" "$DST"; then
  echo "Error: copy to $DST failed (content mismatch)" >&2
  exit 1
fi
echo "WASM copied to $DST"
