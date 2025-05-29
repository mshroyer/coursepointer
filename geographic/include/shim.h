#ifndef COURSEPOINTER_GEO_SHIM_H
#define COURSEPOINTER_GEO_SHIM_H

namespace CoursePointer {

double geodesic_inverse_with_azimuth(
    double lat1, double lon1, double lat2, double lon2,
    double& s12, double& azi1, double& azi2);

void gnomonic_forward(
    double lat0, double lon0, double lat,
    double lon, double& x, double& y);

}  // namespace CoursePointer

#endif  // !defined COURSEPOINTER_GEO_SHIM_H
