#!/bin/sh

# Ensures docs/third_party_licenses.md is up-to-date with our current
# dependency set.

set -e

cargo about generate about.hbs > docs/third_party_licenses.md
