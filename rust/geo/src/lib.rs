#[cxx::bridge(namespace = "GeographicLib")]
mod ffi {
    unsafe extern "C++" {
        include!("geo/vendor/geographiclib/include/GeographicLib/Geodesic.hpp");

        type Geodesic;

        fn Inverse(
            &self,
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
