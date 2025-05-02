"""
Export RideWithGPS POIs as Course Points for Garmin devices

  1. Export a GPX track from RideWithGPS, including POIs as waypoints.
  2. Run: coursepointer make-fit --gpx <gpx_file> --out <fit_file>
  3. Import the FIT file to your Garmin device.

For more info see https://github.com/mshroyer/coursepointer
"""

import argparse
from datetime import datetime, timedelta
import itertools
from typing import List, NamedTuple, Optional
import xml.sax

from fit_tool.fit_file_builder import FitFileBuilder
from fit_tool.profile.messages.course_message import CourseMessage
from fit_tool.profile.messages.event_message import EventMessage
from fit_tool.profile.messages.file_id_message import FileIdMessage
from fit_tool.profile.messages.lap_message import LapMessage
from fit_tool.profile.messages.record_message import RecordMessage
from fit_tool.profile.profile_type import FileType, Manufacturer, Sport, Event, EventType
import geographiclib.geodesic


# Distance calculations in FIT and GPX files are based on WGS84.
GEODESIC = geographiclib.geodesic.Geodesic.WGS84


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
        self.course_name : str = ""
        self._next_wpt_coord : Optional[Coordinate] = None
        self._next_wpt_name : str = ""
        self._in_wpt_name : bool = False
        self._element_stack : List[str] = []

    @staticmethod
    def _make_coord(attrs) -> Coordinate:
        return Coordinate(float(attrs['lat']), float(attrs['lon']))

    def startElement(self, name, attrs):
        self._element_stack.append(name)

        if name == "trkpt":
            self.track_points.append(self._make_coord(attrs))
        elif name == "wpt":
            self._next_wpt_coord = self._make_coord(attrs)

    def characters(self, content):
        if self._element_stack[-2:] == ["wpt", "name"]:
            self._next_wpt_name = content
        elif self._element_stack[-2:] == ["metadata", "name"]:
            self.course_name = content

    def endElement(self, name):
        if name == "wpt":
            self.waypoints.append(Waypoint(name=self._next_wpt_name, coord=self._next_wpt_coord))
            self._next_wpt_coord = None

        self._element_stack.pop()


class GpxTrackFile:
    def __init__(self, path: str):
        self.path : str = path
        self._content_handler = GpxTrackContentHandler()
        self._parsed : bool = False

    def course_name(self) -> str:
        self._parse()
        return self._content_handler.course_name

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
    name: str
    records: List[CourseRecord]

    def __init__(self, name: str, coords: List[Coordinate]):
        # TODO: Initialize with actual course name
        self.name = name
        self.records = self.compute_records(coords)

    @staticmethod
    def compute_records(coords: List[Coordinate]) -> List[CourseRecord]:
        distance = 0.0
        records = []

        if len(coords) > 0:
            records.append(CourseRecord(coords[0], distance))

        for start, end in itertools.pairwise(coords):
            g = GEODESIC.Inverse(start.lat, start.lon, end.lat, end.lon)
            distance += g["s12"]
            records.append(CourseRecord(end, distance))

        return records

    def export_fit(self, out_path: str) -> None:
        SPEED_KMPH = 10.0
        time = start_time = datetime.now()

        builder = FitFileBuilder(auto_define=True, min_string_size=50)

        message = FileIdMessage()
        message.type = FileType.COURSE
        message.manufacturer = Manufacturer.DEVELOPMENT.value
        message.product = 0
        message.timeCreated = time.timestamp()
        message.serialNumber = 0x12345678
        builder.add(message)

        # Every FIT course file MUST contain a Course message
        message = CourseMessage()
        message.courseName = self.name
        message.sport = Sport.CYCLING
        builder.add(message)

        # Timer Events are REQUIRED for FIT course files
        message = EventMessage()
        message.event = Event.TIMER
        message.event_type = EventType.START
        message.timestamp = 1000 * time.timestamp()
        builder.add(message)

        course_records = []  # track points
        for record in self.records:
            message = RecordMessage()
            message.position_lat = record.coord.lat
            message.position_long = record.coord.lon
            message.distance = record.dist_m
            time += timedelta(hours=(record.dist_m / (SPEED_KMPH * 1000)))
            message.timestamp = 1000 * time.timestamp()
            course_records.append(message)

        builder.add_all(course_records)

        # stop event
        message = EventMessage()
        message.event = Event.TIMER
        message.eventType = EventType.STOP_ALL
        message.timestamp = 1000 * time.timestamp()
        builder.add(message)

        # Every FIT course file MUST contain a Lap message
        elapsed_time = time - start_time
        message = LapMessage()
        message.timestamp = 1000 * time.timestamp()
        message.start_time = 1000 * start_time.timestamp()
        message.total_elapsed_time = elapsed_time.total_seconds()
        message.total_timer_time = elapsed_time.total_seconds()
        message.start_position_lat = course_records[0].position_lat
        message.start_position_long = course_records[0].position_long
        message.end_position_lat = course_records[-1].position_lat
        message.endPositionLong = course_records[-1].position_long
        message.total_distance = course_records[-1].distance

        # Finally build the FIT file object and write it to a file
        fit_file = builder.build()
        fit_file.to_file(out_path)


def make_fit(gpx_path: str, fit_path: str) -> None:
    """Converts a GPX track file to a FIT file"""

    track_file = GpxTrackFile(gpx_path)
    course = Course(track_file.course_name(), track_file.track_points())
    course.export_fit(fit_path)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    subparsers = parser.add_subparsers(dest="subparser_name", help="Subcommands")

    parser_make_fit = subparsers.add_parser("make-fit", help="Make a FIT file from a GPX track")
    parser_make_fit.add_argument("--gpx-track", required=True, help="Path to the GPX track file")
    parser_make_fit.add_argument("--out", required=True, help="Output path for the FIT file")

    parser_debug = subparsers.add_parser("info", help="Show GPX track info")
    parser_debug.add_argument("--gpx-track", required=True, help="Path to the GPX track file")

    args = parser.parse_args()

    if args.subparser_name == "make-fit":
        make_fit(args.gpx_track, args.out)
        return
    elif args.subparser_name == "info":
        track_file = GpxTrackFile(args.gpx_track)
        course = Course(track_file.course_name(), track_file.track_points())

        total_length_m = course.records[-1].dist_m if course.records else 0
        print(f"Course name: {course.name}")
        print(f"Loaded {len(course.records)} track points with a total length of {int(total_length_m)}m")
        print(f"Found {len(track_file.waypoints())} waypoints")

        # for trkpt in track_file.track_points():
        #     print(f"Track Point: ({trkpt.lat}, {trkpt.lon})")
        # for record in course.records:
        #     print(f"Course Record: {record}")


if __name__ == "__main__":
    main()
