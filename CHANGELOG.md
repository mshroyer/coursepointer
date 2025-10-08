# CoursePointer Changelog

## v0.3.4

- Use the current time, instead of an arbitrary fixed timestamp, as
  `time_created` in the CLI, and as the default value in the library.  Thanks to
  @nicolacimmino for pointing out that a fixed timestamp can cause devices to
  treat different routes as though they have the same unique identifier.
- Updated to GeographicLib 2.5.2.

## v0.3.3

- Changed default FIT sport to Generic.
- Some enhancements to the proof-of-concept WASM application.
- Updates tracing-subscriber to address a security vulnerability flagged by
  Dependabot, which probably had no impact on coursepointer but just to be
  safe: https://github.com/mshroyer/coursepointer/security/dependabot/1

## v0.3.2

- `pub use TypeError` so external users can deal with type invariant errors
  from `GeoPoint`.

## v0.3.1

- Fix issues with rust docs.

## v0.3.0

- Made the `course` module a public library that can be used to build courses
  in other applications.
- Added rayon-based parallelism, and another optimization that reduced
  allocations.  This doesn't change performance much for smaller, typical-size
  courses like sample `cptr004.gpx`, which currently processes in about 9ms on
  my 13th gen Intel laptop.  But the stress test `cptr006.gpx` with `-t 1000`
  improves from around 240ms to 65ms with these changes.
- Added a CLI option to specify FIT sport.
- Breaking change: Removed the `floor` feature for simplicity.  That
  optimization is now always in use.

## v0.2.0

Initial release.  Seems to work ok.
