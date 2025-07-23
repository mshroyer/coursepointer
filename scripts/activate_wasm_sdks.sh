#!/bin/sh

# Use same as in setup_wasm_sdks.sh:
EMSDK="$HOME/emsdk"
WPACK="$HOME/wasm-pack"

PATH="$WPACK/bin:$PATH"
. "$EMSDK/emsdk_env.sh"

# Explicitly put Emscripten's tools directory ahead on the path, which
# emsdk_env.sh doesn't do.  This should cause wasm-pack to use Emscripten's
# version of wasm-opt instead of whatever just happens to be on the path or
# else the latest version currently available for download.
PATH="$EMSDK/upstream/bin:$PATH"

# Also reuse Emscripten's pinned NodeJS version.
PATH="$(dirname "$EMSDK_NODE"):$PATH"

export PATH
