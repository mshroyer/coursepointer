import garmin_fit_sdk


def validate_fit_file(path: str) -> None:
    stream = garmin_fit_sdk.Stream.from_file(path)
    decoder = garmin_fit_sdk.Decoder(stream)
    messages, errors = decoder.read()
    if errors:
        raise ValueError(f"Errors reading FIT file: {errors}")


def my_adder(a: int, b: int) -> int:
    return a + b
