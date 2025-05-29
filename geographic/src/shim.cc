#include <GeographicLib/Geodesic.hpp>

using GeographicLib::Geodesic;

namespace CoursePointer {

static_assert(std::is_same<GeographicLib::Math::real, double>::value,
    "ffi implementation requires Math::real to be double");

double wgs84_inverse_length_azimuth(
    double lat1, double lon1, double lat2, double lon2,
    double& s12, double& azi1, double& azi2) {
  return Geodesic::WGS84().Inverse(lat1, lon1, lat2, lon2, s12, azi1, azi2);
}

}  // namespace CoursePointer
