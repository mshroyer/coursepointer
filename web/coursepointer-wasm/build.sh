#!/bin/bash

set -e

# Ensure wasm-pack picks up the version of wasm-opt in Emscripten instead of
# some arbitrary one.
. "$(dirname "$0")/../../scripts/activate_wasm_sdks.sh"

echo "which wasm-opt: $(which wasm-opt)"

exec wasm-pack build --target web coursepointer-wasm $@
