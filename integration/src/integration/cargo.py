from enum import Enum, auto
from pathlib import Path
import platform
import subprocess
import sys
from typing import Optional


def workspace_dir() -> Optional[Path]:
    """Get the project's root source directory

    Attempts to resolve the project's root directory, that is to say the local
    git workspace, containing all Rust and Python code, based on the assumption
    we're running in a .venv within that directory.

    """

    exe_parents = Path(sys.executable).parents
    if len(exe_parents) < 3:
        return None

    workspace = exe_parents[2]
    if not (workspace / ".git").is_dir():
        return None

    return workspace


def is_windows() -> bool:
    return platform.system() == "Windows"


def which(program: str) -> Optional[Path]:
    where = "where" if is_windows() else "which"
    try:
        loc = subprocess.check_output([where, program], universal_newlines=True).strip()
        return Path(loc)
    except subprocess.CalledProcessError:
        return None


class NamedEnum(Enum):
    """An enum supporting string conversion of its values"""

    # auto() in StrEnum makes IntelliJ's Python type checking sad, easy enough
    # to roll our own
    def __str__(self) -> str:
        return str(self.name).lower()


class Profile(NamedEnum):
    """A cargo build profile."""

    DEV = auto()
    TEST = auto()
    RELEASE = auto()
    BENCH = auto()

    def target_subdir(self) -> str:
        if self in (self.DEV, self.TEST):
            return "debug"
        else:
            return "release"


class Cargo:
    """The cargo build tool"""

    cargo_bin: Path
    workspace: Path

    def __init__(self, cargo_path: Path, workspace: Path):
        self.cargo_bin = cargo_path
        self.workspace = workspace

    @classmethod
    def default(cls) -> Optional["Cargo"]:
        cargo_bin = which("cargo")
        if cargo_bin is None:
            return None

        return Cargo(cargo_bin, workspace_dir())

    def build_binary(self, package: Path, binary: str, profile: Profile) -> Path:
        """Build a rust binary

        In a package relative to the project's root directory, uses cargo to
        build the named binary with the given profile.  Returns the path to the
        built binary.

        Raises a subprocess.CalledProcessError if the cargo build fails.

        """

        subprocess.check_call(
            [self.cargo_bin, "build", "--package", package, "--bin", binary, "--profile", str(profile)])
        binary_filename = binary + ".exe" if is_windows() else binary

        # This assumes the target directory is in the root of the project
        # directory.  Might want to update this to take into account
        # .cargo/config.toml and CARGO_TARGET_DIR.
        return self.workspace / "target" / profile.target_subdir() / binary_filename
