#!/bin/sh
set -e
SCRIPT_DIR="$(dirname "$0")"
cd "$SCRIPT_DIR/../wasm-bindings" && wasm-pack build --target web
mkdir -p "$SCRIPT_DIR/public"
cp "$SCRIPT_DIR/../wasm-bindings/pkg/wasm_bindings_bg.wasm" "$SCRIPT_DIR/public/wasm_bindings_bg.wasm"
