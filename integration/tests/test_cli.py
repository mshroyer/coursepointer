"""Test the main coursepointer-cli binary"""

from itertools import pairwise
import subprocess

from pytest import approx, raises

from integration import garmin_sdk_read_fit_messages, garmin_sdk_read_fit_header, assert_all_coords_approx_equal, \
    garmin_sdk_get_lap_distance_meters
from integration.fixtures import cargo, data, coursepointer_cli


def test_help(coursepointer_cli):
    assert "Print help" in coursepointer_cli("--help")


def test_no_subcommand(coursepointer_cli):
    with raises(subprocess.CalledProcessError) as einfo:
        coursepointer_cli()

    assert "Usage:" in einfo.value.output


def test_missing_input(tmpdir, coursepointer_cli):
    with raises(subprocess.CalledProcessError) as einfo:
        coursepointer_cli("convert-gpx", tmpdir / "nonexistent.gpx", tmpdir / "out.fit")

    assert "Reading the GPX <INPUT> file" in einfo.value.output
    assert "I/O error" in einfo.value.output


def test_output_file_exists(tmpdir, data, coursepointer_cli):
    with open(tmpdir / "out.fit", "w") as f:
        print("Hello", file=f)

    with raises(subprocess.CalledProcessError) as einfo:
        coursepointer_cli("convert-gpx", data / "cptr002.gpx", tmpdir / "out.fit")

    assert "The file exists" in einfo.value.output


def test_output_file_force(tmpdir, data, coursepointer_cli):
    with open(tmpdir / "out.fit", "w") as f:
        print("Hello", file=f)

    coursepointer_cli("convert-gpx", data / "cptr002.gpx", tmpdir / "out.fit", "--force")


def test_no_courses(tmpdir, data, coursepointer_cli):
    with raises(subprocess.CalledProcessError) as einfo:
        coursepointer_cli("convert-gpx", data / "invalid_empty.gpx", tmpdir / "out.fit")

    assert "No course was found" in einfo.value.output


def test_bad_xml(tmpdir, data, coursepointer_cli):
    with raises(subprocess.CalledProcessError) as einfo:
        coursepointer_cli("convert-gpx", data / "invalid_bad_xml.gpx", tmpdir / "out.fit")

    assert "<INPUT> is not a valid GPX file" in einfo.value.output


def test_conversion_valid(tmpdir, data, coursepointer_cli):
    coursepointer_cli("convert-gpx", data / "cptr002.gpx", tmpdir / "out.fit")
    garmin_sdk_read_fit_messages(tmpdir / "out.fit")


def test_conversion_course_name(tmpdir, data, coursepointer_cli):
    coursepointer_cli("convert-gpx", data / "cptr002.gpx", tmpdir / "out.fit")
    messages = garmin_sdk_read_fit_messages(tmpdir / "out.fit")

    # The file should have a single course message containing the same track
    # name given in the GPX input.
    course_mesgs = messages["course_mesgs"]
    assert len(course_mesgs) == 1
    assert course_mesgs[0]["name"] == "cptr002"


def test_conversion_total_distance(tmpdir, data, coursepointer_cli):
    coursepointer_cli("convert-gpx", data / "cptr003.gpx", tmpdir / "out.fit")

    # Make sure the converted FIT file's lap distance is about equal to that of
    # the FIT file we get when importing the GPX into Garmin Connect and then
    # re-exporting it as FIT.  We use the Connect re-export because this puts
    # the code under test on equal footing given the limited precision of
    # RWGPS's GPX exports.
    conversion_lap_distance = garmin_sdk_get_lap_distance_meters(tmpdir / "out.fit")
    expected_lap_distance = garmin_sdk_get_lap_distance_meters(data / "cptr003_connect.fit")
    assert conversion_lap_distance == approx(expected_lap_distance)


def test_conversion_record_distances(tmpdir, data, coursepointer_cli):
    coursepointer_cli("convert-gpx", data / "cptr003.gpx", tmpdir / "out.fit")

    records = garmin_sdk_read_fit_messages(tmpdir / "out.fit")["record_mesgs"]
    assert records[0]["distance"] == 0

    # Distances should be cumulative
    for a, b in pairwise(records):
        assert a["distance"] <= b["distance"]

    # The final record's distance should be equal to the course file's lap
    # distance
    assert records[-1]["distance"] == garmin_sdk_get_lap_distance_meters(tmpdir / "out.fit")
