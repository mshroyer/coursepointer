#!/bin/sh

set -e

# Ensure wasm-pack picks up the version of wasm-opt in Emscripten instead of
# some arbitrary one.
. "$(dirname "$0")/../../scripts/activate_wasm_sdks.sh"

exec wasm-pack build --target web coursepointer-wasm $@
