#ifndef __COURSEPOINTER_SHIM_H__
#define __COURSEPOINTER_SHIM_H__

#ifdef __EMSCRIPTEN__
#include <emscripten/emscripten.h>
#define EXTERN extern "C" EMSCRIPTEN_KEEPALIVE
#else
#define EXTERN extern "C"
#endif  // defined __EMSCRIPTEN__

EXTERN bool geodesic_direct(double lat1, double lon1, double azi1, double s12,
                            double* lat2, double* lon2, double* a12) noexcept;

EXTERN bool geodesic_inverse_with_azimuth(double lat1, double lon1, double lat2,
                                          double lon2, double* s12,
                                          double* azi1, double* azi2,
                                          double* a12) noexcept;

EXTERN bool gnomonic_forward(double lat0, double lon0, double lat, double lon,
                             double* x, double* y) noexcept;

EXTERN bool gnomonic_reverse(double lat0, double lon0, double x, double y,
                             double* lat, double* lon) noexcept;

EXTERN bool geocentric_forward(double lat, double lon, double h, double* x,
                               double* y, double* z) noexcept;

/**
 * Gets a string with GeographicLib's name and version number
 *
 * The string returned has static lifetime.
 */
EXTERN const char* geographiclib_version() noexcept;

/**
 * Gets a string with the C++ compiler's name and version number
 */
EXTERN const char* compiler_version() noexcept;

#endif  // defined __COURSEPOINTER_SHIM_H__
