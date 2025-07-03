#include <optional>
#include <sstream>

#include <GeographicLib/Config.h>
#include <GeographicLib/Geocentric.hpp>
#include <GeographicLib/Geodesic.hpp>
#include <GeographicLib/Gnomonic.hpp>

#include "rust/cxx.h"

#define STRINGIFY_IMPL(x) #x
#define STRINGIFY(x) STRINGIFY_IMPL(x)

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

rust::Str geographiclib_version() noexcept {
  return GEOGRAPHICLIB_VERSION_STRING;
}

static std::string ver_string;

const char* msvc_version() noexcept {
  if (!ver_string.empty()) {
    return ver_string.c_str();
  }

  unsigned long full_ver = _MSC_FULL_VER;
  auto major = full_ver / 10000000;
  auto minor = (full_ver / 100000) % 100;
  auto patch = full_ver % 100000;

  std::ostringstream s;
  s << "MSVC " << major << "." << minor << "." << patch;
  ver_string = s.str();
  return ver_string.c_str();
}

rust::Str compiler_version() noexcept {
#if defined(_MSC_FULL_VER)
  return msvc_version();
#elif defined(__clang__)
  return "clang " STRINGIFY(__clang_major__) "." STRINGIFY(__clang_minor__) "." STRINGIFY(__clang_patchlevel__);
#else
  return "unknown";
#endif
}

}  // namespace CoursePointer
