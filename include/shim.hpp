/**
 * Shim for Rust FFI
 *
 * Wrappers around GeographicLib functions to make them more palatable to Rust's
 * CXX.
 */

#ifndef COURSEPOINTER_GEO_SHIM_H
#define COURSEPOINTER_GEO_SHIM_H

#include "rust/cxx.h"

namespace CoursePointer {

bool geodesic_inverse_with_azimuth(
    double lat1, double lon1, double lat2, double lon2,
    double& s12, double& azi1, double& azi2, double& a12);

bool geodesic_direct(
    double lat1, double lon1, double az1, double s12,
    double& lat2, double& lon2, double& a12);

bool gnomonic_forward(
    double lat0, double lon0, double lat, double lon,
    double& x, double& y);

bool gnomonic_reverse(
    double lat0, double lon0, double x, double y,
    double& lat, double& lon);

bool geocentric_forward(
    double lat, double lon, double h,
    double& x, double& y, double& z);

/**
 * Returns a string with GeographicLib's name and version number
 *
 * The returned pointer has static scope.
 */
const char* geographiclib_version() noexcept;

/**
 * Returns a string with the compiler name and version number
 *
 * The returned pointer has static scope.
 */
const char* compiler_version() noexcept;

}  // namespace CoursePointer

#endif  // !defined COURSEPOINTER_GEO_SHIM_H
