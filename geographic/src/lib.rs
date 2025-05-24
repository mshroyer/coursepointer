use thiserror::Error;

#[cxx::bridge(namespace = "GeographicLib")]
mod ffi {
    unsafe extern "C++" {
        include!("geographic/geographiclib/include/GeographicLib/Geodesic.hpp");
        include!("geographic/include/shim.h");

        /// Get the static GeographicLib WGS84 geodesic.
        /// 
        /// We rely on C++11's guarantee of thread safety for the static local
        /// variable's initialization.
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
        ) -> Result<f64>;
    }
}

#[derive(Error, Debug)]
pub enum GeographicError {
    #[error("C++ exception from GeographicLib: {0}")]
    Exception(#[from] cxx::Exception),
}

type Result<T> = std::result::Result<T, GeographicError>;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SurfacePoint {
    pub lat: f64,
    pub lon: f64,
}

impl SurfacePoint {
    pub fn new(lat: f64, lon: f64) -> Self {
        Self { lat, lon }
    }
}

pub struct InverseSolution {
    pub meters: f64,
    pub azimuth1: f64,
    pub azimuth2: f64,
}

/// Calculate a solution to the inverse geodesic problem.
pub fn inverse(point1: &SurfacePoint, point2: &SurfacePoint) -> Result<InverseSolution> {
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
    )?;

    Ok(solution)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::SurfacePoint;
    use super::inverse;

    #[test]
    fn test_inverse() {
        let point1 = SurfacePoint::new(0.0, 0.0);
        let point2 = SurfacePoint::new(5.0, 5.0);

        let result = inverse(&point1, &point2).unwrap();
        assert_relative_eq!(result.meters, 784029.0, max_relative = 1.0);
    }
}
