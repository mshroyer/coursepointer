#!/usr/bin/env python3

# Ensures a CI run has completed for the commit.

import argparse
import json
import subprocess
from typing import Optional
import sys


def query_ci_runs(sha: str) -> dict:
    output = subprocess.check_output(
        [
            "gh",
            "api",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "X-GitHub-Api-Version: 2022-11-28",
            f"/repos/mshroyer/coursepointer/actions/workflows/ci.yml/runs?head_sha={sha}",
        ]
    )

    runs = json.loads(output)
    return runs["workflow_runs"]


def successful_run_id(workflow_runs: dict) -> Optional[int]:
    for run in workflow_runs:
        if (
            run["status"] == "completed"
            and run["conclusion"] == "success"
            and run["event"] == "push"
        ):
            return run["id"]
    return False


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("hash", type=str, help="Commit hash")
    args = parser.parse_args()

    id = successful_run_id(query_ci_runs(args.hash))
    if id is None:
        print(f"No successful CI run for commit {args.hash} found", file=sys.stderr)
        sys.exit(1)

    print(
        f"Found successful CI run https://github.com/mshroyer/coursepointer/actions/runs/{id} for commit {args.hash}"
    )


if __name__ == "__main__":
    main()
