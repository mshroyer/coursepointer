"""Read FIT files and show their contents."""

import argparse

import fitdecode
import garmin_fit_sdk


def get_semicircle_degrees(semicircles: int) -> float:
    """Convert semicircle degrees to decimal degrees."""
    return semicircles * 180 / 2**31


def show_frames(path: str) -> None:
    with fitdecode.FitReader(
        path, processor=fitdecode.StandardUnitsDataProcessor(), keep_raw_chunks=True
    ) as reader:
        for frame in reader:
            if frame.frame_type == fitdecode.FIT_FRAME_DATA:
                if frame.name == "record":
                    lat = get_semicircle_degrees(frame.get_field("position_lat").value)
                    lon = get_semicircle_degrees(frame.get_field("position_long").value)
                    dist = get_semicircle_degrees(frame.get_field("distance").value)
                    if frame.has_field("altitude"):
                        altitude = get_semicircle_degrees(
                            frame.get_field("altitude").value
                        )
                        print(
                            f"Record message: lat={lat}, lon={lon}, dist={dist}, alt={altitude}"
                        )
                    else:
                        print(f"Record message: lat={lat}, lon={lon}, dist={dist}")
                elif frame.name == "course_point":
                    lat = get_semicircle_degrees(frame.get_field("position_lat").value)
                    lon = get_semicircle_degrees(frame.get_field("position_long").value)
                    dist = get_semicircle_degrees(frame.get_field("distance").value)
                    print(f"Course point: lat={lat}, lon={lon}, dist={dist}")
                elif frame.name == "event":
                    event = frame.get_field("event").value
                    event_group = frame.get_field("event_group").value
                    event_type = frame.get_field("event_type").value
                    print(f"Event: {event} type={event_type}, group={event_group}")
                else:
                    print(f"Data message: {frame.name}")
            elif frame.frame_type == fitdecode.FIT_FRAME_DEFINITION:
                print(
                    f"=== Definition frame: {frame.name} local num: {frame.local_mesg_num} endianness: {frame.endian} offset: 0x{frame.chunk.offset:08x} ==="
                )
                for field_def in frame.field_defs:
                    print(
                        f"- Field def: {field_def.name} ({field_def.def_num}) = {field_def.type.name} ({field_def.base_type.identifier}) size {field_def.size}"
                    )
                print("=== End definition frame ===")
            else:
                print(frame)


def show_messages(path: str) -> None:
    stream = garmin_fit_sdk.Stream.from_file(path)
    decoder = garmin_fit_sdk.Decoder(stream)
    messages, errors = decoder.read()
    if errors:
        raise ValueError(f"Errors reading FIT file: {errors}")

    print(f"Messages: {list(messages.keys())}")
    for event_mesg in messages["event_mesgs"]:
        print(event_mesg)
    if "course_point_mesgs" in messages:
        for course_point_mesg in messages["course_point_mesgs"]:
            print(course_point_mesg)
    for record_mesg in messages["record_mesgs"]:
        print(record_mesg)


def show_definitions(path: str) -> None:
    with fitdecode.FitReader(path) as reader:
        for frame in reader:
            if frame.frame_type == fitdecode.FIT_FRAME_DEFINITION:
                print(f"=== Definition frame: {frame.name} ===")
                for field_def in frame.field_defs:
                    print(f"- Field def: {field_def.name} = {field_def.type.name}")
                print("=== End definition frame ===")


def main() -> None:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    subparsers = parser.add_subparsers(dest="subparser_name", help="Subcommands")

    parser_show_frames = subparsers.add_parser(
        "show-frames", help="Show a summary of frame info"
    )
    parser_show_frames.add_argument("path", type=str, help="Path to FIT file")

    parser_show_messages = subparsers.add_parser(
        "show-messages", help="Show messages as extracted by the Garmin SDK"
    )
    parser_show_messages.add_argument("path", type=str, help="Path to FIT file")

    args = parser.parse_args()

    if args.subparser_name == "show-frames":
        show_frames(args.path)
    elif args.subparser_name == "show-messages":
        show_messages(args.path)


if __name__ == "__main__":
    main()
