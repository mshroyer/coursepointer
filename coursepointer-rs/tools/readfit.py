import argparse

import fitdecode


def get_semicircle_degrees(semicircles: int) -> float:
    """Convert semicircle degrees to decimal degrees."""
    return semicircles * 180 / 2**31

def show_frames(path: str) -> None:
    with fitdecode.FitReader(path) as reader:
        for frame in reader:
            if frame.frame_type == fitdecode.FIT_FRAME_DATA:
                if frame.name == "record":
                    lat = get_semicircle_degrees(frame.get_field("position_lat").value)
                    lon = get_semicircle_degrees(frame.get_field("position_long").value)
                    dist = get_semicircle_degrees(frame.get_field("distance").value)
                    altitude = get_semicircle_degrees(frame.get_field("altitude").value)
                    print(f"Record message: lat={lat}, lon={lon}, dist={dist}, alt={altitude}")
                elif frame.name == "course_point":
                    lat = get_semicircle_degrees(frame.get_field("position_lat").value)
                    lon = get_semicircle_degrees(frame.get_field("position_long").value)
                    dist = get_semicircle_degrees(frame.get_field("distance").value)
                    print(f"Course point: lat={lat}, lon={lon}, dist={dist}")
                else:
                    print(f"Data message: {frame.name}")
            elif frame.frame_type == fitdecode.FIT_FRAME_DEFINITION:
                print(f"=== Definition frame: {frame.name} ===")
                for field_def in frame.field_defs:
                    print(f"- Field def: {field_def.name} = {field_def.type.name}")
                print("=== End definition frame ===")
            else:
                print(frame)

def main() -> None:
    parser = argparse.ArgumentParser(description="Read and analyze FIT files")
    subparsers = parser.add_subparsers(dest="subparser_name", help="Subcommands")

    parser_show_frames = subparsers.add_parser("show-frames", help="Show a summary of frame info")
    parser_show_frames.add_argument("path", type=str, help="Path to FIT file")

    args = parser.parse_args()

    if args.subparser_name == "show-frames":
        show_frames(args.path)


if __name__ == "__main__":
    main()
