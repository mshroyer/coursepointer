"""Test the main coursepointer-cli binary"""

from itertools import pairwise
import subprocess

from pytest import approx, raises

from integration import garmin_read_messages


def test_help(coursepointer_cli):
    assert "Print help" in coursepointer_cli("--help")


def test_no_subcommand(coursepointer_cli):
    with raises(subprocess.CalledProcessError) as einfo:
        coursepointer_cli()

    assert "Usage:" in einfo.value.output


def test_missing_input(tmpdir, coursepointer_cli):
    with raises(subprocess.CalledProcessError) as einfo:
        coursepointer_cli("convert-gpx", tmpdir / "nonexistent.gpx", tmpdir / "out.fit")

    assert "Opening the GPX <INPUT> file" in einfo.value.output


def test_output_file_exists(tmpdir, data, coursepointer_cli):
    with open(tmpdir / "out.fit", "w") as f:
        print("Hello", file=f)

    with raises(subprocess.CalledProcessError) as einfo:
        coursepointer_cli("convert-gpx", data / "cptr002.gpx", tmpdir / "out.fit")

    # Error message can vary slightly by platform
    assert "file exists" in einfo.value.output.lower()


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
    garmin_read_messages(tmpdir / "out.fit")


def test_course_name(tmpdir, data, coursepointer_cli):
    coursepointer_cli("convert-gpx", data / "cptr002.gpx", tmpdir / "out.fit")
    messages = garmin_read_messages(tmpdir / "out.fit")

    # The file should have a single course message containing the same track
    # name given in the GPX input.
    course_mesgs = messages["course_mesgs"]
    assert len(course_mesgs) == 1
    assert course_mesgs[0]["name"] == "cptr002"


def test_lap_distance(tmpdir, data, coursepointer_cli):
    coursepointer_cli("convert-gpx", data / "cptr003.gpx", tmpdir / "out.fit")

    # Make sure the converted FIT file's lap distance is about equal to that of
    # the FIT file we get when importing the GPX into Garmin Connect and then
    # re-exporting it as FIT.  We use the Connect re-export because this puts
    # the code under test on equal footing given the limited precision of
    # RWGPS's GPX exports.
    conversion_lap_distance = garmin_read_messages(tmpdir / "out.fit")["lap_mesgs"][0]["total_distance"]
    expected_lap_distance = garmin_read_messages(data / "cptr003_connect.fit")["lap_mesgs"][0]["total_distance"]
    assert conversion_lap_distance == approx(expected_lap_distance)


def test_lap_duration(tmpdir, data, ureg, coursepointer_cli):
    coursepointer_cli("convert-gpx", data / "cptr003.gpx", tmpdir / "out.fit")

    speed = 20 * ureg.kilometer / ureg.hour
    lap_mesgs = garmin_read_messages(tmpdir / "out.fit")["lap_mesgs"]
    assert len(lap_mesgs) == 1

    lap_distance = lap_mesgs[0]["total_distance"] * ureg.meter
    lap_elapsed = lap_mesgs[0]["total_timer_time"] * ureg.second
    lap_timer = lap_mesgs[0]["total_elapsed_time"] * ureg.second

    assert lap_elapsed == lap_timer
    assert lap_distance.magnitude == approx((lap_timer * speed).to(ureg.meter).magnitude, rel=0.000_100)


def test_record_distances(tmpdir, data, coursepointer_cli):
    coursepointer_cli("convert-gpx", data / "cptr003.gpx", tmpdir / "out.fit")

    mesgs = garmin_read_messages(tmpdir / "out.fit")
    record_mesgs = mesgs["record_mesgs"]
    assert record_mesgs[0]["distance"] == 0

    # Distances should be cumulative
    for a, b in pairwise(record_mesgs):
        assert a["distance"] <= b["distance"]

    lap_mesgs = mesgs["lap_mesgs"]
    assert len(lap_mesgs) == 1

    # The final record's distance should be equal to the course file's lap
    # distance
    assert record_mesgs[-1]["distance"] == lap_mesgs[0]["total_distance"]


def test_record_timestamps(tmpdir, data, ureg, coursepointer_cli):
    coursepointer_cli("convert-gpx", data / "cptr003.gpx", tmpdir / "out.fit")

    speed = 20 * ureg.kilometer / ureg.hour
    mesgs = garmin_read_messages(tmpdir / "out.fit")
    record_mesgs = mesgs["record_mesgs"]

    start_timestamp = record_mesgs[0]["timestamp"]
    for record in record_mesgs:
        expected_duration = (record["distance"] * ureg.meter / speed).to(ureg.second)
        actual_duration = record["timestamp"] - start_timestamp
        assert actual_duration.seconds == approx(expected_duration.magnitude, abs=1)


def test_timer_event_spacing(tmpdir, data, coursepointer_cli):
    coursepointer_cli("convert-gpx", data / "cptr003.gpx", tmpdir / "out.fit")
    mesgs = garmin_read_messages(tmpdir / "out.fit")

    event_mesgs = mesgs["event_mesgs"]
    assert len(event_mesgs) == 2
    assert event_mesgs[0]["event"] == "timer"
    assert event_mesgs[0]["event_type"] == "start"
    assert event_mesgs[1]["event"] == "timer"
    assert event_mesgs[1]["event_type"] == "stop"

    lap_mesgs = mesgs["lap_mesgs"]
    lap_elapsed = lap_mesgs[0]["total_timer_time"]

    event_spacing = event_mesgs[1]["timestamp"] - event_mesgs[0]["timestamp"]
    assert event_spacing.seconds == lap_elapsed


def test_gpx_rte_conversion(tmpdir, data, ureg, coursepointer_cli):
    coursepointer_cli("convert-gpx", data / "cptr004.gpx", tmpdir / "out.fit")
    mesgs = garmin_read_messages(tmpdir / "out.fit")
    distance = mesgs["record_mesgs"][-1]["distance"] * ureg.meter
    assert distance.to(ureg.mile).magnitude == approx(4.48, abs=0.01)
