// use cxx_build::CFG;

fn main() {
    // CFG.include_prefix = "../vendor/geographiclib/include";
    
    cxx_build::bridge("src/lib.rs")
        .file("vendor/geographiclib/src/Geodesic.cpp")
        .file("src/shim.cc")
        .std("c++14")
        .flag("-I../geo/vendor/geographiclib/include")
        .flag("-I../geo/vendor/geographiclib/BUILD/include")
        .compile("geo");
    
    // println!("cargo:rerun-if-changed=src/main.rs");
    // println!("cargo:rerun-if-changed=vendor/geographiclib/src/Geodesic.cpp");
    // println!("cargo:rerun-if-changed=vendor/geographiclib/include/GeographicLib/Geodesic.hpp");
}
