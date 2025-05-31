use approx::{AbsDiffEq, RelativeEq, abs_diff_eq, relative_eq};
use thiserror::Error;

use crate::measure::{Degrees, Meters};

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
            return Err(TypeError::GeoPointInvariant(
                GeoPointDimension::Latitude,
                lat,
            ));
        }
        if lon.0 < -180.0 || lon.0 > 180.0 {
            return Err(TypeError::GeoPointInvariant(
                GeoPointDimension::Longitude,
                lon,
            ));
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

impl Default for GeoPoint {
    fn default() -> GeoPoint {
        GeoPoint {
            lat: Degrees(0.0),
            lon: Degrees(0.0),
            ele: None,
        }
    }
}

impl AbsDiffEq for GeoPoint {
    type Epsilon = f64;

    fn default_epsilon() -> Self::Epsilon {
        f64::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        abs_diff_eq!(self.lat.0, other.lat.0, epsilon = epsilon)
            && abs_diff_eq!(self.lon.0, other.lon.0, epsilon = epsilon)
    }
}

impl RelativeEq for GeoPoint {
    fn default_max_relative() -> Self::Epsilon {
        0.000_000_000_000_001
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        relative_eq!(
            self.lat().0,
            other.lat().0,
            epsilon = epsilon,
            max_relative = max_relative
        ) && relative_eq!(
            self.lon().0,
            other.lon().0,
            epsilon = epsilon,
            max_relative = max_relative
        )
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct GeoSegment {
    pub point1: GeoPoint,
    pub point2: GeoPoint,
    pub geo_distance: Meters<f64>,
    pub azimuth1: Degrees<f64>,
}

/// A point on a 2D projection.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct XYPoint {
    pub x: Meters<f64>,
    pub y: Meters<f64>,
}

impl Default for XYPoint {
    fn default() -> Self {
        Self {
            x: Meters(0.0),
            y: Meters(0.0),
        }
    }
}

/// Instantiate a `GeoPoint` with a tuple-like syntax, optionally including an
/// elevation in meters.
#[macro_export]
macro_rules! geo_point {
    ( $lat:expr, $lon:expr ) => {
        $crate::coretypes::GeoPoint::new(
            $crate::measure::Degrees($lat),
            $crate::measure::Degrees($lon),
            None,
        )?
    };
    ( $lat:expr, $lon:expr, $ele:expr ) => {
        $crate::coretypes::GeoPoint::new(
            $crate::measure::Degrees($lat),
            $crate::measure::Degrees($lon),
            Some(Meters($ele)),
        )?
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
