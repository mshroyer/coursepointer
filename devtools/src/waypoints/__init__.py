"""Create a table from waypoint metadata

Given a GPX file containing waypoints, this script produces a table of the
waypoints' names and their `sym`, `type`, and `cmt` element contents.  This is
meant to be used to convert exports of sample POI or Waypoint types from apps
like Ride with GPS or Gaia GPS into a Markdown table illustrating how to
interpret them.

"""

from collections import namedtuple
from pathlib import Path
from typing import Iterator, List

import argparse

from defusedxml import ElementTree

PointType = namedtuple("PointType", ["name", "sym", "cmt", "type_"])


def get_point_types(input_path: Path) -> Iterator[PointType]:
    tree = ElementTree.parse(input_path).getroot()
    for wpt in tree.findall("{http://www.topografix.com/GPX/1/1}wpt"):
        name = wpt.find("{http://www.topografix.com/GPX/1/1}name")
        sym = wpt.find("{http://www.topografix.com/GPX/1/1}sym")
        cmt = wpt.find("{http://www.topografix.com/GPX/1/1}cmt")
        type_ = wpt.find("{http://www.topografix.com/GPX/1/1}type")
        yield PointType(
            name=name.text,
            sym=sym.text if sym is not None else "",
            cmt=cmt.text if cmt is not None else "",
            type_=type_.text if type_ is not None else "",
        )


def print_row(items: List[str], lens: List[int], buffer=" "):
    assert len(items) == len(lens)
    for i in range(len(items)):
        print("|", end="")
        print(buffer, end="")
        print(items[i].ljust(lens[i], " "), end="")
        print(buffer, end="")

    print("|")


def print_separator(lens: List[int]):
    items = list(map(lambda len: "-" * len, lens))
    print_row(items, lens, buffer="-")


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "input", type=Path, help="Path to GPX file containing waypoints"
    )
    args = parser.parse_args()

    point_types = list(get_point_types(args.input))
    point_types.sort(key=lambda pt: pt.name if pt.name != "pin" else "")

    name_len = len("name")
    sym_len = len("sym")
    cmt_len = len("cmt")
    type_len = len("type")
    for point_type in point_types:
        name_len = max(name_len, len(point_type.name))
        sym_len = max(sym_len, len(point_type.sym))
        cmt_len = max(cmt_len, len(point_type.cmt))
        type_len = max(type_len, len(point_type.type_))

    lens = [name_len, sym_len, cmt_len, type_len]

    print_row(["name", "sym", "cmt", "type"], lens)
    print_separator(lens)
    for point_type in point_types:
        print_row(
            [point_type.name, point_type.sym, point_type.cmt, point_type.type_], lens
        )


if __name__ == "__main__":
    main()
