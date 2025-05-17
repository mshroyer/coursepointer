from pathlib import Path
import pytest

from integration.cargo import Cargo, Profile, RustBinFunc


@pytest.fixture
def data() -> Path:
    # The data directory gets built into the wheel by hatchling by default.
    return Path(__file__).parent / "data"


@pytest.fixture(scope="session")
def cargo() -> Cargo:
    return Cargo.default()


@pytest.fixture(scope="session")
def coursepointer_cli(cargo) -> RustBinFunc:
    return cargo.make_bin_func(Path("coursepointer-cli"), "coursepointer-cli", Profile.TEST)


@pytest.fixture(scope="session")
def integration_stub(cargo):
    return cargo.make_bin_func(Path("integration-stub"), "integration-stub", Profile.TEST)
