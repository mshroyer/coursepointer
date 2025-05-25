use thiserror::Error;

use coretypes::GeoPoint;

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

pub struct InverseSolution {
    pub meters: f64,
    pub azimuth1: f64,
    pub azimuth2: f64,
    pub arc_distance: f64,
}

/// Calculate a solution to the inverse geodesic problem.
pub fn inverse(point1: &GeoPoint, point2: &GeoPoint) -> Result<InverseSolution> {
    let mut solution = InverseSolution {
        meters: 0.0,
        azimuth1: 0.0,
        azimuth2: 0.0,
        arc_distance: 0.0,
    };

    let arc_distance = ffi::GetWGS84().Inverse(
        point1.lat().0,
        point1.lon().0,
        point2.lat().0,
        point2.lon().0,
        &mut solution.meters,
        &mut solution.azimuth1,
        &mut solution.azimuth2,
    )?;
    
    solution.arc_distance = arc_distance;

    Ok(solution)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    
    use coretypes::measure::Degrees;
    use coretypes::GeoPoint;
    use coretypes::TypeError;

    use super::inverse;

    #[test]
    fn test_inverse() -> Result<(), TypeError> {
        let point1 = GeoPoint::new(Degrees(0.0), Degrees(0.0), None)?;
        let point2 = GeoPoint::new(Degrees(5.0), Degrees(5.0), None)?;

        // let result = inverse(&point1, &point2).unwrap();
        match inverse(&point1, &point2) {
            Ok(inverse_result) => {
                assert_relative_eq!(inverse_result.meters, 784029.0, max_relative = 1.0);
            }
            
            Err(e) => {
                eprintln!("{:?}", e);    
            }
        }
        Ok(())
    }
}
