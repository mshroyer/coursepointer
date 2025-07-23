#!/bin/bash

set -e

. "$(dirname "$0")/activate_wasm_sdks.sh"

NODE_OPTIONS='--import=./setup_node_env.js'
export NODE_OPTIONS

wasm-pack test --node -- --no-default-features -F jsffi
