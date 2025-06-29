#!/bin/sh

# Builds release artifacts once we have a tagged and verified release.

set -e

VERSION=$(python3 scripts/release.py head)

cargo build -r
cargo build -r --target x86_64-apple-darwin
lipo -create -output coursepointer target/release/coursepointer target/x86_64-apple-darwin/release/coursepointer
cp docs/bdist_readme.txt README.txt
zip -j coursepointer-macos-v${VERSION}.zip coursepointer README.txt LICENSE.txt docs/third_party_licenses.md
python3 scripts/release.py upload coursepointer-macos-v${VERSION}.zip
