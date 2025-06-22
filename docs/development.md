## Git submodule

If you've cloned CoursePointer as a git repo, you'll need to import
[GeographicLib](https://geographiclib.sourceforge.io/C++/doc/index.html)'s C++
sources as a git submodule before building.  Run:

```
git submodule update --init
```

## Running and testing

The CLI runs with `cargo run -- <CLI_ARGS>`, and Rust-based tests run with
`cargo test`.

Many of the most important test cases are written as Python-based integration
tests instead of Rust tests, in order to use the
[garmin-fit-sdk](https://pypi.org/project/garmin-fit-sdk/) Python package as a
reference implementation for FIT decoding.  Run these with
[uv](https://docs.astral.sh/uv/):

```
uv run --package integration pytest
```

Some of the integration tests use the nested `integration-stub` binary crate
by passing JSON specifications to it and then examining its output.  Most
others build and test against the `coursepointer` command-line binary.

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
