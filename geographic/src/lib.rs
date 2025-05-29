use coretypes::measure::{Degrees, Meters};
use coretypes::{GeoPoint, XYPoint};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GeographicError {
    #[error("C++ exception from GeographicLib: {0}")]
    Exception(#[from] cxx::Exception),
}

type Result<T> = std::result::Result<T, GeographicError>;

#[allow(clippy::too_many_arguments)]
#[cxx::bridge(namespace = "CoursePointer")]
mod ffi {
    unsafe extern "C++" {
        include!("geographic/include/shim.h");

        fn geodesic_inverse_with_azimuth(
            lat1: f64,
            lon1: f64,
            lat2: f64,
            lon2: f64,
            s12: &mut f64,
            azi1: &mut f64,
            azi2: &mut f64,
        ) -> Result<f64>;

        fn gnomonic_forward(
            lat1: f64,
            lon1: f64,
            lat: f64,
            lon: f64,
            x: &mut f64,
            y: &mut f64,
        ) -> Result<()>;
    }
}

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
pub fn geodesic_inverse(point1: &GeoPoint, point2: &GeoPoint) -> Result<InverseSolution> {
    let mut geo_distance_m = 0.0;
    let mut azimuth1_deg = 0.0;
    let mut azimuth2_deg = 0.0;
    let arc_distance_deg = ffi::geodesic_inverse_with_azimuth(
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

/// Calculate the forward gnomonic project of a point.
///
/// Given a projection centerpoint `point0` and a point `point`, finds the
/// cartesian position of `point` in the gnomonic projection centered on
/// `point0`.
pub fn gnomonic_forward(point0: &GeoPoint, point: &GeoPoint) -> Result<XYPoint> {
    let mut result = XYPoint::default();
    ffi::gnomonic_forward(
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
    use anyhow::Result;
    use approx::assert_relative_eq;
    use coretypes::GeoPoint;
    use coretypes::measure::Degrees;

    use super::{geodesic_inverse, gnomonic_forward};

    #[test]
    fn test_geodesic_inverse() -> Result<()> {
        let point1 = GeoPoint::new(Degrees(0.0), Degrees(0.0), None)?;
        let point2 = GeoPoint::new(Degrees(5.0), Degrees(5.0), None)?;

        let result = geodesic_inverse(&point1, &point2).unwrap();
        assert_relative_eq!(result.geo_distance.0, 784029.0, epsilon = 1.0);
        Ok(())
    }

    #[test]
    fn test_gnomonic_forward() -> Result<()> {
        let point0 = GeoPoint::new(Degrees(20.0), Degrees(-40.0), None)?;
        let point = GeoPoint::new(Degrees(17.0), Degrees(-35.0), None)?;

        let result = gnomonic_forward(&point0, &point)?;
        // point longitude is east of point0's
        assert!(result.x.0 > 0.0);
        // point latitude is south of point0's
        assert!(result.y.0 < 0.0);
        Ok(())
    }
}
