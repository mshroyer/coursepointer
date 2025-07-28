#!/bin/bash

set -e

. "$(dirname "$0")/../../../scripts/activate_wasm_tools.sh"

exec em++ $@
