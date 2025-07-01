#include <GeographicLib/Config.h>
#include <GeographicLib/Geocentric.hpp>
#include <GeographicLib/Geodesic.hpp>
#include <GeographicLib/Gnomonic.hpp>

#include "rust/cxx.h"

using GeographicLib::Geocentric;
using GeographicLib::Geodesic;
using GeographicLib::Gnomonic;

namespace CoursePointer {

static_assert(std::is_same<GeographicLib::Math::real, double>::value,
    "ffi implementation requires Math::real to be double");

double geodesic_inverse_with_azimuth(
    double lat1, double lon1, double lat2, double lon2,
    double& s12, double& azi1, double& azi2) {
  static auto geodesic = Geodesic::WGS84();
  return geodesic.Inverse(lat1, lon1, lat2, lon2, s12, azi1, azi2);
}

double geodesic_direct(
    double lat1, double lon1, double az1, double s12,
    double& lat2, double& lon2) {
  static auto geodesic = Geodesic::WGS84();
  return geodesic.Direct(lat1, lon1, az1, s12, lat2, lon2);
}

void gnomonic_forward(
    double lat0, double lon0, double lat, double lon,
    double& x, double& y) {
  static auto gnomonic = Gnomonic(Geodesic::WGS84());
  gnomonic.Forward(lat0, lon0, lat, lon, x, y);
}

void gnomonic_reverse(
    double lat0, double lon0, double x, double y,
    double& lat, double& lon) {
  static auto gnomonic = Gnomonic(Geodesic::WGS84());
  gnomonic.Reverse(lat0, lon0, x, y, lat, lon);
}

void geocentric_forward(
    double lat, double lon, double h,
    double& x, double& y, double& z) {
  static auto geocentric = Geocentric::WGS84();
  geocentric.Forward(lat, lon, h, x, y, z);
}

rust::Str geographiclib_version_string() {
  return GEOGRAPHICLIB_VERSION_STRING;
}

}  // namespace CoursePointer
