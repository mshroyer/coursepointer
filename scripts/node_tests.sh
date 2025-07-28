#!/bin/bash

set -e

. "$(dirname "$0")/activate_wasm_sdks.sh"

NODE_OPTIONS='--import=./setup_node_env.js'
export NODE_OPTIONS

#wasm-pack test --node -- --no-default-features -F jsffi

#cargo build --package coursepointer --tests --target wasm32-unknown-unknown \
#      --no-default-features -F jsffi

CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="$(which wasm-bindgen-test-runner)"
export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER

WASM_BINDGEN_TEST_ONLY_NODE="1"
export WASM_BINDGEN_TEST_ONLY_NODE

cargo test --target wasm32-unknown-unknown --no-default-features -F jsffi
