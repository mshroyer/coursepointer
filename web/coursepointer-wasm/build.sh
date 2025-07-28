#!/bin/bash

set -e

PROJECT=$(cd "$(dirname "$0")/../.." && pwd)

# Ensure wasm-pack picks up the version of wasm-opt in Emscripten instead of
# some arbitrary one.
. "$PROJECT/scripts/activate_wasm_sdks.sh"

echo "which wasm-opt: $(which wasm-opt)"

#exec wasm-pack build --target web coursepointer-wasm $@

RELEASE_DIR="$PROJECT/target/wasm32-unknown-unknown/release"
PKG_DIR="$PROJECT/web/coursepointer-wasm/pkg"
WASM_FILE="coursepointer_wasm.wasm"
BG_WASM_FILE="coursepointer_wasm_bg.wasm"
OPT_WASM_FILE="coursepointer_wasm_bg.wasm-opt.wasm"

cargo build --lib --release --target wasm32-unknown-unknown \
      --package coursepointer-wasm

wasm-bindgen "$RELEASE_DIR/$WASM_FILE" \
	     --out-dir "$PKG_DIR" \
	     --typescript --target web

wasm-opt "$PKG_DIR/$BG_WASM_FILE" \
	 -o "$PKG_DIR/$OPT_WASM_FILE" \
	 -O3

mv -f "$PKG_DIR/$OPT_WASM_FILE" "$PKG_DIR/$BG_WASM_FILE"
