#!/bin/sh

# On a Linux host, this sets up specific versions of our Emscripten and
# wasm-pack toolsets to try to make our builds reproducible-ish on GitHub.

set -e

EMSDK_VERSION=4.0.11
WPACK_VERSION=0.13.1

EMSDK="$HOME/emsdk"
WPACK="$HOME/wasm-pack"
WBIND="$HOME/wasm-bindgen"

PROJECT=$(cd "$(dirname "$0")/.." && pwd)

if [ -d "$EMSDK" ]; then
	cd "$EMSDK"
	# We'll still use the tool from HEAD, but we'll ask it to install a
	# specific SDK version.
	git pull
else
	git clone https://github.com/emscripten-core/emsdk.git "$EMSDK"
	cd "$EMSDK"
fi
cd "$EMSDK"
./emsdk install $EMSDK_VERSION
./emsdk activate $EMSDK_VERSION

wbind_version() {
	grep -A1 'name = "wasm-bindgen"' "$PROJECT/Cargo.lock" \
		| tail -n1 \
		| sed -e 's/version = "\(.*\)"/\1/'
}

cd
if [ ! -d "$WBIND" ]; then
	mkdir "$WBIND"
fi
echo cargo install --version "$(wbind_version)" wasm-bindgen-cli --root "$WBIND"
cargo install --version "$(wbind_version)" wasm-bindgen-cli --root "$WBIND"

if [ ! -d "$WPACK" ]; then
	mkdir "$WPACK"
fi
cd
cargo install --version $WPACK_VERSION wasm-pack --root "$WPACK"
