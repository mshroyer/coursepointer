#!/usr/bin/env python3
"""
Run cargo build or test with feature flags from environment

This provides a cross-platform way to pass custom flag sets to cargo from a
GitHub workflow.

"""

import os
import subprocess
import sys
from typing import List


def run_cargo_with_features(args: List[str]):
    subcommand = args[0]
    default_features = os.getenv("CARGO_DEFAULT_FEATURES", "true") == "true"
    extra_features = os.getenv("CARGO_EXTRA_FEATURES", "")

    full_args = ["cargo", subcommand]
    if extra_features != "":
        full_args.append("-F")
        full_args.append(extra_features)

    if not default_features:
        full_args.append("--no-default-features")

    full_args.extend(args[1:])
    subprocess.run(full_args, check=True)


if __name__ == "__main__":
    run_cargo_with_features(sys.argv[1:])
