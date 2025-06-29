"""Runs lints on a new release to make sure aren't forgetting anything.

Before running this:

1. The repo should be tagged with the new release version and in a clean state.
2. The crate's version number should be set correctly.
3. The CHANGELOG should have been updated.
4. The output of

"""

import argparse
import re
import subprocess
import sys
import tomllib

from integration.cargo import workspace_dir


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


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("version", type=str, help="New release version")
    args = parser.parse_args()

    if not is_checkout_unmodified():
        print("Git checkout is modified!")
        sys.exit(1)

    if crate_version() != args.version:
        print("Crate version mismatch!", file=sys.stderr)
        sys.exit(1)

    if last_changelog_version() != args.version:
        print("CHANGELOG is not up-to-date!", file=sys.stderr)
        sys.exit(1)

    if not is_cargo_about_up_to_date():
        print("docs/third_party_licenses.md needs to be updated!")
        sys.exit(1)


if __name__ == "__main__":
    main()
