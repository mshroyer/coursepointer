#!/bin/sh

set -e

NODE_OPTIONS='--import=./setup_node_env.js'
export NODE_OPTIONS

wasm-pack test --node -- --no-default-features -F jsffi
