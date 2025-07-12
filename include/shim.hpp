/**
 * Shim for Rust FFI
 *
 * Wrappers around GeographicLib functions to make them more palatable to Rust's
 * CXX.
 */

#ifndef COURSEPOINTER_GEO_SHIM_H
#define COURSEPOINTER_GEO_SHIM_H

#ifdef __EMSCRIPTEN__
#include <emscripten/emscripten.h>
#define EXTERN extern "C" EMSCRIPTEN_KEEPALIVE
#else
#define EXTERN extern "C"
#endif  // defined __EMSCRIPTEN__

EXTERN bool geodesic_inverse_with_azimuth(
    double lat1, double lon1, double lat2, double lon2,
    double& s12, double& azi1, double& azi2, double& a12);

EXTERN bool geodesic_direct(
    double lat1, double lon1, double az1, double s12,
    double& lat2, double& lon2, double& a12);

EXTERN bool gnomonic_forward(
    double lat0, double lon0, double lat, double lon,
    double& x, double& y);

EXTERN bool gnomonic_reverse(
    double lat0, double lon0, double x, double y,
    double& lat, double& lon);

EXTERN bool geocentric_forward(
    double lat, double lon, double h,
    double& x, double& y, double& z);

/**
 * Gets a string with GeographicLib's name and version number
 *
 * The string is written to the provided buffer, truncated if necessary, with a
 * null terminator.  In native code we could simply return a statically-scoped
 * string pointer, buf for webassembly this allows GeographicLib and
 * CoursePointer to be compiled into separate modules that do not necessarily
 * share memory.
 */
EXTERN void get_geographiclib_version(char* buf, size_t buf_sz);

/**
 * Returns a string with the compiler name and version number
 *
 * Works the same as `get_geographiclib_version`.
 */
EXTERN void get_compiler_version(char* buf, size_t buf_sz);

#endif  // !defined COURSEPOINTER_GEO_SHIM_H
