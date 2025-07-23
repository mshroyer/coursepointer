#!/bin/bash

set -e

. "$(dirname "$0")/../../../scripts/activate_wasm_sdks.sh"

exec em++ $@
