import argparse

import garmin_fit_sdk as fit


def main():
    parser = argparse.ArgumentParser(description="Examine course points in a FIT course")
    parser.add_argument("path")
    args = parser.parse_args()

    stream = fit.Stream.from_file(args.path)
    decoder = fit.Decoder(stream)
    messages, errors = decoder.read()

    print("Errors: {}".format(errors))
    print(messages.keys())
    print(messages["course_point_mesgs"])


if __name__ == "__main__":
    main()
