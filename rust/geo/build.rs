use cxx_build::CFG;

fn main() {
    CFG.include_prefix = "../vendor/geographiclib/include";
    
    // cxx_build::bridge("src/lib.rs")
    //     .file("vendor/geographiclib/src/Geodesic.cpp")
    //     .compile("geo");
    // 
    // println!("cargo:rerun-if-changed=src/main.rs");
    // println!("cargo:rerun-if-changed=vendor/geographiclib/src/Geodesic.cpp");
    // println!("cargo:rerun-if-changed=vendor/geographiclib/include/GeographicLib/Geodesic.hpp");
}
