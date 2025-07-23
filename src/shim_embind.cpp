#include <emscripten/bind.h>
#include <emscripten/emscripten.h>

#include "shim.h"

using namespace emscripten;

struct DirectSolution {
  bool ok;
  double lat2;
  double lon2;
  double a12;
};

DirectSolution embind_geodesic_direct(double lat1, double lon1, double azi1,
                                      double s12) {
  DirectSolution sln;
  sln.ok =
      geodesic_direct(lat1, lon1, azi1, s12, &sln.lat2, &sln.lon2, &sln.a12);
  return sln;
}

struct InverseSolution {
  bool ok;
  double s12;
  double azi1;
  double azi2;
  double a12;
};

InverseSolution embind_geodesic_inverse(double lat1, double lon1, double lat2,
                                        double lon2) {
  InverseSolution sln;
  sln.ok = geodesic_inverse_with_azimuth(lat1, lon1, lat2, lon2, &sln.s12,
                                         &sln.azi1, &sln.azi2, &sln.a12);
  return sln;
}

struct XyPoint {
  bool ok;
  double x;
  double y;
};

XyPoint embind_gnomonic_forward(double lat0, double lon0, double lat,
                                double lon) {
  XyPoint sln;
  sln.ok = gnomonic_forward(lat0, lon0, lat, lon, &sln.x, &sln.y);
  return sln;
}

struct GeoPoint {
  bool ok;
  double lat;
  double lon;
};

GeoPoint embind_gnomonic_reverse(double lat0, double lon0, double x, double y) {
  GeoPoint sln;
  sln.ok = gnomonic_reverse(lat0, lon0, x, y, &sln.lat, &sln.lon);
  return sln;
}

struct XyzPoint {
  bool ok;
  double x;
  double y;
  double z;
};

XyzPoint embind_geocentric_forward(double lat, double lon, double h) {
  XyzPoint sln;
  sln.ok = geocentric_forward(lat, lon, h, &sln.x, &sln.y, &sln.z);
  return sln;
}

std::string embind_geographiclib_version() {
  return geographiclib_version();
}

std::string embind_compiler_version() {
  return compiler_version();
}

EMSCRIPTEN_KEEPALIVE
EMSCRIPTEN_BINDINGS(geographiclib_shim) {
  value_object<DirectSolution>("DirectSolution")
      .field("ok", &DirectSolution::ok)
      .field("lat2", &DirectSolution::lat2)
      .field("lon2", &DirectSolution::lon2)
      .field("a12", &DirectSolution::a12);

  function("geodesic_direct", &embind_geodesic_direct);

  value_object<InverseSolution>("InverseSolution")
      .field("ok", &InverseSolution::ok)
      .field("s12", &InverseSolution::s12)
      .field("azi1", &InverseSolution::azi1)
      .field("azi2", &InverseSolution::azi2)
      .field("a12", &InverseSolution::a12);

  function("geodesic_inverse", &embind_geodesic_inverse);

  value_object<XyPoint>("XyPoint")
      .field("ok", &XyPoint::ok)
      .field("x", &XyPoint::x)
      .field("y", &XyPoint::y);

  function("gnomonic_forward", &embind_gnomonic_forward);

  value_object<GeoPoint>("GeoPoint")
      .field("ok", &GeoPoint::ok)
      .field("lat", &GeoPoint::lat)
      .field("lon", &GeoPoint::lon);

  function("gnomonic_reverse", &embind_gnomonic_reverse);

  value_object<XyzPoint>("XyzPoint")
      .field("ok", &XyzPoint::ok)
      .field("x", &XyzPoint::x)
      .field("y", &XyzPoint::y)
      .field("z", &XyzPoint::z);

  function("geocentric_forward", &embind_geocentric_forward);

  function("geographiclib_version", &embind_geographiclib_version);

  function("compiler_version", &embind_compiler_version);
}
