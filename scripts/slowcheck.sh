#!/bin/sh

# Run quickchecks in a loop until one fails.

set -e

export QUICKCHECK_TESTS=100000
export QUICKCHECK_MAX_TESTS=1000000
export RUST_LOG=quickcheck

while true
do
	cargo test -r qc_ -- --nocapture
done
