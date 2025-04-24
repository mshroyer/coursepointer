#ifndef COURSEPOINTER_GEO_SHIM_H
#define COURSEPOINTER_GEO_SHIM_H

namespace GeographicLib {

class Geodesic;

const Geodesic& GetWGS84();

}  // namespace GeographicLib

#endif  // !defined COURSEPOINTER_GEO_SHIM_H
