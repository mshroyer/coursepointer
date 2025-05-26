pub mod measure;

use thiserror::Error;

use measure::Degrees;
use measure::Meters;

#[derive(Error, Debug)]
pub enum TypeError {
    #[error("geographic point invariant: invalid value {1:?} for {0:?}")]
    GeoPointInvariant(GeoPointDimension, Degrees<f64>),
}

type Result<T> = std::result::Result<T, TypeError>;

/// A point on the surface of the WGS84 ellipsoid.
/// 
/// Enforces valid latitude and longitude values as type invariants.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct GeoPoint {
    lat: Degrees<f64>,
    lon: Degrees<f64>,
    ele: Option<Meters<f64>>,
}

#[derive(Debug)]
pub enum GeoPointDimension {
    Latitude,
    Longitude,
    Elevation,
}

impl GeoPoint {
    pub fn new(lat: Degrees<f64>, lon: Degrees<f64>, ele: Option<Meters<f64>>) -> Result<GeoPoint> {
        if lat.0 < -90.0 || lat.0 > 90.0 {
            return Err(TypeError::GeoPointInvariant(GeoPointDimension::Latitude, lat));
        }
        if lon.0 < -180.0 || lon.0 > 180.0 {
            return Err(TypeError::GeoPointInvariant(GeoPointDimension::Longitude, lon));
        }
        Ok(Self { lat, lon, ele })
    }
    
    /// Get point latitude
    #[inline(always)]
    pub fn lat(&self) -> Degrees<f64> {
        self.lat
    }
    
    /// Get point longitude
    #[inline(always)]
    pub fn lon(&self) -> Degrees<f64> {
        self.lon
    }
    
    /// Get point elevation, if known
    #[inline(always)]
    pub fn ele(&self) -> Option<Meters<f64>> {
        self.ele
    }
}

/// Instantiate a `GeoPoint` with a tuple-like syntax, optionally including an
/// elevation in meters.
#[macro_export]
macro_rules! geo_point {
    ( $lat:expr, $lon:expr ) => {
        $crate::GeoPoint::new(Degrees($lat), Degrees($lon), None)?
    };
    ( $lat:expr, $lon:expr, $ele:expr ) => {
        $crate::GeoPoint::new(Degrees($lat), Degrees($lon), Some(Meters($ele)))?
    };
}

/// Instantiate a vec of `GeoPoint` with tuple-like syntax, optionally including
/// an elevation in meters.
#[macro_export]
macro_rules! geo_points {
    ( $( ( $lat:expr, $lon:expr $(, $ele:expr )? $(,)? ) ),* $(,)? ) => {
        vec![ $( $crate::geo_point!($lat, $lon $( , $ele )?) ),* ]
    };
}
