#!/usr/bin/env python3

"""Runs lints on a new release to make sure aren't forgetting anything.

Before running this:

1. The repo should be tagged with the new release version and in a clean state.
2. The crate's version number should be set correctly.
3. The CHANGELOG should have been updated.
4. The output of

"""

import argparse
from pathlib import Path
import re
import subprocess
import sys
import tomllib
from typing import List, Optional


def workspace_dir() -> Optional[Path]:
    exe_parents = Path(sys.executable).parents
    if len(exe_parents) < 3:
        return None

    workspace = exe_parents[2]
    if not (workspace / ".git").is_dir():
        return None

    return workspace


def crate_version() -> str:
    with open(workspace_dir() / "Cargo.toml", "rb") as f:
        cargo = tomllib.load(f)

    return cargo["package"]["version"]


def last_changelog_version() -> str:
    pattern = re.compile(r"^## v(\d+\.\d+\.\d+)")
    with open(workspace_dir() / "CHANGELOG.md") as f:
        for line in f:
            m = pattern.match(line)
            if m:
                return m.group(1)
    return None


def is_checkout_unmodified() -> bool:
    output = subprocess.check_output(
        ["git", "status", "--porcelain"], cwd=workspace_dir(), universal_newlines=True
    ).strip()
    return len(output) == 0


def is_cargo_about_up_to_date() -> bool:
    with open(workspace_dir() / "docs" / "third_party_licenses.md", "w") as f:
        subprocess.run(
            ["cargo", "about", "generate", "about.hbs"],
            cwd=workspace_dir(),
            universal_newlines=True,
            check=True,
            stdout=f,
        )

    return is_checkout_unmodified()


def rev_parse(rev: str) -> str:
    return subprocess.check_output(
        ["git", "rev-parse", rev],
        cwd=workspace_dir(),
        universal_newlines=True,
    ).strip()


def read_tag(tag: str) -> str:
    return rev_parse(f"tags/{tag}")


def get_tags_at(rev: str) -> List[str]:
    output = subprocess.check_output(
        ["git", "tag", "--points-at", rev],
        cwd=workspace_dir(),
        universal_newlines=True,
    ).strip()
    return output.splitlines()


def get_tagged_version(rev: str) -> Optional[str]:
    pattern = re.compile(r"^v(\d+\.\d+\.\d+)$")
    for tag in get_tags_at(rev):
        m = pattern.match(tag)
        if m is not None:
            return m.group(1)
    return None


def read_head() -> str:
    return rev_parse("HEAD")


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.parse_args()

    if not is_checkout_unmodified():
        print("Git checkout is modified!", file=sys.stderr)
        sys.exit(1)

    version = get_tagged_version("HEAD")
    if version is None:
        print("HEAD has no tagged version", file=sys.stderr)
        sys.exit(1)

    if crate_version() != version:
        print("Crate version mismatch!", file=sys.stderr)
        sys.exit(1)

    if last_changelog_version() != version:
        print("CHANGELOG is not up-to-date!", file=sys.stderr)
        sys.exit(1)

    if not is_cargo_about_up_to_date():
        print("docs/third_party_licenses.md needs to be updated!", file=sys.stderr)
        sys.exit(1)

    print("Release verified.")


if __name__ == "__main__":
    main()
