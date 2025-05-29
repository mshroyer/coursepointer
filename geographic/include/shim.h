#ifndef COURSEPOINTER_GEO_SHIM_H
#define COURSEPOINTER_GEO_SHIM_H

#include <GeographicLib/Geodesic.hpp>

namespace CoursePointer {

double wgs84_inverse_length_azimuth(
    double lat1, double lon1, double lat2, double lon2,
    double& s12, double& azi1, double& azi2);

}  // namespace CoursePointer

#endif  // !defined COURSEPOINTER_GEO_SHIM_H
