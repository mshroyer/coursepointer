## Git submodule

If you've cloned CoursePointer as a git repo, you'll need to import
[GeographicLib](https://geographiclib.sourceforge.io/C++/doc/index.html)'s C++
sources as a git submodule before building.  Run:

```
git submodule init
git submodule update
```

## Running and testing

The CLI and Rust tests run with `cargo run` and `cargo test`.

Many test cases are written as Python integration tests, in order to use the
[garmin-fit-sdk](https://pypi.org/project/garmin-fit-sdk/) Python package as a
reference implementation for FIT decoding.  Run these with
[uv](https://docs.astral.sh/uv/):

```
uv run --package integration pytest
```

Some of the integration tests use the nested `integration-stub` binary crate
by passing JSON specifications to it and then examining its output.

## Formatting

Though the project builds with the stable Rust toolchain, it uses nightly for
`cargo fmt` for access to unstable features:

```
rustup toolchain install nightly
cargo +nightly fmt
```

Python code is formatted by running [ruff](https://docs.astral.sh/ruff/) from
the top-level project directory:

```
ruff check
ruff format
```
