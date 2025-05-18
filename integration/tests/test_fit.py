"""Test FIT file encoding

Uses integration-stub to write specified FIT course files, then verifies the
results in the Garmin SDK.

"""

from datetime import datetime, timezone
from typing import List, Tuple

from pytest import approx

from integration import CourseSpec, garmin_sdk_read_fit
from integration.fixtures import cargo, integration_stub


def test_empty_course(tmpdir, integration_stub):
    spec = CourseSpec()
    spec.write_file(tmpdir / "spec.json")
    integration_stub("write-fit", "--spec", tmpdir / "spec.json", "--out", tmpdir / "out.fit")
    garmin_sdk_read_fit(tmpdir / "out.fit")


def test_start_time(tmpdir, integration_stub):
    start_time = datetime(2025, 5, 18, 1, 26, 10, tzinfo=timezone.utc)

    spec = CourseSpec(start_time=start_time)
    spec.write_file(tmpdir / "spec.json")
    integration_stub("write-fit", "--spec", tmpdir / "spec.json", "--out", tmpdir / "out.fit")
    messages = garmin_sdk_read_fit(tmpdir / "out.fit")

    # The course's start time should be encoded correctly as the lap message's
    # start time.
    assert messages["lap_mesgs"][0]["start_time"] == start_time

    # ...and also as the timestamp of the start event message.
    first_event = messages["event_mesgs"][0]
    assert first_event["event_type"] == "start"
    assert first_event["timestamp"] == start_time


def test_course_name(tmpdir, integration_stub):
    course_name = "Foo Course"
    spec = CourseSpec(name=course_name)
    spec.write_file(tmpdir / "spec.json")
    integration_stub("write-fit", "--spec", tmpdir / "spec.json", "--out", tmpdir / "out.fit")
    messages = garmin_sdk_read_fit(tmpdir / "out.fit")

    assert messages["course_mesgs"][0]["name"] == course_name


def record_coords(record: dict) -> Tuple[float, float]:
    """Get coordinate tuple for a record message

    Returns a (lat, lon) tuple in decimal degrees for the given FIT record
    message, as returned by the Garmin SDK.

    """
    lat = 180 * record["position_lat"] / 2**31
    lon = 180 * record["position_long"] / 2**31
    return lat, lon


def assert_coords_approx_eq(a: List[Tuple[float, float]], b: List[Tuple[float, float]]) -> None:
    assert len(a) == len(b)
    for i in range(len(a)):
        lat_a = a[i][0]
        lon_a = a[i][1]
        lat_b = b[i][0]
        lon_b = b[i][1]

        assert lat_a == approx(lat_b)
        assert lon_a == approx(lon_b)


def test_record_coords(tmpdir, integration_stub):
    coords = [(0.0, 0.0), (0.5, -0.5), (1.0, 0.0), (-1.0, 0.5)]

    spec = CourseSpec(records=coords)
    spec.write_file(tmpdir / "spec.json")
    integration_stub("write-fit", "--spec", tmpdir / "spec.json", "--out", tmpdir / "out.fit")
    messages = garmin_sdk_read_fit(tmpdir / "out.fit")

    assert_coords_approx_eq(list(map(record_coords, messages["record_mesgs"])), coords)
