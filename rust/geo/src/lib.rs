// static WGS84: LazyLock<ffi::Geodesic> = LazyLock::new(|| ffi::Geodesic::new(0.0, 0.0));

#[cxx::bridge(namespace = "GeographicLib")]
mod ffi {
    unsafe extern "C++" {
        include!("geo/vendor/geographiclib/include/GeographicLib/Geodesic.hpp");
        include!("geo/include/shim.h");

        fn GetWGS84() -> &'static Geodesic;

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

#[derive(Debug)]
pub struct SurfacePoint {
    lat: f64,
    lon: f64,
}

impl SurfacePoint {
    pub fn new(lat: f64, lon: f64) -> Self {
        Self { lat, lon }
    }
}

pub struct InverseSolution {
    meters: f64,
    azimuth1: f64,
    azimuth2: f64,
}

/// Calculate a solution to the inverse geodesic problem.
pub fn inverse(point1: &SurfacePoint, point2: &SurfacePoint) -> Result<InverseSolution, String> {
    let mut solution = InverseSolution {
        meters: 0.0,
        azimuth1: 0.0,
        azimuth2: 0.0,
    };

    let _ = ffi::GetWGS84().Inverse(
        point1.lat,
        point1.lon,
        point2.lat,
        point2.lon,
        &mut solution.meters,
        &mut solution.azimuth1,
        &mut solution.azimuth2,
    );

    Ok(solution)
}

#[cfg(test)]
mod tests {
    use super::SurfacePoint;
    use super::inverse;

    #[test]
    fn test_inverse() {
        let point1 = SurfacePoint::new(52.0, 13.0);
        let point2 = SurfacePoint::new(48.0, 2.0);

        let result = inverse(&point1, &point2).unwrap();
        assert!(result.meters > 0.0);
    }
}
