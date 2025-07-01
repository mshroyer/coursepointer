# CoursePointer Changelog

## v0.2.1

- Made the `course` module a publically-accessible library that can be used to
  programmatically build courses from other forms of input.
- Added rayon-based parallelism and some optimization to reduce allocations.
  This doesn't change much for smaller, typical-size courses like sample
  `cptr004.gpx`, which currently processes in about 9ms on my 13th gen Intel
  laptop, but the stress test `cptr006.gpx` with `-t 1000` improves from
  around 240ms to 65ms.
- Breaking: Removed the `floor` feature for simplicity.  That optimization is
  now always in use.

## v0.2.0

Initial release.  Seems to work ok.
