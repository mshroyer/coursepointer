from enum import StrEnum, auto
from pathlib import Path


class Profile(StrEnum):
    """A cargo build profile."""

    DEV = auto()
    TEST = auto()
    RELEASE = auto()
    BENCH = auto()


class Cargo:
    """The cargo build tool"""

    cargo_bin: Path
    project: Path

    def __init__(self, cargo_path: Path, project: Path):
        self.cargo_bin = cargo_path
        self.project = project

    def build_binary(self, package: Path, binary: str, profile: Profile) -> Path:
        """Build a rust binary

        In a package relative to the project's root directory, uses cargo to
        build the named binary with the given profile.  Returns the path to the
        built binary.

        """

        return self.project / "target" / str(profile) / binary
