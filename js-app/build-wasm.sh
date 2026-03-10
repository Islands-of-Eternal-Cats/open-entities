#!/bin/sh
cd "$(dirname "$0")/../wasm-bindings" && wasm-pack build --target web
