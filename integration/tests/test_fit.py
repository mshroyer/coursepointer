"""Test FIT file encoding

Uses integration-stub to write specified FIT course files, then verifies the
results in the Garmin SDK.

"""

from pathlib import Path

import pytest

from integration import CourseSpec, validate_fit_file
from integration.fixtures import cargo, integration_stub


def test_empty_course(tmpdir, integration_stub):
    spec = CourseSpec([])
    spec.write_file(tmpdir / "spec.json")

    integration_stub("write-fit", "--spec", tmpdir / "spec.json", "--out", tmpdir / "out.fit")
    validate_fit_file(tmpdir / "out.fit")
