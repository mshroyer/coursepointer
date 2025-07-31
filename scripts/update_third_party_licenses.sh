#!/bin/sh

# Ensures docs/third_party_licenses.md is up-to-date with our current
# dependency set.

set -e

PROJECT="$(cd $(dirname "$0")/.. && pwd)"

cd "$PROJECT"
cargo about generate "$PROJECT/scripts/about.hbs" \
      -o "$PROJECT/docs/third_party_licenses.md"

cd "$PROJECT/web/coursepointer-wasm"
cargo about generate "$PROJECT/scripts/about.hbs" \
      -o "$PROJECT/docs/web_third_party_licenses.md"
