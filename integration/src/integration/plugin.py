from collections import namedtuple
from pathlib import Path
import shutil
import subprocess
import tempfile

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


CachedConversion = namedtuple("CachedConversion", ["out_file", "exception"])


@pytest.fixture(scope="session")
def caching_convert(coursepointer_cli):
    session_dir = Path(tempfile.mkdtemp())
    cache = {}

    def _convert(input: Path) -> Path:
        if input in cache:
            cached = cache[input]
            if cached.exception:
                raise integration.fail_with_subprocess_error(cached.exception)
            return cached.out_file

        out_dir = Path(tempfile.mkdtemp(dir=session_dir))
        out_file = out_dir / "out.fit"
        try:
            coursepointer_cli("convert-gpx", input, out_file)
        except subprocess.CalledProcessError as e:
            cache[input] = CachedConversion(out_file, e)
            raise integration.fail_with_subprocess_error(e)

        cache[input] = CachedConversion(out_file, None)
        return out_file

    yield _convert

    shutil.rmtree(session_dir)


@pytest.fixture(scope="session")
def caching_mesgs():
    cache = {}

    def _read_mesgs(file: Path) -> dict:
        if file in cache:
            return cache[file]

        mesgs = integration.garmin_read_messages(file)
        cache[file] = mesgs
        return mesgs

    return _read_mesgs


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
