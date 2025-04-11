"""
Export RideWithGPS POIs as Course Points for Garmin devices

  1. Export a GPX track from RideWithGPS, including POIs as waypoints.
  2. Run: coursepointer make-fit --gpx <gpx_file> --out <fit_file>
  3. Import the FIT file to your Garmin device.

For more info see https://github.com/mshroyer/coursepointer
"""

import argparse
import itertools
from typing import List, NamedTuple, Optional
import xml.sax

from geographiclib.geodesic import Geodesic


class Coordinate(NamedTuple):
    """A coordinate on the WGS84 ellipsoid."""
    lat: float
    lon: float


class Waypoint(NamedTuple):
    """A waypoint from a GPX file.

    Represents a point of interest from the source route.
    """
    name: str
    coord: Coordinate


# TODO: Validate document type
# TODO: Implement error callback
class GpxTrackContentHandler(xml.sax.ContentHandler):
    def __init__(self):
        super().__init__()
        self.track_points : List[Coordinate] = []
        self.waypoints : List[Waypoint] = []
        self._next_wpt_coord : Optional[Coordinate] = None
        self._next_wpt_name : str = ""
        self._in_wpt_name : bool = False

    @staticmethod
    def _make_coord(attrs) -> Coordinate:
        return Coordinate(float(attrs['lat']), float(attrs['lon']))

    def startElement(self, name, attrs):
        if name == "trkpt":
            self.track_points.append(self._make_coord(attrs))
        elif name == "wpt":
            self._next_wpt_coord = self._make_coord(attrs)
        elif name == "name" and self._next_wpt_coord is not None:
            self._in_wpt_name = True

    def characters(self, content):
        if self._in_wpt_name:
            self._next_wpt_name = content

    def endElement(self, name):
        if name == "wpt":
            self.waypoints.append(Waypoint(name=self._next_wpt_name, coord=self._next_wpt_coord))
            self._next_wpt_coord = None
        elif name == "name" and self._in_wpt_name:
            self._in_wpt_name = False


class GpxTrackFile:
    def __init__(self, path: str):
        self.path : str = path
        self._content_handler = GpxTrackContentHandler()
        self._parsed : bool = False

    def track_points(self) -> List[Coordinate]:
        self._parse()
        return self._content_handler.track_points

    def waypoints(self) -> List[Waypoint]:
        self._parse()
        return self._content_handler.waypoints

    def _parse(self):
        if not self._parsed:
            parser = xml.sax.make_parser()
            parser.setContentHandler(self._content_handler)
            parser.parse(self.path)
            self._parsed = True


class CourseRecord(NamedTuple):
    """A course record as in a FIT file.

    Represents a point along a course, along with its cumulative distance from the start of the course along the WGS84
    ellipsoid.
    """
    coord: Coordinate
    dist_m: float


class Course:
    records: List[CourseRecord]

    def __init__(self, coords: List[Coordinate]):
        self.records = self.compute_records(coords)

    @staticmethod
    def compute_records(coords: List[Coordinate]) -> List[CourseRecord]:
        distance = 0.0
        records = []

        if len(coords) > 0:
            records.append(CourseRecord(coords[0], distance))

        for start, end in itertools.pairwise(coords):
            g = Geodesic.WGS84.Inverse(start.lat, start.lon, end.lat, end.lon)
            distance += g['s12']
            records.append(CourseRecord(end, distance))

        return records


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("path", help="Path to the GPX track file")
    args = parser.parse_args()

    track_file = GpxTrackFile(args.path)
    for trkpt in track_file.track_points():
        print(f"Track Point: ({trkpt.lat}, {trkpt.lon})")

    course = Course(track_file.track_points())
    for record in course.records:
        print(f"Course Record: {record}")


if __name__ == "__main__":
    main()
