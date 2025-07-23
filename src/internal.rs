//! Exports intended for internal use only.
//!
//! These need to be exported for access from the main CLI and the
//! `integration-stub` binary, but they are not intended for use by external
//! code. This module's API may change without semantic versioning!

use dimensioned::si::Meter;

use crate::GeoPoint;
use crate::algorithm::{FromGeoPoints, intercept_distance_floor, karney_interception};
pub use crate::fit::PROFILE_VERSION;
use crate::geographic::geodesic_inverse;
pub use crate::geographic::{compiler_version_str, geodesic_direct, geographiclib_version_str};
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

#[cfg(any(feature = "cli", feature = "jsffi"))]
pub mod report {
    use std::cmp::min;
    use std::fmt::{Display, Write};

    use anyhow::Result;
    use dimensioned::si::Meter;

    use crate::ConversionInfo;

    pub fn conversion_report<T>(info: ConversionInfo) -> Result<String>
    where
        T: From<Meter<f64>> + Display,
    {
        // Build a report to print after the tracing span surrounding this function
        // has exited. If debug logging is enabled, this ensures the report to
        // STDOUT will be printed after all the tracing stuff.
        let mut r = String::new();
        match info.course_name {
            Some(name) => writeln!(
                &mut r,
                "Converted course {:?} of length {:.02}\n",
                name,
                T::from(info.total_distance)
            )?,
            None => writeln!(
                &mut r,
                "Converted an unnamed course of length {:.02}\n",
                T::from(info.total_distance)
            )?,
        };
        writeln!(
            &mut r,
            "Processed {} waypoints, {} of which {}{}",
            info.num_waypoints,
            info.course_points.len(),
            if info.course_points.len() == 1 {
                "was identified as a course point"
            } else {
                "were identified as course points"
            },
            if !info.course_points.is_empty() {
                ":"
            } else {
                ""
            },
        )?;
        let max_listing = 24usize;
        for i in 0..min(max_listing, info.course_points.len()) {
            let point = &info.course_points[i];
            writeln!(
                &mut r,
                "- {} at {:.02}{}",
                point.name,
                T::from(point.distance),
                if i == 0 { " along the course" } else { "" }
            )?;
        }
        if info.course_points.len() > max_listing {
            writeln!(&mut r, "(and others)")?;
        }
        Ok(r)
    }
}
