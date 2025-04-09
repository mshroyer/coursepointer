import argparse
from typing import List, NamedTuple
import xml.sax


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


class GpxTrackContentHandler(xml.sax.ContentHandler):
    def __init__(self):
        super().__init__()
        self.track_points : List[Coordinate] = []
        self.waypoints : List[Waypoint] = []
        self.next_wpt_lat : float = 0
        self.next_wpt_lon : float = 0
        self.in_wpt : bool = False
        self.in_wpt_name : bool = False

    def startElement(self, name, attrs):
        if name == "trkpt":
            lat = float(attrs["lat"])
            lon = float(attrs["lon"])
            self.track_points.append(Coordinate(lat, lon))
        elif name == "wpt":
            self.next_wpt_lat = float(attrs["lat"])
            self.next_wpt_lon = float(attrs["lon"])
            self.in_wpt = True
        elif name == "name" and self.in_wpt:
            self.in_wpt_name = True

    def characters(self, content):
        if self.in_wpt_name:
            self.waypoints.append(Waypoint(name=content, coord=Coordinate(self.next_wpt_lat, self.next_wpt_lon)))
            self.in_wpt_name = False

    def endElement(self, name):
        if name == "wpt":
            self.in_wpt = False


class GpxTrackFile:
    def __init__(self, path: str):
        self.path = path
        self._content_handler = GpxTrackContentHandler()
        self._parsed = False

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


def main() -> None:
    parser = argparse.ArgumentParser(description="Extract track points from a GPX file")
    parser.add_argument("path", help="Path to the GPX track file")
    args = parser.parse_args()

    track_file = GpxTrackFile(args.path)
    for trkpt in track_file.track_points():
        print(f"Track Point: ({trkpt.lat}, {trkpt.lon})")
    for wpt in track_file.waypoints():
        print(f"Waypoint: {wpt.name} at ({wpt.coord.lat}, {wpt.coord.lon})")


if __name__ == "__main__":
    main()
