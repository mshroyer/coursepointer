import argparse
from dataclasses import dataclass
from typing import Generator


@dataclass
class Coordinate:
    """A coordinate on the WGS84 ellipsoid."""
    lat: float
    lon: float


@dataclass
class Waypoint:
    """A waypoint on the WGS84 ellipsoid."""
    name: str
    coord: Coordinate


class GpxTrackFile:
    """A GPX track file."""

    def __init__(self, path: str):
        self.path = path

    def track_points(self) -> Generator[Coordinate]:
        """Generate GPX trackpt sequence"""

        pass

    def waypoints(self) -> Generator[Waypoint]:
        pass


def main() -> None:
    parser = argparse.ArgumentParser(description="Extract track points from a GPX file")
    parser.add_argument("path", help="Path to the GPX track file")
    args = parser.parse_args()


if __name__ == "__main__":
    main()
