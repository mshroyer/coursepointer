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
 * Returns a string with GeographicLib's name and version number
 *
 * The returned pointer has static scope.
 */
EXTERN const char* geographiclib_version() noexcept;

/**
 * Returns a string with the compiler name and version number
 *
 * The returned pointer has static scope.
 */
EXTERN const char* compiler_version() noexcept;

#endif  // !defined COURSEPOINTER_GEO_SHIM_H
