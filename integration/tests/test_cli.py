"""Test the main coursepointer-cli binary"""

from datetime import datetime, timezone
import subprocess
from itertools import pairwise

from pytest import approx, raises
import shutil

from integration import field, garmin_read_file_header


class TestUI:
    """Test the CLI's user interface"""

    def test_help(self, coursepointer_cli):
        assert "Print help" in coursepointer_cli("--help").stdout

    def test_no_subcommand(self, coursepointer_cli):
        with raises(subprocess.CalledProcessError) as einfo:
            coursepointer_cli()

        assert "Usage:" in einfo.value.stderr

    def test_missing_input(self, tmpdir, coursepointer_cli):
        with raises(subprocess.CalledProcessError) as einfo:
            coursepointer_cli(
                "convert", tmpdir / "nonexistent.gpx", "-o", tmpdir / "out.fit"
            )

        assert "Opening the GPX <INPUT> file" in einfo.value.stderr

    def test_output_file_exists(self, tmpdir, data, coursepointer_cli):
        with open(tmpdir / "out.fit", "w") as f:
            print("Hello", file=f)

        with raises(subprocess.CalledProcessError) as einfo:
            coursepointer_cli("convert", data / "cptr002.gpx", "-o", tmpdir / "out.fit")

        # Error message can vary slightly by platform
        assert "file exists" in einfo.value.stderr.lower()

    def test_output_file_force(self, tmpdir, data, coursepointer_cli):
        with open(tmpdir / "out.fit", "w") as f:
            print("Hello", file=f)

        coursepointer_cli(
            "convert", data / "cptr002.gpx", "-o", tmpdir / "out.fit", "--force"
        )

    def test_default_output_path(self, tmpdir, data, coursepointer_cli):
        shutil.copyfile(data / "cptr002.gpx", tmpdir / "cptr002.gpx")
        coursepointer_cli("convert", tmpdir / "cptr002.gpx")

        assert (tmpdir / "cptr002.fit").exists()

    def test_speed_arg(self, tmpdir, data, ureg, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr003.gpx", "--speed", "30.0")
        out = caching_mesgs(out_file)

        distance = field(out, "lap", 0, "total_distance") * ureg.meter

        # The elapsed and timer times should be set to the same value.
        elapsed = field(out, "lap", 0, "total_elapsed_time") * ureg.second

        # The total distance should be approximately equal to the speed
        # specified to the CLI times the recorded lap time.
        speed = 30 * ureg.kilometer / ureg.hour
        assert distance.magnitude == approx(
            (elapsed * speed).to(ureg.meter).magnitude, rel=0.0001
        )

    def test_sport_arg(self, tmpdir, data, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr004.gpx", "--sport", "hiking")
        out = caching_mesgs(out_file)

        assert field(out, "course", 0, "sport") == "hiking"

    def test_no_courses(self, tmpdir, data, coursepointer_cli):
        with raises(subprocess.CalledProcessError) as einfo:
            coursepointer_cli(
                "convert", data / "invalid_empty.gpx", "-o", tmpdir / "out.fit"
            )

        assert "No course was found" in einfo.value.stderr

    def test_multiple_routes(self, tmpdir, data, coursepointer_cli):
        with raises(subprocess.CalledProcessError) as einfo:
            coursepointer_cli("convert", data / "cptr007.gpx", "-o", tmpdir / "out.fit")

        assert "Unexpected number of courses" in einfo.value.stderr

    def test_bad_xml(self, tmpdir, data, coursepointer_cli):
        with raises(subprocess.CalledProcessError) as einfo:
            coursepointer_cli(
                "convert", data / "invalid_bad_xml.gpx", "-o", tmpdir / "out.fit"
            )

        assert "<INPUT> is not a valid GPX file" in einfo.value.stderr

    def test_negative_threshold(self, tmpdir, data, coursepointer_cli):
        with raises(subprocess.CalledProcessError) as einfo:
            coursepointer_cli(
                "convert",
                data / "cptr002.gpx",
                "-o",
                tmpdir / "out.fit",
                "--threshold=-30",
            )

        assert "negative" in einfo.value.stderr

    def test_low_speed(self, tmpdir, data, coursepointer_cli):
        with raises(subprocess.CalledProcessError) as einfo:
            coursepointer_cli(
                "convert", data / "cptr002.gpx", "-o", tmpdir / "out.fit", "--speed=0"
            )

        assert "too low" in einfo.value.stderr


class TestFIT:
    """Test FIT encoding in the CLI.

    Tests that low-level details of FIT encoding and course file are correct.
    The exact GPX used as the input to the conversion isn't relevant to these
    tests.

    """

    def test_header_protocol_verison(self, data, caching_convert):
        out_file = caching_convert(data / "cptr004.gpx")
        header = garmin_read_file_header(out_file)

        # Protocol version 1 is represented as 0x10, 2 as 0x20.
        assert header.protocol_version == 0x10

    def test_header_profile_version(self, data, integration_stub, caching_convert):
        out_file = caching_convert(data / "cptr004.gpx")
        header = garmin_read_file_header(out_file)

        # The output file should encode the same profile version.
        lib_profile_version = int(
            integration_stub("show-profile-version").stdout.strip()
        )
        assert header.profile_version == lib_profile_version

    def test_course_sport(self, data, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr004.gpx")
        mesgs = caching_mesgs(out_file)

        assert len(mesgs["course_mesgs"]) == 1
        assert field(mesgs, "course", 0, "sport") == "cycling"

    def test_file_id_type(self, data, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr004.gpx")
        mesgs = caching_mesgs(out_file)

        assert len(mesgs["file_id_mesgs"]) == 1
        assert field(mesgs, "file_id", 0, "type") == "course"

    def test_file_id_manufacturer(self, data, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr004.gpx")
        mesgs = caching_mesgs(out_file)

        assert field(mesgs, "file_id", 0, "manufacturer") == "development"

    def test_file_id_time_created(self, data, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr004.gpx")
        mesgs = caching_mesgs(out_file)

        time_created = field(mesgs, "file_id", 0, "time_created")
        assert datetime(2019, 11, 23, 00, 00, 00, tzinfo=timezone.utc) == time_created

    def test_file_id_product_name(self, data, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr004.gpx")
        mesgs = caching_mesgs(out_file)

        assert field(mesgs, "file_id", 0, "product_name") == "CoursePointer"

    def test_file_creator_versions(self, data, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr004.gpx")
        mesgs = caching_mesgs(out_file)

        assert len(mesgs["file_creator_mesgs"]) == 1
        assert field(mesgs, "file_creator", 0, "software_version") == 42
        assert field(mesgs, "file_creator", 0, "hardware_version") == 1


class TestConvert:
    """Tests that GPX routes and tracks are converted faithfully"""

    def test_course_name(self, data, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr002.gpx")
        mesgs = caching_mesgs(out_file)

        # The file should have a single course message containing the same track
        # name given in the GPX input.
        course_mesgs = mesgs["course_mesgs"]
        assert len(course_mesgs) == 1
        assert course_mesgs[0]["name"] == "cptr002"

    def test_lap_distance(self, data, ureg, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr003.gpx")
        out = caching_mesgs(out_file)

        assert len(out["lap_mesgs"]) == 1

        # We'll compare the output to the results of importing the GPX into
        # Garmin Connect and then re-exporting it as FIT, because this puts the
        # code under test on an equal footing given the limited precision of
        # RWGPS's GPX exports.
        ref = caching_mesgs(data / "cptr003_connect.fit")

        # The total distance should be approximately equal to that in the FIT
        # exported from Connect.
        out_distance = field(out, "lap", 0, "total_distance") * ureg.meter
        ref_distance = field(ref, "lap", 0, "total_distance") * ureg.meter
        assert out_distance.magnitude == approx(ref_distance.magnitude)

    def test_lap_timer(self, data, ureg, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr003.gpx")
        out = caching_mesgs(out_file)

        distance = field(out, "lap", 0, "total_distance") * ureg.meter

        # The elapsed and timer times should be set to the same value.
        elapsed = field(out, "lap", 0, "total_elapsed_time") * ureg.second
        timer = field(out, "lap", 0, "total_timer_time") * ureg.second
        assert elapsed == timer

        # The total distance should be approximately equal to the speed
        # specified to the CLI times the recorded lap time.
        speed = 5 * ureg.kilometer / ureg.hour
        assert distance.magnitude == approx(
            (timer * speed).to(ureg.meter).magnitude, rel=0.0001
        )

    def test_record_distances(self, data, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr003.gpx")
        mesgs = caching_mesgs(out_file)

        assert field(mesgs, "record", 0, "distance") == 0

        # Distances should be cumulative
        for a, b in pairwise(mesgs["record_mesgs"]):
            assert a["distance"] <= b["distance"]

        assert len(mesgs["lap_mesgs"]) == 1

        # The final record's distance should be equal to the course file's lap
        # distance
        assert field(mesgs, "record", -1, "distance") == field(
            mesgs, "lap", 0, "total_distance"
        )

    def test_record_timestamps(self, data, ureg, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr003.gpx")
        mesgs = caching_mesgs(out_file)

        speed = 5 * ureg.kilometer / ureg.hour
        record_mesgs = mesgs["record_mesgs"]

        start_timestamp = record_mesgs[0]["timestamp"]
        for record in record_mesgs:
            expected_duration = (record["distance"] * ureg.meter / speed).to(
                ureg.second
            )
            actual_duration = record["timestamp"] - start_timestamp
            assert actual_duration.seconds == approx(expected_duration.magnitude, abs=2)

    def test_timer_event_spacing(self, data, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr003.gpx")
        mesgs = caching_mesgs(out_file)

        event_mesgs = mesgs["event_mesgs"]
        assert len(event_mesgs) == 2
        assert event_mesgs[0]["event"] == "timer"
        assert event_mesgs[0]["event_type"] == "start"
        assert event_mesgs[1]["event"] == "timer"
        assert event_mesgs[1]["event_type"] == "stop"

        lap_mesgs = mesgs["lap_mesgs"]
        lap_elapsed = lap_mesgs[0]["total_timer_time"]

        event_spacing = event_mesgs[1]["timestamp"] - event_mesgs[0]["timestamp"]
        # Comparison is approximate because event timestamps have one-second
        # resolution, while lap time has millisecond resolution.
        assert event_spacing.seconds == approx(lap_elapsed, abs=2)

    def test_timer_event_group(self, data, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr003.gpx")
        mesgs = caching_mesgs(out_file)

        # I don't know if this is needed for anything, but Garmin Connect sets a
        # zero event group so we might as well do the same.
        event_mesgs = mesgs["event_mesgs"]
        assert event_mesgs[0]["event_group"] == event_mesgs[1]["event_group"] == 0

    def test_gpx_rte_conversion(self, data, ureg, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr004.gpx")
        mesgs = caching_mesgs(out_file)

        distance = field(mesgs, "record", -1, "distance") * ureg.meter
        assert distance.to(ureg.mile).magnitude == approx(4.48, abs=0.01)


class TestIntercept:
    """Tests calculation of waypoint intercepts as course points"""

    def test_waypoint_interception(self, data, ureg, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr004.gpx")
        mesgs = caching_mesgs(out_file)

        # This route should have identified four course points
        assert len(mesgs["course_point_mesgs"]) == 4

        course_length = field(mesgs, "lap", 0, "total_distance") * ureg.meter
        for course_point in mesgs["course_point_mesgs"]:
            distance = course_point["distance"] * ureg.meter
            assert distance > 0 * ureg.meter
            assert distance <= course_length

        names = list(map(lambda mesg: mesg["name"], mesgs["course_point_mesgs"]))
        assert names[0] == "Castle Rock Fal"
        assert names[1] == "Russell Point"
        assert names[2] == "Emily Smith Poi"
        assert names[3] == "Goat Point Over"

    def test_threshold_arg(self, data, ureg, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr004.gpx", "--strategy", "first")
        mesgs = caching_mesgs(out_file)

        russell_point = mesgs["course_point_mesgs"][1]
        assert russell_point["name"] == "Russell Point"

        expected_meters = (2.04 * ureg.mile).to(ureg.meter)
        assert russell_point["distance"] == approx(expected_meters.magnitude, rel=0.01)

        # If we set the threshold argument to < 35m, the first interception with
        # Russell Point won't be until we come around the loop on the hike,
        # putting the distance at around 2.55 miles instead of 2.04 miles.
        out_file2 = caching_convert(
            data / "cptr004.gpx", "--strategy", "first", "--threshold", "20"
        )
        mesgs2 = caching_mesgs(out_file2)

        russell_point2 = mesgs2["course_point_mesgs"][1]
        assert russell_point2["name"] == "Russell Point"

        expected_meters2 = (2.55 * ureg.mile).to(ureg.meter)
        assert russell_point2["distance"] == approx(
            expected_meters2.magnitude, rel=0.01
        )

    def test_default_nearest_strategy(self, data, ureg, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr004.gpx")
        mesgs = caching_mesgs(out_file)

        russell_point = mesgs["course_point_mesgs"][1]
        assert russell_point["name"] == "Russell Point"

        # Even though we pass within 35m of Russell Point at around 2.04 miles,
        # we pass much nearer at 2.55 miles. With the default intercept strategy
        # set to "nearest" we should choose the latter interception by default.
        expected_meters = (2.55 * ureg.mile).to(ureg.meter)
        assert russell_point["distance"] == approx(expected_meters.magnitude, rel=0.01)

    def test_all_strategy(self, data, ureg, caching_convert, caching_mesgs):
        out_file = caching_convert(data / "cptr004.gpx", "--strategy", "all")
        mesgs = caching_mesgs(out_file)

        # With strategy "all" we intercept Russell Point twice within 35m:
        assert len(mesgs["course_point_mesgs"]) == 5

        russell_point1 = mesgs["course_point_mesgs"][1]
        assert russell_point1["name"] == "Russell Point"
        assert russell_point1["distance"] == approx(
            (2.04 * ureg.mile).to(ureg.meter).magnitude, rel=0.01
        )

        russell_point2 = mesgs["course_point_mesgs"][2]
        assert russell_point2["name"] == "Russell Point"
        assert russell_point2["distance"] == approx(
            (2.55 * ureg.mile).to(ureg.meter).magnitude, rel=0.01
        )
