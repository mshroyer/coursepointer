"""Test FIT file encoding

Uses integration-stub to write specified FIT course files, then verifies the
results in the Garmin SDK.

"""

from datetime import datetime, timezone

from integration import CourseSpec, read_fit_messages
from integration.fixtures import cargo, integration_stub


def test_empty_course(tmpdir, integration_stub):
    spec = CourseSpec()
    spec.write_file(tmpdir / "spec.json")
    integration_stub("write-fit", "--spec", tmpdir / "spec.json", "--out", tmpdir / "out.fit")
    read_fit_messages(tmpdir / "out.fit")


def test_start_time(tmpdir, integration_stub):
    start_time = datetime(2025, 5, 18, 1, 26, 10, tzinfo=timezone.utc)

    spec = CourseSpec(start_time=start_time)
    spec.write_file(tmpdir / "spec.json")
    integration_stub("write-fit", "--spec", tmpdir / "spec.json", "--out", tmpdir / "out.fit")
    messages = read_fit_messages(tmpdir / "out.fit")

    assert len(messages["lap_mesgs"]) == 1
    assert messages["lap_mesgs"][0]["start_time"] == start_time
