# geo crate

Provides a thin wrapper around select functions in GeographicLib using cxx.

## Building

Building this crate requires the `geographiclib` git submodule. Initialize it with:

```bash
git submodule init
git submodule update
```

It also requires the cmake-generated `BUILD/include/Config.h` file. To generated it run:

```bash
mkdir vendor/geographiclib/BUILD
cd vendor/geographiclib/BUILD
cmake ..
```

You should then be able to build the crate with `cargo build`.
