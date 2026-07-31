#!/bin/sh

# Lint and formatting check actions that commits should pass.

set -e

PROJECT=$(cd "$(dirname "$0")/.." && pwd)
cd "$PROJECT"

cargo +nightly fmt --check
cargo +nightly check

# Invoke ruff with uv to ensure we use the version pinned in our lockfile,
# otherwise stuff can break unexpectedly as new versions of ruff are released.
uv run ruff check
uv run ruff format --check --diff
