import json
from pathlib import Path
from typing import List, Tuple

import garmin_fit_sdk


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

    records: List[SurfacePoint]

    def __init__(self, records: List[Tuple[float, float]]) -> None:
        self.records = []
        for record in records:
            self.records.append(SurfacePoint(*record))

    def to_dict(self) -> dict:
        return {"records": list(map(lambda r: r.to_dict(), self.records))}

    def write_file(self, path: Path) -> None:
        with open(path, "w") as f:
            json.dump(self.to_dict(), f)


def validate_fit_file(path: Path) -> None:
    stream = garmin_fit_sdk.Stream.from_file(path)
    decoder = garmin_fit_sdk.Decoder(stream)
    messages, errors = decoder.read()
    if errors:
        raise ValueError(f"Errors reading FIT file: {errors}")
