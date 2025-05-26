"""When you assume you make a...

Checks assumptions about how Garmin FIT encoding *should* work by examining FIT
exports from Garmin Connect.

"""

from itertools import pairwise

from pytest import approx

from integration import fitdecode_get_definition_frames, garmin_sdk_read_fit_messages, garmin_sdk_get_lap_distance_meters, garmin_sdk_read_fit_header
from integration.fixtures import data


def test_lap_messages(data):
    laps = garmin_sdk_read_fit_messages(data / "cptr003_connect.fit")["lap_mesgs"]
    assert len(laps) == 1


def test_protocol_version(data):
    header = garmin_sdk_read_fit_header(data / "cptr003_connect.fit")
    assert header.protocol_version == 0x10


def test_endianness(data):
    # Garmin Connect exports big endian FIT files.
    for definition_frame in fitdecode_get_definition_frames(data / "cptr003_connect.fit"):
        assert definition_frame.endian == ">"


def test_record_distances(data):
    records = garmin_sdk_read_fit_messages(data / "cptr003_connect.fit")["record_mesgs"]
    assert records[0]["distance"] == 0

    # Distances should be cumulative
    for a, b in pairwise(records):
        assert a["distance"] <= b["distance"]

    # The final record's distance should be equal to the course file's lap
    # distance
    assert records[-1]["distance"] == approx(garmin_sdk_get_lap_distance_meters(data / "cptr003_connect.fit"))
