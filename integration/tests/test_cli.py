"""Test the main coursepointer-cli binary"""


from pytest import approx

from integration import garmin_sdk_read_fit_messages, garmin_sdk_read_fit_header, assert_all_coords_approx_equal, \
    garmin_sdk_get_lap_distance_meters
from integration.fixtures import cargo, data, coursepointer_cli


def test_help(coursepointer_cli):
    assert "Print help" in coursepointer_cli("--help")


def test_conversion_valid(tmpdir, data, coursepointer_cli):
    coursepointer_cli("convert-gpx", "--input", data / "cptr002.gpx", "--output", tmpdir / "out.fit")
    garmin_sdk_read_fit_messages(tmpdir / "out.fit")


def test_conversion_course_name(tmpdir, data, coursepointer_cli):
    coursepointer_cli("convert-gpx", "--input", data / "cptr002.gpx", "--output", tmpdir / "out.fit")
    messages = garmin_sdk_read_fit_messages(tmpdir / "out.fit")

    # The file should have a single course message containing the same track
    # name given in the GPX input.
    course_mesgs = messages["course_mesgs"]
    assert len(course_mesgs) == 1
    assert course_mesgs[0]["name"] == "cptr002"


def test_conversion_distance(tmpdir, data, coursepointer_cli):
    coursepointer_cli("convert-gpx", "--input", data / "cptr003.gpx", "--output", tmpdir / "out.fit")
    conversion_distance = garmin_sdk_get_lap_distance_meters(tmpdir / "out.fit")
    expected_distance = garmin_sdk_get_lap_distance_meters(data / "cptr003_connect.fit")

    assert conversion_distance == approx(expected_distance)
