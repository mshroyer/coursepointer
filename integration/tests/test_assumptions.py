"""When you assume you make a...

Checks assumptions about how Garmin FIT encoding *should* work by examining FIT
exports from Garmin Connect.

"""

from itertools import pairwise

from pytest import approx

from integration import (
    fitdecode_get_definition_frames,
    garmin_read_file_header,
)


def test_lap_messages(data, caching_mesgs):
    laps = caching_mesgs(data / "cptr003_connect.fit")["lap_mesgs"]
    assert len(laps) == 1


def test_protocol_version(data):
    header = garmin_read_file_header(data / "cptr003_connect.fit")
    assert header.protocol_version == 0x10


def test_endianness(data):
    # Garmin Connect exports big endian FIT files.
    for definition_frame in fitdecode_get_definition_frames(
        data / "cptr003_connect.fit"
    ):
        assert definition_frame.endian == ">"


def test_record_distances(data, caching_mesgs):
    mesgs = caching_mesgs(data / "cptr003_connect.fit")
    record_mesgs = mesgs["record_mesgs"]
    assert record_mesgs[0]["distance"] == 0

    # Distances should be cumulative
    for a, b in pairwise(record_mesgs):
        assert a["distance"] <= b["distance"]

    # The final record's distance should be equal to the course file's lap
    # distance
    lap_mesgs = mesgs["lap_mesgs"]
    assert len(lap_mesgs) == 1
    assert record_mesgs[-1]["distance"] == approx(lap_mesgs[0]["total_distance"])


def test_speed_scale(data, ureg, caching_mesgs):
    mesgs = caching_mesgs(data / "cptr003_connect.fit")
    record_speeds = list(map(lambda msg: msg["speed"], mesgs["record_mesgs"]))
    avg_speed = ((sum(record_speeds) / len(record_speeds)) * ureg("m/s")).to("km/h")

    # The record messages' speed fields should be returned by the SDK as
    # unscaled meters per second:
    assert avg_speed > 15 * ureg("km/h")
    assert avg_speed < 30 * ureg("km/h")
