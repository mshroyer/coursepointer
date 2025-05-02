#ifndef COURSEPOINTER_GEO_SHIM_H
#define COURSEPOINTER_GEO_SHIM_H

namespace GeographicLib {

class Geodesic;

/**
 * Get the global instance of the WGS84 ellipsoid.
 *
 * This is a shim to invoke GeographicLib::Geodesic::WGS84(), since at the time
 * of writing the cxx binding generator seemingly can't invoke static member
 * functions.
 */
const Geodesic& GetWGS84();

}  // namespace GeographicLib

#endif  // !defined COURSEPOINTER_GEO_SHIM_H
