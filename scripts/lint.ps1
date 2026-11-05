$ErrorActionPreference = 'Stop'

cargo +nightly fmt --check
cargo +nightly check

uv run ruff check
uv run ruff format --check --diff
