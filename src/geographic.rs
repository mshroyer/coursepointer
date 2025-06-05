//! Functions from GeographicLib
//!
//! Wraps the CXX FFI for GeographicLib in a friendlier interface.

use dimensioned::si::{M, Meter};
use thiserror::Error;

use crate::measure::Degree;
use crate::types::{GeoPoint, TypeError, XYPoint};

#[derive(Error, Debug)]
pub enum GeographicError {
    #[error("C++ exception from GeographicLib: {0}")]
    Exception(#[from] cxx::Exception),
    #[error("Core type error")]
    Type(#[from] TypeError),
}

type Result<T> = std::result::Result<T, GeographicError>;

/// A solution to the inverse problem in geodesy.
#[allow(dead_code)]
pub struct InverseSolution {
    /// Arc distance between the points.
    pub arc_distance: Degree<f64>,

    /// Geodesic distance between the points.
    pub geo_distance: Meter<f64>,

    /// Azimuth of the geodesic as measured at point1.
    pub azimuth1: Degree<f64>,

    /// Azimuth of the geodesic as measured at point1.
    pub azimuth2: Degree<f64>,
}

/// Calculate a solution to the inverse geodesic problem.
///
/// Finds the shortest geodesic between two points on the surface of WGS84,
/// ignoring any elevation data.
pub fn geodesic_inverse(point1: &GeoPoint, point2: &GeoPoint) -> Result<InverseSolution> {
    let mut geo_distance_m = 0.0;
    let mut azimuth1_deg = 0.0;
    let mut azimuth2_deg = 0.0;
    let arc_distance_deg = crate::ffi::geodesic_inverse_with_azimuth(
        point1.lat().0,
        point1.lon().0,
        point2.lat().0,
        point2.lon().0,
        &mut geo_distance_m,
        &mut azimuth1_deg,
        &mut azimuth2_deg,
    )?;

    Ok(InverseSolution {
        arc_distance: Degree(arc_distance_deg),
        geo_distance: geo_distance_m * M,
        azimuth1: Degree(azimuth1_deg),
        azimuth2: Degree(azimuth2_deg),
    })
}

/// A solution to the direct problem in geodesy.
#[allow(dead_code)]
pub struct DirectSolution {
    /// Arc distance between the points.
    pub arc_distance: Degree<f64>,

    /// Destination point.
    pub point2: GeoPoint,
}

/// Calculate a solution to the direct geodesic problem.
///
/// Given a start point, azimuth, and a geodesic distance, computes the point
/// where we end up and its arc distance from the start point.
pub fn geodesic_direct(
    point1: &GeoPoint,
    azimuth: Degree<f64>,
    distance: Meter<f64>,
) -> Result<DirectSolution> {
    let mut lat2_deg = 0.0;
    let mut lon2_deg = 0.0;
    let arc_distance_deg = crate::ffi::geodesic_direct(
        point1.lat().0,
        point1.lon().0,
        azimuth.0,
        distance.value_unsafe,
        &mut lat2_deg,
        &mut lon2_deg,
    )?;
    Ok(DirectSolution {
        arc_distance: Degree(arc_distance_deg),
        point2: GeoPoint::new(Degree(lat2_deg), Degree(lon2_deg), None)?,
    })
}

/// Calculate the forward gnomonic projection of a point.
///
/// Given a projection centerpoint `point0` and a point `point`, finds the
/// cartesian position of `point` in the gnomonic projection centered on
/// `point0`.
pub fn gnomonic_forward(point0: &GeoPoint, point: &GeoPoint) -> Result<XYPoint> {
    let mut result = XYPoint::default();
    crate::ffi::gnomonic_forward(
        point0.lat().0,
        point0.lon().0,
        point.lat().0,
        point.lon().0,
        &mut result.x.value_unsafe,
        &mut result.y.value_unsafe,
    )?;
    Ok(result)
}

/// Calculate the reverse gnomonic projection of a point.
///
/// Given a projection centerpoint `point0` and a projected (cartesian) point
/// `xypoint`, finds the latitude and longitude corresponding to `xypoint` given
/// the gnomonic projection centered on `point0`.
pub fn gnomonic_reverse(point0: &GeoPoint, xypoint: &XYPoint) -> Result<GeoPoint> {
    let mut lat_deg = 0.0;
    let mut lon_deg = 0.0;
    crate::ffi::gnomonic_reverse(
        point0.lat().0,
        point0.lon().0,
        xypoint.x.value_unsafe,
        xypoint.y.value_unsafe,
        &mut lat_deg,
        &mut lon_deg,
    )?;
    Ok(GeoPoint::new(Degree(lat_deg), Degree(lon_deg), None)?)
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use approx::assert_relative_eq;
    use dimensioned::si::M;

    use super::{geodesic_direct, geodesic_inverse, gnomonic_forward, gnomonic_reverse};
    use crate::measure::Degree;
    use crate::types::GeoPoint;

    #[test]
    fn test_geodesic_inverse() -> Result<()> {
        let point1 = GeoPoint::new(Degree(0.0), Degree(0.0), None)?;
        let point2 = GeoPoint::new(Degree(5.0), Degree(5.0), None)?;

        let result = geodesic_inverse(&point1, &point2)?;
        assert_relative_eq!(
            result.geo_distance,
            784029.0 * M,
            max_relative = 0.000_001 * M
        );
        Ok(())
    }

    #[test]
    fn test_geodesic_direct() -> Result<()> {
        let point1 = GeoPoint::new(Degree(10.0), Degree(-20.0), None)?;
        let point2 = GeoPoint::new(Degree(30.0), Degree(40.0), None)?;

        let inverse = geodesic_inverse(&point1, &point2)?;
        let result = geodesic_direct(&point1, inverse.azimuth1, inverse.geo_distance)?;
        // The direct result should reproduce the target point used to obtain
        // the inverse solution.
        assert_relative_eq!(result.point2, point2);
        Ok(())
    }

    #[test]
    fn test_gnomonic_forward() -> Result<()> {
        let point0 = GeoPoint::new(Degree(20.0), Degree(-40.0), None)?;
        let point = GeoPoint::new(Degree(17.0), Degree(-35.0), None)?;

        let result = gnomonic_forward(&point0, &point)?;
        // point's longitude is east of point0's
        assert!(result.x.value_unsafe > 0.0);
        // point's latitude is south of point0's
        assert!(result.y.value_unsafe < 0.0);
        Ok(())
    }

    #[test]
    fn test_gnomonic_reverse() -> Result<()> {
        let point0 = GeoPoint::new(Degree(20.0), Degree(-40.0), None)?;
        let point = GeoPoint::new(Degree(17.0), Degree(-35.0), None)?;

        let xypoint = gnomonic_forward(&point0, &point)?;
        let result = gnomonic_reverse(&point0, &xypoint)?;
        assert_relative_eq!(result, point);
        Ok(())
    }
}
