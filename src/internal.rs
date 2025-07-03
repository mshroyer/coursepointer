//! Exports intended for internal use only.
//!
//! These need to be exported for access from the main CLI and the
//! `integration-stub` binary, but they are not intended for use by external
//! code. This module's API may change without semantic versioning!

use dimensioned::si::Meter;

use crate::GeoPoint;
use crate::algorithm::{FromGeoPoints, intercept_distance_floor, karney_interception};
pub use crate::ffi::{compiler_version, geographiclib_version};
pub use crate::fit::PROFILE_VERSION;
use crate::geographic::geodesic_inverse;
pub use crate::measure::{Kilometer, Mile};
use crate::types::{GeoAndXyzPoint, GeoSegment};

/// Print debugging info about an intercept scenario
pub fn debug_intercept(s1: &GeoPoint, s2: &GeoPoint, p: &GeoPoint) -> crate::Result<()> {
    println!("s1: {s1:?}");
    println!("s2: {s2:?}");
    println!("p:  {p:?}");

    fn p2p_dist(a: &GeoPoint, b: &GeoPoint) -> crate::Result<Meter<f64>> {
        Ok(geodesic_inverse(a, b)?.geo_distance)
    }
    println!("s1 -- s2: {}", p2p_dist(s1, s2)?);
    println!("s1 -- p:  {}", p2p_dist(s1, p)?);
    println!("s2 -- p:  {}", p2p_dist(s2, p)?);

    let s1_xyz: GeoAndXyzPoint = (*s1).try_into()?;
    let s2_xyz: GeoAndXyzPoint = (*s2).try_into()?;
    let seg = GeoSegment::<GeoAndXyzPoint>::from_geo_points(&s1_xyz, &s2_xyz)?;
    let p_xyz = GeoAndXyzPoint::try_from(*p)?;

    let intercept_point = karney_interception(&seg, &p_xyz)?;
    let intercept_dist = geodesic_inverse(&intercept_point, p)?.geo_distance;

    println!("intercept_dist: {intercept_dist}");
    println!(
        "         floor: {}",
        intercept_distance_floor(&seg, &p_xyz)?
    );

    Ok(())
}
