# geo crate

Provides a thin wrapper around select functions in GeographicLib using cxx.

## Building

Building this crate requires the `geographiclib` git submodule. Initialize it with:

```bash
git submodule init
git submodule update
```

You should then be able to build the crate with `cargo build`.

## Updating GeographicLib

The build requires the cmake-generated `BUILD/include/Config.h` file to be coped into the include/geographiclib
directory. After upgrading the library, regenerated this header file by running:

```bash
mkdir vendor/geographiclib/BUILD
cd vendor/geographiclib/BUILD
cmake .. -DBUILD_SHARED_LIBS=OFF
```
