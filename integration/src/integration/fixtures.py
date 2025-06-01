from pathlib import Path

import pint
import pytest

from integration.cargo import Cargo, Profile, RustBinFunc


@pytest.fixture
def ureg() -> pint.UnitRegistry:
    return pint.UnitRegistry()


@pytest.fixture
def data() -> Path:
    # The data directory gets built into the wheel by hatchling by default.
    return Path(__file__).parent / "data"


@pytest.fixture(scope="session")
def cargo() -> Cargo:
    return Cargo.default()


@pytest.fixture(scope="session")
def coursepointer_cli(cargo) -> RustBinFunc:
    return cargo.make_bin_func(None, "coursepointer", Profile.TEST)


@pytest.fixture(scope="session")
def integration_stub(cargo):
    return cargo.make_bin_func(
        Path("integration-stub"), "integration-stub", Profile.TEST
    )
