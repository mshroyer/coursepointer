use coretypes::GeoPoint;
use coretypes::XYPoint;
use coretypes::measure::{Degrees, Meters};
use thiserror::Error;

#[cxx::bridge(namespace = "CoursePointer")]
mod ffi {
    unsafe extern "C++" {
        include!("geographic/include/shim.h");

        #[allow(clippy::too_many_arguments)]
        fn wgs84_inverse_length_azimuth(
            lat1: f64,
            lon1: f64,
            lat2: f64,
            lon2: f64,
            s12: &mut f64,
            azi1: &mut f64,
            azi2: &mut f64,
        ) -> Result<f64>;

        #[allow(clippy::too_many_arguments)]
        fn wgs84_gnomonic_forward(
            lat1: f64,
            lon1: f64,
            lat: f64,
            lon: f64,
            x: &mut f64,
            y: &mut f64,
        ) -> Result<()>;
    }
}

#[derive(Error, Debug)]
pub enum GeographicError {
    #[error("C++ exception from GeographicLib: {0}")]
    Exception(#[from] cxx::Exception),
}

type Result<T> = std::result::Result<T, GeographicError>;

/// A solution to the inverse problem in geodesy.
pub struct InverseSolution {
    /// Arc distance between the points.
    pub arc_distance: Degrees<f64>,

    /// Geodesic distance between the points.
    pub geo_distance: Meters<f64>,

    /// Azimuth of the geodesic as measured at point1.
    pub azimuth1: Degrees<f64>,

    /// Azimuth of the geodesic as measured at point2.
    pub azimuth2: Degrees<f64>,
}

/// Calculate a solution to the inverse geodesic problem.
///
/// Finds the shortest geodesic between two points on the surface of WGS84,
/// ignoring any elevation data.
pub fn solve_inverse(point1: &GeoPoint, point2: &GeoPoint) -> Result<InverseSolution> {
    let mut geo_distance_m = 0.0;
    let mut azimuth1_deg = 0.0;
    let mut azimuth2_deg = 0.0;
    let arc_distance_deg = ffi::wgs84_inverse_length_azimuth(
        point1.lat().0,
        point1.lon().0,
        point2.lat().0,
        point2.lon().0,
        &mut geo_distance_m,
        &mut azimuth1_deg,
        &mut azimuth2_deg,
    )?;

    Ok(InverseSolution {
        arc_distance: Degrees(arc_distance_deg),
        geo_distance: Meters(geo_distance_m),
        azimuth1: Degrees(azimuth1_deg),
        azimuth2: Degrees(azimuth2_deg),
    })
}

pub fn gnomonic_forward(
    point0: &GeoPoint,
    point: &GeoPoint,
) -> Result<XYPoint> {
    let mut result = XYPoint::default();
    ffi::wgs84_gnomonic_forward(
        point0.lat().0,
        point0.lon().0,
        point.lat().0,
        point.lon().0,
        &mut result.x.0,
        &mut result.y.0,
    )?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use coretypes::measure::Degrees;
    use coretypes::{GeoPoint, TypeError};

    use super::solve_inverse;

    #[test]
    fn test_inverse() -> Result<(), TypeError> {
        let point1 = GeoPoint::new(Degrees(0.0), Degrees(0.0), None)?;
        let point2 = GeoPoint::new(Degrees(5.0), Degrees(5.0), None)?;

        let result = solve_inverse(&point1, &point2).unwrap();
        assert_relative_eq!(result.geo_distance.0, 784029.0, max_relative = 1.0);
        Ok(())
    }
}
