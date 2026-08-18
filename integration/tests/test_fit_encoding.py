"""Test FIT file encoding

Uses integration-stub to write specified FIT course files, then verifies using
the Garmin SDK that the crate's output is valid and that various elements are
written correctly to the course file.

Because the FIT profile specifies unit scaling for some values and uncommon
conventions (e.g., the Garmin epoch) for others, one goal of these tests is to
ensure our implementation scales values appropriately.  Where possible, such as
when interpreting date_time values, we rely on the SDK's own logic as a
reference implementation.  In other cases, like when converting from semicircles
back into degrees of latitude and longitude, the SDK does not provide the
conversion, so we implement our own in Python.

"""

from datetime import datetime, timezone

from pytest import approx

from integration import (
    CourseSpec,
    fitdecode_record_field_names,
    garmin_read_messages,
    garmin_sdk_record_coords,
    semicircles_to_degrees,
    assert_all_coords_approx_equal,
    assert_coords_approx_equal,
)


# TODO: Add and test for remaining FIT course fields
#   - Serial number
# - course
#   - Sub-sport
# - record
#   - speed?
# - event
#   - event_group (Garmin Connect sets this to zero)


# The altitude field's resolution is 20cm, per its scale of 5 in the FIT
# profile, so encoding rounds by up to 10cm.  Allow a little more than that so
# that values right on the boundary don't fail on floating point error.
ALTITUDE_ABS_TOLERANCE = 0.11


def test_start_time(tmpdir, integration_stub):
    start_time = datetime(2025, 5, 18, 1, 26, 10, tzinfo=timezone.utc)

    spec = CourseSpec(start_time=start_time)
    spec.write_file(tmpdir / "spec.json")
    integration_stub(
        "write-fit", "--spec", tmpdir / "spec.json", "--out", tmpdir / "out.fit"
    )
    messages = garmin_read_messages(tmpdir / "out.fit")

    # The course's start time should be encoded correctly as the lap message's
    # start time.
    assert messages["lap_mesgs"][0]["start_time"] == start_time
    assert messages["lap_mesgs"][0]["timestamp"] == start_time

    # ...and also as the timestamp of the start event message.
    first_event = messages["event_mesgs"][0]
    assert first_event["event_type"] == "start"
    assert first_event["timestamp"] == start_time


def test_course_name(tmpdir, integration_stub):
    course_name = "Foo Course"

    spec = CourseSpec(name=course_name)
    spec.write_file(tmpdir / "spec.json")
    integration_stub(
        "write-fit", "--spec", tmpdir / "spec.json", "--out", tmpdir / "out.fit"
    )
    messages = garmin_read_messages(tmpdir / "out.fit")

    assert messages["course_mesgs"][0]["name"] == course_name


def test_course_name_truncation(tmpdir, integration_stub):
    course_name = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore"

    spec = CourseSpec(name=course_name)
    spec.write_file(tmpdir / "spec.json")
    integration_stub(
        "write-fit", "--spec", tmpdir / "spec.json", "--out", tmpdir / "out.fit"
    )
    messages = garmin_read_messages(tmpdir / "out.fit")

    # The course name should be truncated to 31 characters, as the field size is
    # configured to 32.
    expected = "Lorem ipsum dolor sit amet, con"
    assert messages["course_mesgs"][0]["name"] == expected


def test_record_coords(tmpdir, integration_stub):
    coords = [(0.0, 0.0), (0.5, -0.5), (1.0, 0.0), (-1.0, 0.5)]

    spec = CourseSpec(records=coords)
    spec.write_file(tmpdir / "spec.json")
    integration_stub(
        "write-fit", "--spec", tmpdir / "spec.json", "--out", tmpdir / "out.fit"
    )
    messages = garmin_read_messages(tmpdir / "out.fit")

    assert_all_coords_approx_equal(
        list(map(garmin_sdk_record_coords, messages["record_mesgs"])), coords
    )


def test_record_altitudes(tmpdir, integration_stub):
    # The FIT profile stores altitude as a uint16 scaled by 5 with an offset of
    # 500m, so include elevations on either side of sea level, which the offset
    # puts in the middle of the field's range.
    records = [
        (0.0, 0.0, 0.0),
        (0.5, -0.5, 93.4),
        (1.0, 0.0, -211.7),
        (-1.0, 0.5, 8848.9),
    ]

    spec = CourseSpec(records=records)
    spec.write_file(tmpdir / "spec.json")
    integration_stub(
        "write-fit", "--spec", tmpdir / "spec.json", "--out", tmpdir / "out.fit"
    )
    messages = garmin_read_messages(tmpdir / "out.fit")

    altitudes = [record["altitude"] for record in messages["record_mesgs"]]
    assert altitudes == approx(
        [record[2] for record in records], abs=ALTITUDE_ABS_TOLERANCE
    )


def test_record_altitudes_absent(tmpdir, integration_stub):
    # Records without elevation shouldn't get an altitude field at all, rather
    # than one holding a made-up or invalid value.
    records = [(0.0, 0.0), (0.5, -0.5), (1.0, 0.0)]

    spec = CourseSpec(records=records)
    spec.write_file(tmpdir / "spec.json")
    integration_stub(
        "write-fit", "--spec", tmpdir / "spec.json", "--out", tmpdir / "out.fit"
    )

    assert "altitude" not in fitdecode_record_field_names(tmpdir / "out.fit")


def test_record_altitudes_partial(tmpdir, integration_stub):
    # When only some records have an elevation, no altitudes are written at
    # all: Edge and fenix devices show a record whose altitude is the profile's
    # invalid value as an absurdly high elevation instead of a missing one.
    records = [(0.0, 0.0, 12.5), (0.5, -0.5), (1.0, 0.0, 30.0)]

    spec = CourseSpec(records=records)
    spec.write_file(tmpdir / "spec.json")
    integration_stub(
        "write-fit", "--spec", tmpdir / "spec.json", "--out", tmpdir / "out.fit"
    )

    assert "altitude" not in fitdecode_record_field_names(tmpdir / "out.fit")


def test_record_altitudes_out_of_range(tmpdir, integration_stub):
    # An elevation the profile's altitude field can't represent counts as
    # missing too, so one bad value takes the whole course's altitudes with it
    # rather than being written as a wrapped-around or invalid value.
    records = [(0.0, 0.0, 100.0), (0.5, -0.5, 20000.0), (1.0, 0.0, 200.0)]

    spec = CourseSpec(records=records)
    spec.write_file(tmpdir / "spec.json")
    integration_stub(
        "write-fit", "--spec", tmpdir / "spec.json", "--out", tmpdir / "out.fit"
    )

    assert "altitude" not in fitdecode_record_field_names(tmpdir / "out.fit")


def test_record_altitudes_at_range_limits(tmpdir, integration_stub):
    # The extremes of what the altitude field can represent are still written.
    records = [(0.0, 0.0, -500.0), (0.5, -0.5, 12606.8)]

    spec = CourseSpec(records=records)
    spec.write_file(tmpdir / "spec.json")
    integration_stub(
        "write-fit", "--spec", tmpdir / "spec.json", "--out", tmpdir / "out.fit"
    )
    messages = garmin_read_messages(tmpdir / "out.fit")

    altitudes = [record["altitude"] for record in messages["record_mesgs"]]
    assert altitudes == approx([-500.0, 12606.8], abs=ALTITUDE_ABS_TOLERANCE)


def test_lap_coords(tmpdir, integration_stub):
    coords = [(0.0, 0.0), (0.5, -0.5), (1.0, 0.0), (-1.0, 0.5)]

    spec = CourseSpec(records=coords)
    spec.write_file(tmpdir / "spec.json")
    integration_stub(
        "write-fit", "--spec", tmpdir / "spec.json", "--out", tmpdir / "out.fit"
    )
    messages = garmin_read_messages(tmpdir / "out.fit")

    lap = messages["lap_mesgs"][0]
    assert_coords_approx_equal(
        semicircles_to_degrees((lap["start_position_lat"], lap["start_position_long"])),
        coords[0],
    )
    assert_coords_approx_equal(
        semicircles_to_degrees((lap["end_position_lat"], lap["end_position_long"])),
        coords[-1],
    )
