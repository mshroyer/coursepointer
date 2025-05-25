pub mod measure;

use thiserror::Error;

use measure::Degrees;
use measure::Meters;

#[derive(Debug)]
pub enum GeoPointDimension {
    Latitude,
    Longitude,
    Elevation,
}

#[derive(Error, Debug)]
pub enum TypeError {
    #[error("geographic point invariant: invalid value {1:?} for {0:?}")]
    GeoPointInvariant(GeoPointDimension, Degrees<f64>),
}

type Result<T> = std::result::Result<T, TypeError>;

/// A point on the surface of the WGS84 ellipsoid.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct GeoPoint {
    lat: Degrees<f64>,
    lon: Degrees<f64>,
    ele: Option<Meters<f64>>,
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
    pub fn lat(&self) -> Degrees<f64> {
        self.lat
    }
    
    /// Get point longitude
    pub fn lon(&self) -> Degrees<f64> {
        self.lon
    }
    
    /// Get point elevation, if known
    pub fn ele(&self) -> Option<Meters<f64>> {
        self.ele
    }
}
