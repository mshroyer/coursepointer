#include <sstream>

#include <GeographicLib/Config.h>
#include <GeographicLib/Geocentric.hpp>
#include <GeographicLib/Geodesic.hpp>
#include <GeographicLib/Gnomonic.hpp>

#include "shim.hpp"

#define STRINGIFY_IMPL(x) #x
#define STRINGIFY(x) STRINGIFY_IMPL(x)

using GeographicLib::Geocentric;
using GeographicLib::Geodesic;
using GeographicLib::Gnomonic;

namespace {

static_assert(std::is_same<GeographicLib::Math::real, double>::value,
    "ffi implementation requires Math::real to be double");

#ifdef _MSC_FULL_VER

static std::string ver_string;

const char* msvc_version(unsigned long full_ver) noexcept {
  if (!ver_string.empty()) {
    return ver_string.c_str();
  }

  auto major = full_ver / 10000000;
  auto minor = (full_ver / 100000) % 100;
  auto patch = full_ver % 100000;

  std::ostringstream s;
  s << "MSVC " << major << "." << minor << "." << patch;
  ver_string = s.str();
  return ver_string.c_str();
}

#endif  // defined _MSC_FULL_VER

const char* compiler_version() noexcept {
#if defined(_MSC_FULL_VER)
  return msvc_version(_MSC_FULL_VER);
#elif defined(__clang__)
  return "clang " STRINGIFY(__clang_major__) "." STRINGIFY(__clang_minor__) "." STRINGIFY(__clang_patchlevel__);
#elif defined(__GNUC__)
#ifdef __MINGW32__
#define CCNAME "mingw"
#else
#define CCNAME "gcc"
#endif
  return CCNAME " " STRINGIFY(__GNUC__) "." STRINGIFY(__GNUC_MINOR__) "." STRINGIFY(__GNUC_PATCHLEVEL__);
#else
  return "unknown";
#endif
}

const char* geographiclib_version() noexcept {
  return "GeographicLib " GEOGRAPHICLIB_VERSION_STRING;
}

}  // namespace


EXTERN bool geodesic_inverse_with_azimuth(
    double lat1, double lon1, double lat2, double lon2,
    double& s12, double& azi1, double& azi2, double& a12) {
  try {
    static auto geodesic = Geodesic::WGS84();
    a12 = geodesic.Inverse(lat1, lon1, lat2, lon2, s12, azi1, azi2);
  } catch (...) {
    return false;
  }
  return true;
}

EXTERN bool geodesic_direct(
    double lat1, double lon1, double az1, double s12,
    double& lat2, double& lon2, double& a12) {
  try {
    static auto geodesic = Geodesic::WGS84();
    a12 = geodesic.Direct(lat1, lon1, az1, s12, lat2, lon2);
  } catch (...) {
    return false;
  }
  return true;
}

EXTERN bool gnomonic_forward(
    double lat0, double lon0, double lat, double lon,
    double& x, double& y) {
  try {
    static auto gnomonic = Gnomonic(Geodesic::WGS84());
    gnomonic.Forward(lat0, lon0, lat, lon, x, y);
  } catch (...) {
    return false;
  }
  return true;
}

EXTERN bool gnomonic_reverse(
    double lat0, double lon0, double x, double y,
    double& lat, double& lon) {
  try {
    static auto gnomonic = Gnomonic(Geodesic::WGS84());
    gnomonic.Reverse(lat0, lon0, x, y, lat, lon);
  } catch (...) {
    return false;
  }
  return true;
}

EXTERN bool geocentric_forward(
    double lat, double lon, double h,
    double& x, double& y, double& z) {
  try {
    static auto geocentric = Geocentric::WGS84();
    geocentric.Forward(lat, lon, h, x, y, z);
  } catch (...) {
    return false;
  }
  return true;
}

EXTERN void get_geographiclib_version(char* buf, size_t buf_sz) {
  std::snprintf(buf, buf_sz, "%s", geographiclib_version());
}

EXTERN void get_compiler_version(char* buf, size_t buf_sz) {
  std::snprintf(buf, buf_sz, "%s", compiler_version());
}
