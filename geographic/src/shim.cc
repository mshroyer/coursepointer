#include <GeographicLib/Geodesic.hpp>

namespace GeographicLib {

const Geodesic& GetWGS84() {
    return Geodesic::WGS84();
}

}  // namespace GeographicLib
