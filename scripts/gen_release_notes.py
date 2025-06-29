#!/usr/bin/env python3

"""Generate release notes

Extract a specific version's release notes from CHANGELOG.md.

"""

import argparse
import sys
from pathlib import Path
from typing import Optional


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("version", type=str, help="Version number")
    args = parser.parse_args()

    with open("CHANGELOG.md") as r:
        with open("release_notes.md", "w") as w:
            current_version = False
            past_padding = False
            for line in r:
                if current_version:
                    if line.startswith("## "):
                        return

                    if line.strip() != "":
                        past_padding = True
                    if past_padding:
                        print(line.strip(), file=w)
                elif line.strip() == f"## v{args.version}":
                    current_version = True


if __name__ == "__main__":
    main()
