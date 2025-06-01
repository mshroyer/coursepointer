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

from integration import (
    CourseSpec,
    garmin_read_messages,
    garmin_read_file_header,
    garmin_sdk_record_coords,
    semicircles_to_degrees,
    assert_all_coords_approx_equal,
    assert_coords_approx_equal,
)


# TODO: Add and test for remaining FIT course fields
# - file_id
#   - Manufacturer
#   - Serial number
# - course
#   - Sport
#   - Sub-sport
# - record
#   - altitude
#   - speed?
# - event
#   - event_group (Garmin Connect sets this to zero)
# - file_creator


def test_header_fields(tmpdir, integration_stub):
    spec = CourseSpec()
    spec.write_file(tmpdir / "spec.json")
    integration_stub(
        "write-fit", "--spec", tmpdir / "spec.json", "--out", tmpdir / "out.fit"
    )
    header = garmin_read_file_header(tmpdir / "out.fit")

    # Protocol version 1 is represented as 0x10, 2 as 0x20.
    assert header.protocol_version == 0x10

    # The output file should encode the same profile version.
    lib_profile_version = int(integration_stub("show-profile-version").stdout.strip())
    assert header.profile_version == lib_profile_version


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
