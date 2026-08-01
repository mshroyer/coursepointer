from datetime import datetime, timezone
import json
from pathlib import Path
from subprocess import CalledProcessError
from typing import Any, Iterator, List, Optional, Tuple

from pytest import approx, fail
from defusedxml import ElementTree
import garmin_fit_sdk
import fitdecode


def rfc9557_utc(ts: datetime) -> str:
    """Formats an RFC9557 timestamp in UTC

    Formats the timestamp as RFC9557 in UTC, using the 'Z' suffix to indicate
    an unspecified local time offset.

    """
    return ts.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


class SurfacePoint:
    def __init__(self, lat: float, lon: float, ele: Optional[float] = None):
        self.lat = lat
        self.lon = lon
        self.ele = ele

    def to_dict(self) -> dict:
        return {"lat": self.lat, "lon": self.lon, "ele": self.ele}


class CourseSpec:
    """Specification of a course for integration-stub

    Serializes to a JSON file, which integration-stub uses as input.

    Records are given as (lat, lon) or (lat, lon, ele) tuples, the latter
    specifying an elevation in meters.

    """

    name: str
    start_time: datetime
    records: List[SurfacePoint]

    def __init__(
        self,
        name: str = "",
        start_time: datetime = datetime.now(timezone.utc),
        records: Optional[List[Tuple[float, ...]]] = None,
    ) -> None:
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


def garmin_read_file_header(path: Path):
    """Read the FIT file header using the Garmin SDK

    Returns a FileHeader object, whose class is only locally defined in the SDK.

    """

    stream = garmin_fit_sdk.Stream.from_file(path)
    decoder = garmin_fit_sdk.Decoder(stream)
    return decoder.read_file_header(True)


def garmin_read_messages(path: Path) -> dict[str, Any]:
    """Read messages from the FIT file using the Garmin SDK

    Raises a ValueError if the file is invalid.

    """

    stream = garmin_fit_sdk.Stream.from_file(path)
    decoder = garmin_fit_sdk.Decoder(stream)
    messages, errors = decoder.read()
    if errors:
        raise ValueError(f"Errors reading FIT file: {errors}")

    return messages


def field(mesgs: dict[str, Any], mesg_name: str, index: int, field_name: str) -> Any:
    """Shorthand for accessing a field in Garmin SDK messages"""
    return mesgs[mesg_name + "_mesgs"][index][field_name]


def semicircles_to_degrees(coords: Tuple[float, float]) -> Tuple[float, float]:
    lat = 180 * coords[0] / 2**31
    lon = 180 * coords[1] / 2**31
    return lat, lon


def garmin_sdk_record_coords(record: dict[str, Any]) -> Tuple[float, float]:
    """Get coordinate tuple for a record message

    Returns a (lat, lon) tuple in decimal degrees for the given FIT record
    message, as returned by the Garmin SDK.

    """
    return semicircles_to_degrees((record["position_lat"], record["position_long"]))


def gpx_route_points(path: Path) -> List[Tuple[float, float, Optional[float]]]:
    """Read a GPX file's track or route points

    Returns the file's trkpt and rtept elements in document order, as (lat, lon,
    ele) tuples, where the elevation is None if the point doesn't specify one.

    Consecutively repeated points are collapsed, the same way the crate skips
    the zero-length segments they would otherwise produce, so that the result
    lines up with the records of a converted course.

    """

    points = []
    for element in ElementTree.parse(path).getroot().iter():
        _, _, tag = element.tag.rpartition("}")
        if tag not in ("trkpt", "rtept"):
            continue

        ele = element.find("{*}ele")
        point = (
            float(element.get("lat")),
            float(element.get("lon")),
            None if ele is None else float(ele.text),
        )
        if not points or points[-1] != point:
            points.append(point)

    return points


def gpx_route_elevations(path: Path) -> List[Optional[float]]:
    """Read the elevations of a GPX file's track or route points"""
    return [point[2] for point in gpx_route_points(path)]


def fitdecode_get_definition_frames(
    path: Path,
) -> Iterator[fitdecode.records.FitDefinitionMessage]:
    with fitdecode.FitReader(path) as reader:
        for frame in reader:
            if frame.frame_type == fitdecode.FIT_FRAME_DEFINITION:
                yield frame


def fitdecode_record_field_names(path: Path) -> List[str]:
    """Get the names of the fields defined for a FIT file's record messages

    Fails if the file doesn't contain exactly one record definition message.

    """

    definitions = [
        frame
        for frame in fitdecode_get_definition_frames(path)
        if frame.name == "record"
    ]
    assert len(definitions) == 1
    return [field_def.name for field_def in definitions[0].field_defs]


def fitdecode_record_altitudes(path: Path) -> List[Optional[float]]:
    """Get the altitude of each of a FIT file's record messages

    Returns None for a record whose altitude is absent, or is set to the
    profile's invalid value.

    We read these with fitdecode rather than the Garmin SDK because the SDK's
    treatment of the invalid value varies by version: 21.178.0 decodes it as an
    altitude of 12607.0 meters instead of reporting the field as unset.

    """

    altitudes = []
    with fitdecode.FitReader(path) as reader:
        for frame in reader:
            if frame.frame_type != fitdecode.FIT_FRAME_DATA or frame.name != "record":
                continue
            altitudes.append(
                frame.get_value("altitude") if frame.has_field("altitude") else None
            )

    return altitudes


def assert_coords_approx_equal(
    left: Tuple[float, float], right: Tuple[float, float]
) -> None:
    # Test for approximate equality with an absolute tolerance of two
    # Garmin semicircles.
    assert left == approx(right, rel=0.0, abs=(180.0 / (2**30)))


def assert_all_coords_approx_equal(
    left: List[Tuple[float, float]], right: List[Tuple[float, float]]
) -> None:
    assert len(left) == len(right)
    for i in range(len(left)):
        assert_coords_approx_equal(left[i], right[i])


def fail_with_subprocess_error(e: CalledProcessError):
    lines = [f"Command {e.cmd!r} exited with return code {e.returncode}"]

    if getattr(e, "stdout", None):
        out = (
            e.stdout if isinstance(e.stdout, str) else e.stdout.decode(errors="ignore")
        )
        if out:
            lines.append("=== STDOUT ===")
            lines.append(out.rstrip())

    if getattr(e, "stderr", None):
        err = (
            e.stderr if isinstance(e.stderr, str) else e.stderr.decode(errors="ignore")
        )
        if err:
            lines.append("=== STDERR ===")
            lines.append(err.rstrip())

    msg = "\n".join(lines)

    # Suppress the default Python stack trace output
    fail(msg, pytrace=False)
