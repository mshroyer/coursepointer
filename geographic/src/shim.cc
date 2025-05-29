#include <GeographicLib/Geodesic.hpp>
#include <GeographicLib/Gnomonic.hpp>

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

void gnomonic_forward(
    double lat0, double lon0, double lat,
    double lon, double& x, double& y) {
  static auto gnomonic = Gnomonic(Geodesic::WGS84());
  gnomonic.Forward(lat0, lon0, lat, lon, x, y);
}

}  // namespace CoursePointer
