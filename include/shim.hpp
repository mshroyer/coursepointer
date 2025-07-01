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

double geodesic_inverse_with_azimuth(
    double lat1, double lon1, double lat2, double lon2,
    double& s12, double& azi1, double& azi2);

double geodesic_direct(
    double lat1, double lon1, double az1, double s12,
    double& lat2, double& lon2);

void gnomonic_forward(
    double lat0, double lon0, double lat, double lon,
    double& x, double& y);

void gnomonic_reverse(
    double lat0, double lon0, double x, double y,
    double& lat, double& lon);

void geocentric_forward(
    double lat, double lon, double h,
    double& x, double& y, double& z);

rust::Str geographiclib_version_string();

}  // namespace CoursePointer

#endif  // !defined COURSEPOINTER_GEO_SHIM_H
