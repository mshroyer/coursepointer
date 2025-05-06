import fitdecode
import garmin_fit_sdk as fit


class FitCourse:
    def __init__(self, messages):
        self.messages = messages

    def to_json(self):
        return self.messages

    @classmethod
    def from_fit(klass, path: str) -> "FitCourse":
        with fitdecode.FitReader(path) as reader:
            for frame in reader:
                if frame.frame_type == fitdecode.FIT_FRAME_DATA:
                    print(f"Data message: {frame.name}")
                elif frame.frame_type == fitdecode.FIT_FRAME_DEFINITION:
                    print("=== Definition frame ===")
                    for field_def in frame.field_defs:
                        print(f"- Field def: {field_def.name} = {field_def.type.name}")
                    print("=== End definition frame ===")
                else:
                    print(frame)

        return klass({})

    @classmethod
    def from_fit_garmin(klass, path: str) -> "FitCourse":
        stream = fit.Stream.from_file(path)
        decoder = fit.Decoder(stream)
        if not decoder.check_integrity():
            raise ValueError("Not a valid FIT file")

        messages, errors = decoder.read(expand_sub_fields=False, expand_components=False, merge_heart_rates=False, mesg_listener=mesg_listener)
        if errors:
            raise ValueError(f"Errors reading FIT file: {errors}")

        print(f"Messages: {list(messages.keys())}")

        return klass(messages)


def mesg_listener(mesg_num, message):
    print((mesg_num, message))


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Examine course points in a FIT course")
    parser.add_argument("path")
    args = parser.parse_args()

    fit_course = FitCourse.from_fit(args.path)
    print(fit_course.to_json())


if __name__ == "__main__":
    main()