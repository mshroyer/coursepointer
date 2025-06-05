//! A CLI tool and library for computing FIT course points from GPX.
//!
//! Builds on
//! [GeographicLib](https://geographiclib.sourceforge.io/C++/doc/index.html) to
//! compute the interception points and course distances of waypoints near
//! routes or tracks imported from a GPX file, then encodes this as a FIT course
//! for navigation on a Garmin device such as an Edge bicycle computer or a
//! Fenix watch.
//!
//! See the [`convert_gpx`] function, which is used by the CLI, for the main
//! entry point into the library.
//!
//! # Feature flags
//!
//! - `cli` enables the additional dependencies needed by the CLI
//! - `full-geolib` causes cxx_build to build all GeographicLib sources instead
//!   of a hand-picked subset

mod algorithm;
mod course;
mod fit;
mod geographic;
mod gpx;
mod measure;
pub mod testonly;
mod types;

use std::io::{BufRead, Write};

use chrono::Utc;
use dimensioned::f64prefixes::KILO;
use dimensioned::si::M;
use dimensioned::si::f64consts::HR;
pub use fit::FitEncodeError;
use thiserror::Error;

use crate::course::{CourseError, CourseSetBuilder};
use crate::fit::CourseFile;
use crate::gpx::{GpxItem, GpxReader};
use crate::types::TypeError;

#[derive(Error, Debug)]
pub enum CoursePointerError {
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error("GPX processing error")]
    Gpx(#[from] gpx::GpxError),
    #[error("Course error")]
    Course(#[from] CourseError),
    #[error("Unexpected number of courses (tracks or routes) in input: {0}")]
    CourseCount(usize),
    #[error("FIT encoding error")]
    FitEncode(#[from] fit::FitEncodeError),
    #[error("Core type error")]
    Type(#[from] TypeError),
}

pub type Result<T> = std::result::Result<T, CoursePointerError>;

/// Convert GPX into a FIT course file.
///
/// The `BufRead` bound on `gpx_input` is required internally by quick_xml, but
/// this doesn't imply by contrast this function will construct its own
/// `BufWrite` for the output. `fit_output` should probably also be given as a
/// buffered `Write`.
pub fn convert_gpx<R: BufRead, W: Write>(gpx_input: R, fit_output: W) -> Result<()> {
    let mut builder = CourseSetBuilder::new();
    let gpx_reader = GpxReader::from_reader(gpx_input);
    for item in gpx_reader {
        let item = item?;
        match item {
            GpxItem::TrackOrRoute => {
                builder.create_course();
            }

            GpxItem::TrackOrRouteName(name) => {
                builder.current_mut()?.set_name(name);
            }

            GpxItem::TrackOrRoutePoint(p) => {
                builder.current_mut()?.add_route_point(p)?;
            }

            GpxItem::Waypoint(wpt) => {
                builder.add_waypoint(wpt);
            }

            _ => (),
        }
    }

    let course_set = builder.build()?;
    if course_set.courses.len() != 1usize {
        return Err(CoursePointerError::CourseCount(course_set.courses.len()));
    }
    let course = course_set.courses.first().unwrap();
    let speed = 20.0 * (KILO * M) / HR;
    let course_file = CourseFile::new(course, Utc::now(), speed);
    course_file.encode(fit_output)?;

    Ok(())
}

/// CXX Generated FFI for GeographicLib
///
/// This currently has to be inline in lib.rs because non-inline mods are
/// unstable in proc macro input:
/// <https://github.com/rust-lang/rust/issues/54727>
#[allow(clippy::too_many_arguments)]
#[cxx::bridge(namespace = "CoursePointer")]
mod ffi {
    unsafe extern "C++" {
        include!("coursepointer/include/shim.hpp");

        fn geodesic_direct(
            lat1: f64,
            lon1: f64,
            az1: f64,
            s12: f64,
            lat2: &mut f64,
            lon2: &mut f64,
        ) -> Result<f64>;

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

        fn gnomonic_reverse(
            lat1: f64,
            lon1: f64,
            x: f64,
            y: f64,
            lat: &mut f64,
            lon: &mut f64,
        ) -> Result<()>;
    }
}
