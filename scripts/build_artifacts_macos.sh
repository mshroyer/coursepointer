#!/bin/sh

# Builds release artifacts once we have a tagged and verified release.

set -e

cargo build -r
cargo build -r --target x86_64-apple-darwin
lipo -create -output coursepointer target/release/coursepointer target/x86_64-apple-darwin/release/coursepointer
zip coursepointer-macos.zip coursepointer docs/third_party_licenses.md
python3 scripts/release.py upload coursepointer-macos.zip
