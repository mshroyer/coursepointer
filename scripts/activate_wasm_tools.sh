#!/bin/bash

# emsdk_env.sh appears to require bash vs. Ubuntu's dash

if [ -z "$BASH_SOURCE" ]; then
	echo "This script needs BASH_SOURCE to be available" >&2
	exit 1
fi

# Use same as in setup_wasm_sdks.sh:
PROJECT=$(cd "$(dirname "$BASH_SOURCE")/.." && pwd)
WASM_TOOLS="$PROJECT/.wasm_tools"
EMSDK="$WASM_TOOLS/emsdk"
WBIND="$WASM_TOOLS/wasm-bindgen"

PATH="$WBIND/bin:$PATH"
. "$EMSDK/emsdk_env.sh"

# Explicitly put Emscripten's tools directory ahead on the path, which
# emsdk_env.sh doesn't do.  This should cause wasm-pack to use Emscripten's
# version of wasm-opt instead of whatever just happens to be on the path or
# else the latest version currently available for download.
PATH="$EMSDK/upstream/bin:$PATH"

# Also reuse Emscripten's pinned NodeJS version.
PATH="$(dirname "$EMSDK_NODE"):$PATH"

export PATH
