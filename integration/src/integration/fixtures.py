from pathlib import Path

import subprocess
import pint
import pytest

import integration
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


# Wrap test case invocations to clarify subprocess errors
#
# Wraps each test function so that we can capture any uncaught
# CalledProcessErrors and instead fail the test, printing the subprocess's
# stdout and stderr without an excessive and unhelpful Python stack trace.
#
# Ideally, we could just implement pytest_runtest_call instead, but as of pytest
# 8.3.5 doing this without `wraptest` this results in duplicate invocations of
# the test method--and with it, an generator we can't use to intercept the
# exception.


@pytest.hookimpl
def pytest_itemcollected(item):
    item.runtest_wrapped = item.runtest
    item.runtest = _runtest.__get__(item, item.__class__)


def _runtest(self):
    try:
        self.runtest_wrapped()
    except subprocess.CalledProcessError as e:
        integration.fail_with_subprocess_error(e)
