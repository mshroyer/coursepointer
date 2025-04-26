fn main() {
    cxx_build::bridge("src/lib.rs")
        .files([
            "src/shim.cc",
            "vendor/geographiclib/src/Accumulator.cpp",
            "vendor/geographiclib/src/AlbersEqualArea.cpp",
            "vendor/geographiclib/src/AuxAngle.cpp",
            "vendor/geographiclib/src/AuxLatitude.cpp",
            "vendor/geographiclib/src/AzimuthalEquidistant.cpp",
            "vendor/geographiclib/src/CassiniSoldner.cpp",
            "vendor/geographiclib/src/CircularEngine.cpp",
            "vendor/geographiclib/src/DAuxLatitude.cpp",
            "vendor/geographiclib/src/DMS.cpp",
            "vendor/geographiclib/src/DST.cpp",
            "vendor/geographiclib/src/Ellipsoid.cpp",
            "vendor/geographiclib/src/EllipticFunction.cpp",
            "vendor/geographiclib/src/GARS.cpp",
            "vendor/geographiclib/src/Geocentric.cpp",
            "vendor/geographiclib/src/GeoCoords.cpp",
            "vendor/geographiclib/src/Geodesic.cpp",
            "vendor/geographiclib/src/GeodesicExact.cpp",
            "vendor/geographiclib/src/GeodesicLine.cpp",
            "vendor/geographiclib/src/GeodesicLineExact.cpp",
            "vendor/geographiclib/src/Geohash.cpp",
            "vendor/geographiclib/src/Geoid.cpp",
            "vendor/geographiclib/src/Georef.cpp",
            "vendor/geographiclib/src/Gnomonic.cpp",
            "vendor/geographiclib/src/GravityCircle.cpp",
            "vendor/geographiclib/src/GravityModel.cpp",
            "vendor/geographiclib/src/Intersect.cpp",
            "vendor/geographiclib/src/LambertConformalConic.cpp",
            "vendor/geographiclib/src/LocalCartesian.cpp",
            "vendor/geographiclib/src/MagneticCircle.cpp",
            "vendor/geographiclib/src/MagneticModel.cpp",
            "vendor/geographiclib/src/Math.cpp",
            "vendor/geographiclib/src/MGRS.cpp",
            "vendor/geographiclib/src/NormalGravity.cpp",
            "vendor/geographiclib/src/OSGB.cpp",
            "vendor/geographiclib/src/PolarStereographic.cpp",
            "vendor/geographiclib/src/PolygonArea.cpp",
            "vendor/geographiclib/src/Rhumb.cpp",
            "vendor/geographiclib/src/SphericalEngine.cpp",
            "vendor/geographiclib/src/TransverseMercator.cpp",
            "vendor/geographiclib/src/TransverseMercatorExact.cpp",
            "vendor/geographiclib/src/Utility.cpp",
            "vendor/geographiclib/src/UTMUPS.cpp",
        ])
        .flag("-I../geo/vendor/geographiclib/include")
        .flag("-I../geo/vendor/geographiclib/BUILD/include")
        .compile("geocxx");

    // println!("cargo:rerun-if-changed=src/main.rs");
    // println!("cargo:rerun-if-changed=vendor/geographiclib/src/Geodesic.cpp");
    // println!("cargo:rerun-if-changed=vendor/geographiclib/include/GeographicLib/Geodesic.hpp");
}
