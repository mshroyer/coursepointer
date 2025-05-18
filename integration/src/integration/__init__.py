from datetime import datetime, timezone
import json
from pathlib import Path
from typing import List, Optional, Tuple

from pytest import approx
import garmin_fit_sdk


def rfc9557_utc(ts: datetime) -> str:
    """Formats an RFC9557 timestamp in UTC

    Formats the timestamp as RFC9557 in UTC, using the 'Z' suffix to indicate
    an unspecified local time offset.

    """
    return ts.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


class SurfacePoint:
    def __init__(self, lat: float, lon: float):
        self.lat = lat
        self.lon = lon

    def to_dict(self) -> dict:
        return {"lat": self.lat, "lon": self.lon}


class CourseSpec:
    """Specification of a course for integration-stub

    Serializes to a JSON file, which integration-stub uses as input.

    """

    name: str
    start_time: datetime
    records: List[SurfacePoint]

    def __init__(self, name: str = "", start_time: datetime = datetime.now(timezone.utc),
                 records: Optional[List[Tuple[float, float]]] = None) -> None:
        self.name = name
        self.start_time = start_time

        self.records = []
        if records:
            for record in records:
                self.records.append(SurfacePoint(*record))

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "start_time": rfc9557_utc(self.start_time),
            "records": list(map(lambda r: r.to_dict(), self.records)),
        }

    def write_file(self, path: Path) -> None:
        with open(path, "w") as f:
            json.dump(self.to_dict(), f)


def garmin_sdk_read_fit(path: Path) -> dict:
    """Read messages from the FIT file using the Garmin SDK"""

    stream = garmin_fit_sdk.Stream.from_file(path)
    decoder = garmin_fit_sdk.Decoder(stream)
    messages, errors = decoder.read()
    if errors:
        raise ValueError(f"Errors reading FIT file: {errors}")

    return messages


def garmin_sdk_record_coords(record: dict) -> Tuple[float, float]:
    """Get coordinate tuple for a record message

    Returns a (lat, lon) tuple in decimal degrees for the given FIT record
    message, as returned by the Garmin SDK.

    """
    lat = 180 * record["position_lat"] / 2 ** 31
    lon = 180 * record["position_long"] / 2 ** 31
    return lat, lon


def assert_coords_approx_eq(a: List[Tuple[float, float]], b: List[Tuple[float, float]]) -> None:
    assert len(a) == len(b)
    for i in range(len(a)):
        # Test for approximate equality with an absolute tolerance of two
        # Garmin semicircles.
        assert a[i] == approx(b[i], rel=0.0, abs=(180.0 / (2 ** 30)))
