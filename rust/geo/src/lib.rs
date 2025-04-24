#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("vendor/geographiclib/include/GeographicLib/Geodesic.hpp");

        fn Indirect(
            lat1: f64,
            lon1: f64,
            lat2: f64,
            lon2: f64,
            s12: &mut f64,
            azi1: &mut f64,
            azi2: &mut f64,
        ) -> f64;
    }
}
