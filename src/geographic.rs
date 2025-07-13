//! Functions from GeographicLib
//!
//! Wraps the CXX FFI for GeographicLib in a friendlier interface.

use dimensioned::si::Meter;
use thiserror::Error;
pub use wrappers::{
    compiler_version_str, geocentric_forward, geodesic_direct, geodesic_inverse,
    geographiclib_version_str, gnomonic_forward, gnomonic_reverse,
};

use crate::measure::Degree;
use crate::types::{GeoAndXyzPoint, GeoPoint, TypeError, XyzPoint};

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum GeographicError {
    #[error("Unknown C++ exception from GeographicLib")]
    UnknownException,
    #[error("Core type error")]
    Type(#[from] TypeError),
    #[error("JSON deserialization")]
    Json(String),
}

type Result<T> = std::result::Result<T, GeographicError>;

/// A solution to the direct problem in geodesy.
#[allow(dead_code)]
pub struct DirectSolution {
    /// Arc distance between the points.
    pub arc_distance: Degree<f64>,

    /// Destination point.
    pub point2: GeoPoint,
}

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

#[cfg(not(feature = "jsffi"))]
mod wrappers {
    use std::ffi::CStr;

    use dimensioned::si::{M, Meter};

    use crate::geographic::wrappers::ffi::{compiler_version, geographiclib_version};
    use crate::geographic::{DirectSolution, GeographicError, InverseSolution, Result};
    use crate::types::{XyPoint, XyzPoint};
    use crate::{DEG, Degree, GeoPoint};

    /// Calculate a solution to the direct geodesic problem.
    ///
    /// Given a start point, azimuth, and a geodesic distance, computes the
    /// point where we end up and its arc distance from the start point.
    pub fn geodesic_direct(
        point1: &GeoPoint,
        azimuth: Degree<f64>,
        distance: Meter<f64>,
    ) -> Result<DirectSolution> {
        let mut lat2_deg = 0.0;
        let mut lon2_deg = 0.0;
        let mut arc_distance_deg = 0.0;
        let ok = unsafe {
            ffi::geodesic_direct(
                point1.lat().value_unsafe,
                point1.lon().value_unsafe,
                azimuth.value_unsafe,
                distance.value_unsafe,
                &mut lat2_deg,
                &mut lon2_deg,
                &mut arc_distance_deg,
            )
        };

        if ok {
            Ok(DirectSolution {
                arc_distance: arc_distance_deg * DEG,
                point2: GeoPoint::new(lat2_deg * DEG, lon2_deg * DEG, None)?,
            })
        } else {
            Err(GeographicError::UnknownException)
        }
    }

    /// Calculate a solution to the inverse geodesic problem.
    ///
    /// Finds the shortest geodesic between two points on the surface of WGS84,
    /// ignoring any elevation data.
    pub fn geodesic_inverse(point1: &GeoPoint, point2: &GeoPoint) -> Result<InverseSolution> {
        let mut geo_distance_m = 0.0;
        let mut azimuth1_deg = 0.0;
        let mut azimuth2_deg = 0.0;
        let mut arc_distance_deg = 0.0;
        let ok = unsafe {
            ffi::geodesic_inverse_with_azimuth(
                point1.lat().value_unsafe,
                point1.lon().value_unsafe,
                point2.lat().value_unsafe,
                point2.lon().value_unsafe,
                &mut geo_distance_m,
                &mut azimuth1_deg,
                &mut azimuth2_deg,
                &mut arc_distance_deg,
            )
        };

        if ok {
            Ok(InverseSolution {
                arc_distance: arc_distance_deg * DEG,
                geo_distance: geo_distance_m * M,
                azimuth1: azimuth1_deg * DEG,
                azimuth2: azimuth2_deg * DEG,
            })
        } else {
            Err(GeographicError::UnknownException)
        }
    }

    /// Calculate the forward gnomonic projection of a point.
    ///
    /// Given a projection centerpoint `point0` and a point `point`, finds the
    /// cartesian position of `point` in the gnomonic projection centered on
    /// `point0`.
    pub fn gnomonic_forward(point0: &GeoPoint, point: &GeoPoint) -> Result<XyPoint> {
        let mut result = XyPoint::default();
        let ok = unsafe {
            ffi::gnomonic_forward(
                point0.lat().value_unsafe,
                point0.lon().value_unsafe,
                point.lat().value_unsafe,
                point.lon().value_unsafe,
                &mut result.x.value_unsafe,
                &mut result.y.value_unsafe,
            )
        };

        if ok {
            Ok(result)
        } else {
            Err(GeographicError::UnknownException)
        }
    }

    /// Calculate the reverse gnomonic projection of a point.
    ///
    /// Given a projection centerpoint `point0` and a projected (cartesian)
    /// point `xypoint`, finds the latitude and longitude corresponding to
    /// `xypoint` given the gnomonic projection centered on `point0`.
    pub fn gnomonic_reverse(point0: &GeoPoint, xypoint: &XyPoint) -> Result<GeoPoint> {
        let mut lat_deg = 0.0;
        let mut lon_deg = 0.0;
        let ok = unsafe {
            ffi::gnomonic_reverse(
                point0.lat().value_unsafe,
                point0.lon().value_unsafe,
                xypoint.x.value_unsafe,
                xypoint.y.value_unsafe,
                &mut lat_deg,
                &mut lon_deg,
            )
        };

        if ok {
            Ok(GeoPoint::new(lat_deg * DEG, lon_deg * DEG, None)?)
        } else {
            Err(GeographicError::UnknownException)
        }
    }

    pub fn geocentric_forward(point: &GeoPoint) -> Result<XyzPoint> {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut z = 0.0;
        let ok = unsafe {
            ffi::geocentric_forward(
                point.lat().value_unsafe,
                point.lon().value_unsafe,
                0.0,
                &mut x,
                &mut y,
                &mut z,
            )
        };

        if ok {
            Ok(XyzPoint {
                x: x * M,
                y: y * M,
                z: z * M,
            })
        } else {
            Err(GeographicError::UnknownException)
        }
    }

    pub fn geographiclib_version_str() -> &'static str {
        unsafe { CStr::from_ptr(geographiclib_version()).to_str().unwrap() }
    }

    pub fn compiler_version_str() -> &'static str {
        unsafe { CStr::from_ptr(compiler_version()).to_str().unwrap() }
    }

    /// CXX Generated FFI for GeographicLib
    ///
    /// This currently has to be inline in lib.rs because non-inline mods in
    /// proc macro input are unstable: <https://github.com/rust-lang/rust/issues/54727>
    #[allow(clippy::too_many_arguments)]
    mod ffi {
        use std::ffi::c_char;

        unsafe extern "C" {
            pub fn geodesic_direct(
                lat1: f64,
                lon1: f64,
                az1: f64,
                s12: f64,
                lat2: &mut f64,
                lon2: &mut f64,
                a12: &mut f64,
            ) -> bool;

            pub fn geodesic_inverse_with_azimuth(
                lat1: f64,
                lon1: f64,
                lat2: f64,
                lon2: f64,
                s12: &mut f64,
                azi1: &mut f64,
                azi2: &mut f64,
                a12: &mut f64,
            ) -> bool;

            pub fn gnomonic_forward(
                lat1: f64,
                lon1: f64,
                lat: f64,
                lon: f64,
                x: &mut f64,
                y: &mut f64,
            ) -> bool;

            pub fn gnomonic_reverse(
                lat1: f64,
                lon1: f64,
                x: f64,
                y: f64,
                lat: &mut f64,
                lon: &mut f64,
            ) -> bool;

            pub fn geocentric_forward(
                lat: f64,
                lon: f64,
                h: f64,
                x: &mut f64,
                y: &mut f64,
                z: &mut f64,
            ) -> bool;

            pub fn geographiclib_version() -> *const c_char;

            pub fn compiler_version() -> *const c_char;
        }
    }
}

#[cfg(feature = "jsffi")]
mod wrappers {
    use dimensioned::si::{M, Meter};

    use crate::geographic::{DirectSolution, GeographicError, InverseSolution, Result};
    use crate::types::{XyPoint, XyzPoint};
    use crate::{DEG, Degree, GeoPoint};

    pub fn geodesic_direct(
        point1: &GeoPoint,
        azimuth: Degree<f64>,
        distance: Meter<f64>,
    ) -> Result<DirectSolution> {
        let out_js = ffi::geodesic_direct(
            point1.lat().value_unsafe,
            point1.lon().value_unsafe,
            azimuth.value_unsafe,
            distance.value_unsafe,
        );
        let out: ffi::DirectSolution = serde_wasm_bindgen::from_value(out_js)
            .map_err(|_| GeographicError::Json("DirectSolution".to_owned()))?;
        if out.ok {
            Ok(DirectSolution {
                arc_distance: out.a12 * DEG,
                point2: GeoPoint::new(out.lat2 * DEG, out.lon2 * DEG, None)?,
            })
        } else {
            Err(GeographicError::UnknownException)
        }
    }

    pub fn geodesic_inverse(point1: &GeoPoint, point2: &GeoPoint) -> Result<InverseSolution> {
        let out_js = ffi::geodesic_inverse(
            point1.lat().value_unsafe,
            point1.lon().value_unsafe,
            point2.lat().value_unsafe,
            point2.lon().value_unsafe,
        );
        let out: ffi::InverseSolution = serde_wasm_bindgen::from_value(out_js)
            .map_err(|_| GeographicError::Json("InverseSolution".to_owned()))?;
        if out.ok {
            Ok(InverseSolution {
                arc_distance: out.a12 * DEG,
                geo_distance: out.s12 * M,
                azimuth1: out.azi1 * DEG,
                azimuth2: out.azi2 * DEG,
            })
        } else {
            Err(GeographicError::UnknownException)
        }
    }

    pub fn gnomonic_forward(point0: &GeoPoint, point: &GeoPoint) -> Result<XyPoint> {
        let out_js = ffi::gnomonic_forward(
            point0.lat().value_unsafe,
            point0.lon().value_unsafe,
            point.lat().value_unsafe,
            point.lon().value_unsafe,
        );
        let out: ffi::XyPoint = serde_wasm_bindgen::from_value(out_js)
            .map_err(|_| GeographicError::Json("XyPoint".to_owned()))?;
        if out.ok {
            Ok(XyPoint {
                x: out.x * M,
                y: out.y * M,
            })
        } else {
            Err(GeographicError::UnknownException)
        }
    }

    pub fn gnomonic_reverse(point0: &GeoPoint, xypoint: &XyPoint) -> Result<GeoPoint> {
        let out_js = ffi::gnomonic_reverse(
            point0.lat().value_unsafe,
            point0.lon().value_unsafe,
            xypoint.x.value_unsafe,
            xypoint.y.value_unsafe,
        );
        let out: ffi::GeoPoint = serde_wasm_bindgen::from_value(out_js)
            .map_err(|_| GeographicError::Json("GeoPoint".to_owned()))?;
        if out.ok {
            Ok(GeoPoint::new(out.lat * DEG, out.lon * DEG, None)?)
        } else {
            Err(GeographicError::UnknownException)
        }
    }

    pub fn geocentric_forward(point: &GeoPoint) -> Result<XyzPoint> {
        let out_js =
            ffi::geocentric_forward(point.lat().value_unsafe, point.lon().value_unsafe, 0.0);
        let out: ffi::XyzPoint = serde_wasm_bindgen::from_value(out_js)
            .map_err(|_| GeographicError::Json("XyzPoint".to_owned()))?;
        if out.ok {
            Ok(XyzPoint {
                x: out.x * M,
                y: out.y * M,
                z: out.z * M,
            })
        } else {
            Err(GeographicError::UnknownException)
        }
    }

    pub fn geographiclib_version_str() -> String {
        ffi::geographiclib_version()
    }

    pub fn compiler_version_str() -> String {
        ffi::compiler_version()
    }

    mod ffi {
        use serde::Deserialize;
        use wasm_bindgen::prelude::*;

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct DirectSolution {
            pub ok: bool,
            pub lat2: f64,
            pub lon2: f64,
            pub a12: f64,
        }

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "GEO"])]
            pub fn geodesic_direct(lat1: f64, lon1: f64, azi1: f64, s12: f64) -> JsValue;
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct InverseSolution {
            pub ok: bool,
            pub s12: f64,
            pub azi1: f64,
            pub azi2: f64,
            pub a12: f64,
        }

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "GEO"])]
            pub fn geodesic_inverse(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> JsValue;
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct XyPoint {
            pub ok: bool,
            pub x: f64,
            pub y: f64,
        }

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "GEO"])]
            pub fn gnomonic_forward(lat0: f64, lon0: f64, lat: f64, lon: f64) -> JsValue;
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct GeoPoint {
            pub ok: bool,
            pub lat: f64,
            pub lon: f64,
        }

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "GEO"])]
            pub fn gnomonic_reverse(lat0: f64, lon0: f64, x: f64, y: f64) -> JsValue;
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct XyzPoint {
            pub ok: bool,
            pub x: f64,
            pub y: f64,
            pub z: f64,
        }

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "GEO"])]
            pub fn geocentric_forward(lat: f64, lon: f64, h: f64) -> JsValue;
        }

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "GEO"])]
            pub fn geographiclib_version() -> String;
        }

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = ["window", "GEO"])]
            pub fn compiler_version() -> String;
        }
    }
}

impl TryFrom<GeoPoint> for XyzPoint {
    type Error = GeographicError;

    fn try_from(value: GeoPoint) -> std::result::Result<Self, Self::Error> {
        geocentric_forward(&value)
    }
}

impl TryFrom<GeoPoint> for GeoAndXyzPoint {
    type Error = GeographicError;

    fn try_from(value: GeoPoint) -> std::result::Result<Self, Self::Error> {
        let xyz = geocentric_forward(&value)?;
        Ok(GeoAndXyzPoint { geo: value, xyz })
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use approx::assert_relative_eq;
    use dimensioned::si::M;

    use super::{geodesic_direct, geodesic_inverse, gnomonic_forward, gnomonic_reverse};
    use crate::measure::DEG;
    use crate::types::GeoPoint;

    #[test]
    fn test_geodesic_inverse() -> Result<()> {
        let point1 = GeoPoint::new(0.0 * DEG, 0.0 * DEG, None)?;
        let point2 = GeoPoint::new(5.0 * DEG, 5.0 * DEG, None)?;

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
        let point1 = GeoPoint::new(10.0 * DEG, -20.0 * DEG, None)?;
        let point2 = GeoPoint::new(30.0 * DEG, 40.0 * DEG, None)?;

        let inverse = geodesic_inverse(&point1, &point2)?;
        let result = geodesic_direct(&point1, inverse.azimuth1, inverse.geo_distance)?;
        // The direct result should reproduce the target point used to obtain
        // the inverse solution.
        assert_relative_eq!(result.point2, point2);
        Ok(())
    }

    #[test]
    fn test_gnomonic_forward() -> Result<()> {
        let point0 = GeoPoint::new(20.0 * DEG, -40.0 * DEG, None)?;
        let point = GeoPoint::new(17.0 * DEG, -35.0 * DEG, None)?;

        let result = gnomonic_forward(&point0, &point)?;
        // point's longitude is east of point0's
        assert!(result.x.value_unsafe > 0.0);
        // point's latitude is south of point0's
        assert!(result.y.value_unsafe < 0.0);
        Ok(())
    }

    #[test]
    fn test_gnomonic_reverse() -> Result<()> {
        let point0 = GeoPoint::new(20.0 * DEG, -40.0 * DEG, None)?;
        let point = GeoPoint::new(17.0 * DEG, -35.0 * DEG, None)?;

        let xypoint = gnomonic_forward(&point0, &point)?;
        let result = gnomonic_reverse(&point0, &xypoint)?;
        assert_relative_eq!(result, point);
        Ok(())
    }
}
