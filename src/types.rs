use approx::{AbsDiffEq, RelativeEq, abs_diff_eq, relative_eq};
use dimensioned::si::{M, Meter};
use thiserror::Error;

use crate::measure::{DEG, Degree};

/// A type invariant error for basic types like [`GeoPoint`]
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum TypeError {
    #[error("Geographic point invariant: invalid value {1:?} for {0:?}")]
    GeoPointInvariant(GeoPointDimension, Degree<f64>),
    #[error("Casting between numeric types")]
    NumericCast,
}

pub type Result<T> = std::result::Result<T, TypeError>;

/// A point on the surface of the WGS84 ellipsoid
///
/// Enforces valid latitude and longitude values as type invariants.  May
/// optionally contain an elevation in addition to latitude and longitude.
///
/// # Example
///
/// ```
/// use dimensioned::si::M;
/// use coursepointer::{DEG, GeoPoint};
/// let point = GeoPoint::new(37.45 * DEG, -122.11 * DEG, Some(10.0 * M));
/// ```
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct GeoPoint {
    lat: Degree<f64>,
    lon: Degree<f64>,
    ele: Option<Meter<f64>>,
}

#[derive(Debug)]
pub enum GeoPointDimension {
    Latitude,
    Longitude,
    Elevation,
}

impl GeoPoint {
    pub fn new(lat: Degree<f64>, lon: Degree<f64>, ele: Option<Meter<f64>>) -> Result<GeoPoint> {
        if lat.value_unsafe < -90.0 || lat.value_unsafe > 90.0 {
            return Err(TypeError::GeoPointInvariant(
                GeoPointDimension::Latitude,
                lat,
            ));
        }
        if lon.value_unsafe < -180.0 || lon.value_unsafe > 180.0 {
            return Err(TypeError::GeoPointInvariant(
                GeoPointDimension::Longitude,
                lon,
            ));
        }
        Ok(Self { lat, lon, ele })
    }

    /// Get point latitude
    pub fn lat(&self) -> Degree<f64> {
        self.lat
    }

    /// Get point longitude
    pub fn lon(&self) -> Degree<f64> {
        self.lon
    }

    /// Get point elevation, if known
    pub fn ele(&self) -> Option<Meter<f64>> {
        self.ele
    }
}

impl Default for GeoPoint {
    fn default() -> GeoPoint {
        GeoPoint {
            lat: 0.0 * DEG,
            lon: 0.0 * DEG,
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
        abs_diff_eq!(
            self.lat.value_unsafe,
            other.lat.value_unsafe,
            epsilon = epsilon
        ) && abs_diff_eq!(
            self.lon.value_unsafe,
            other.lon.value_unsafe,
            epsilon = epsilon
        )
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
            self.lat().value_unsafe,
            other.lat().value_unsafe,
            epsilon = epsilon,
            max_relative = max_relative
        ) && relative_eq!(
            self.lon().value_unsafe,
            other.lon().value_unsafe,
            epsilon = epsilon,
            max_relative = max_relative
        )
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct XyzPoint {
    pub x: Meter<f64>,
    pub y: Meter<f64>,
    pub z: Meter<f64>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct GeoAndXyzPoint {
    pub geo: GeoPoint,
    pub xyz: XyzPoint,
}

/// A geodesic segment of a course.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct GeoSegment<'a, P>
where
    P: HasGeoPoint,
{
    /// The segment's start point.
    pub start: &'a P,

    /// The segment's end point.
    pub end: &'a P,

    /// The geodesic length of the segment.
    pub geo_length: Meter<f64>,

    /// The azimuth of the segment as measured at its start point.
    pub start_azimuth: Degree<f64>,
}

pub trait HasGeoPoint: PartialEq + Copy {
    fn geo(&self) -> &GeoPoint;
}

impl HasGeoPoint for GeoPoint {
    fn geo(&self) -> &GeoPoint {
        self
    }
}

impl HasGeoPoint for GeoAndXyzPoint {
    fn geo(&self) -> &GeoPoint {
        &self.geo
    }
}

pub trait HasXyzPoint {
    fn xyz(&self) -> &XyzPoint;
}

impl HasXyzPoint for XyzPoint {
    fn xyz(&self) -> &XyzPoint {
        self
    }
}

impl<'a> HasXyzPoint for &'a XyzPoint {
    fn xyz(&self) -> &'a XyzPoint {
        self
    }
}

impl HasXyzPoint for GeoAndXyzPoint {
    fn xyz(&self) -> &XyzPoint {
        &self.xyz
    }
}

/// A point on a 2D projection.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct XyPoint {
    pub x: Meter<f64>,
    pub y: Meter<f64>,
}

impl Default for XyPoint {
    fn default() -> Self {
        Self {
            x: 0.0 * M,
            y: 0.0 * M,
        }
    }
}

/// Instantiate a [`GeoPoint`] with a tuple-like syntax, optionally including an
/// elevation in meters.
#[doc(hidden)]
#[macro_export]
macro_rules! geo_point {
    ( $lat:expr, $lon:expr ) => {
        $crate::types::GeoPoint::new(
            $lat * $crate::measure::DEG,
            $lon * $crate::measure::DEG,
            None,
        )
    };
    ( $lat:expr, $lon:expr, $ele:expr ) => {
        $crate::types::GeoPoint::new(
            $lat * $crate::measure::DEG,
            $lon * $crate::measure::DEG,
            Some($ele * ::dimensioned::si::M),
        )
    };
}

/// Instantiate a vec of `GeoPoint` with tuple-like syntax, optionally including
/// an elevation in meters.
#[doc(hidden)]
#[macro_export]
macro_rules! geo_points {
    ( $( ( $lat:expr, $lon:expr $(, $ele:expr )? $(,)? ) ),* $(,)? ) => {
        (|| -> $crate::types::Result<Vec<GeoPoint>> { Ok(vec![ $( $crate::geo_point!($lat, $lon $( , $ele )?)? ),* ]) })()
    };
}
