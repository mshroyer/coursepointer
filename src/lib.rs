//! CoursePointer is a CLI tool and library for computing Garmin FIT courses and
//! [course
//! points](https://support.garmin.com/en-US/?faq=aisqGZTLwH5LvbExSdO6L6) from
//! routes and waypoints.
//!
//! This crate helps waypoints (such as from a GPX file) appear in [Up
//! Ahead](https://support.garmin.com/en-US/?faq=lQMibRoY2I5Y4pP8EXgxv7) on
//! compatible devices, like Fenix watches and Edge bicycle computers.
//!
//! See the repo's
//! [README](https://github.com/mshroyer/coursepointer/blob/main/README.md)
//! for details about how this works and the problem it solves.
//!
//! # Binary
//!
//! The `coursepointer` binary takes as input a GPX file containing a single
//! route or track, and outputs a Garmin FIT course file in which those of the
//! GPX's waypoints that are within a threshold distance of the route/track have
//! been converted to FIT course points.  Also see
//! [README](https://github.com/mshroyer/coursepointer/blob/main/README.md)
//! for more information about using the binary.
//!
//! # Library
//!
//! The library crate contains the bulk of the binary's logic.  It builds on top
//! of [GeographicLib](https://geographiclib.sourceforge.io/C++/doc/index.html)
//! to compute the interception points and course distances of waypoints near
//! routes or tracks, then encodes this all as a FIT course.
//!
//! It also provides, in the [`course`] module, lower-lever builders to compose
//! courses and course points out of routes and waypoints programmatically.
//!
//! # Feature flags
//!
//! - `cli` enables the additional dependencies needed by the CLI.  This needs
//!   to be explicitly enabled if installing the CLI with `cargo install`.
//!
//! - `rayon` enables computing course points in parallel using [rayon](https://docs.rs/rayon/latest/rayon/).
//!   This improves the binary's runtime significantly in stress tests, and at
//!   least doesn't hurt in more typical cases, on my machine.  Enabled by
//!   default.
//!
//! - `full-geolib` causes cxx_build to build all GeographicLib sources instead
//!   of a hand-picked subset.  This is mainly useful when experimenting with
//!   new FFI additions, otherwise it simply slows the build down.

mod algorithm;
pub mod course;
mod fit;
mod geographic;
mod gpx;
#[doc(hidden)]
pub mod internal;
mod measure;
mod point_type;
mod types;

use std::convert::Infallible;
use std::io::{BufRead, Write};

use dimensioned::si::Meter;
pub use fit::{FitCourseOptions, FitEncodeError};
use thiserror::Error;
use tracing::{Level, debug, span};

use crate::algorithm::AlgorithmError;
use crate::course::{
    Course, CourseError, CoursePoint, CourseSet, CourseSetBuilder, CourseSetOptions,
};
pub use crate::fit::{CourseFile, CoursePointType, Sport};
use crate::geographic::GeographicError;
use crate::gpx::{GpxItem, GpxReader};
pub use crate::measure::{DEG, Degree};
use crate::point_type::{GpxCreator, get_course_point_type, get_gpx_creator};
pub use crate::types::{GeoPoint, TypeError};

/// An error in a high-level library operation
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum CoursePointerError {
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error("GPX processing error")]
    Gpx(#[from] gpx::GpxError),
    #[error("GPX items received in unexpected order")]
    GpxOrder,
    #[error("Course error")]
    Course(#[from] CourseError),
    #[error("Unexpected number of courses (tracks or routes) in input: {0}")]
    CourseCount(usize),
    #[error("Course does not contain any records")]
    EmptyRecords,
    #[error("FIT encoding error")]
    FitEncode(#[from] FitEncodeError),
    #[error("Core type error")]
    Type(#[from] TypeError),
    #[error("Geographic calculation error")]
    Geographic(#[from] GeographicError),
    #[error("Infallible")]
    Infallible(#[from] Infallible),
    #[error("Algorithm")]
    Algorithm(#[from] AlgorithmError),
}

pub type Result<T> = std::result::Result<T, CoursePointerError>;

/// Summarizes the result of converting GPX into a FIT course
///
/// The result of a successful invocation of [`convert_gpx_to_fit`].
#[derive(Debug)]
pub struct ConversionInfo {
    /// The name of the course that was converted, if given
    pub course_name: Option<String>,

    /// The total distance of the course
    pub total_distance: Meter<f64>,

    /// The total number of waypoints specified in the input, including those
    /// not identified as course points
    pub num_waypoints: usize,

    /// The set of course points identified
    pub course_points: Vec<CoursePoint>,
}

/// Convert GPX into a FIT course file
///
/// The `BufRead` bound on `gpx_input` is required by quick_xml, but this
/// doesn't imply by contrast this function will construct its own `BufWrite`
/// for the output. `fit_output` should probably also be given as a buffered
/// `Write`.
///
/// The GPX input is required to contain exactly one route or track, and may
/// contain zero or more waypoints.
///
/// The `fit_speed` parameter sets a speed for placing timestamps along the FIT
/// course.  On compatible devices, this will determine the speed of the
/// "virtual partner".
///
/// This combines the behavior of [`read_gpx`] and [`write_fit_course`].
pub fn convert_gpx_to_fit<R: BufRead, W: Write>(
    gpx_input: R,
    fit_output: W,
    course_options: CourseSetOptions,
    fit_options: FitCourseOptions,
) -> Result<ConversionInfo> {
    let mut course_set = read_gpx(course_options, gpx_input)?;
    let course = course_set.courses.remove(0);
    write_fit_course(&course, fit_output, fit_options)?;

    Ok(ConversionInfo {
        course_name: course.name.clone(),
        total_distance: course.records.last().unwrap().cumulative_distance,
        num_waypoints: course_set.num_waypoints,
        course_points: course.course_points,
    })
}

/// Read a GPX file into a [`CourseSet`]
pub fn read_gpx<R: BufRead>(options: CourseSetOptions, gpx_input: R) -> Result<CourseSet> {
    let mut builder = CourseSetBuilder::new(options);

    {
        let span = span!(Level::DEBUG, "read_input");
        let _guard = span.enter();
        let mut num_items = 0usize;
        let mut skipped_items = 0usize;
        let mut creator = GpxCreator::Unknown;
        let gpx_reader = GpxReader::from_reader(gpx_input);
        for item in gpx_reader {
            let item = item?;
            num_items += 1;
            match item {
                GpxItem::Creator(s) => {
                    creator = get_gpx_creator(s.as_str());
                }

                GpxItem::TrackOrRoute => {
                    builder.add_route();
                }

                GpxItem::TrackOrRouteName(name) => {
                    builder
                        .last_route_mut()
                        .ok_or(CoursePointerError::GpxOrder)?
                        .with_name(name);
                }

                GpxItem::TrackOrRoutePoint(p) => {
                    builder
                        .last_route_mut()
                        .ok_or(CoursePointerError::GpxOrder)?
                        .with_route_point(p);
                }

                GpxItem::Waypoint(wpt) => {
                    builder.add_waypoint(wpt.point, get_course_point_type(creator, &wpt), wpt.name);
                }

                _ => {
                    skipped_items += 1;
                }
            }
        }
        debug!(
            "Read {} GpxItem(s), matching {}",
            num_items,
            num_items - skipped_items
        );
    }

    if builder.num_routes() != 1usize {
        return Err(CoursePointerError::CourseCount(builder.num_routes()));
    }
    Ok(builder.build()?)
}

/// Write a single [`Course`] into a GPX course file
///
/// The `fit_speed` parameter sets a speed for placing timestamps along the FIT
/// course.  On compatible devices, this will determine the speed of the
/// "virtual partner".
pub fn write_fit_course<W: Write>(
    course: &Course,
    fit_output: W,
    options: FitCourseOptions,
) -> Result<()> {
    let course_file = CourseFile::new(course, options);
    course_file.encode(fit_output)?;
    Ok(())
}

/// CXX Generated FFI for GeographicLib
///
/// This currently has to be inline in lib.rs because non-inline mods in proc
/// macro input are unstable: <https://github.com/rust-lang/rust/issues/54727>
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

        fn geocentric_forward(
            lat: f64,
            lon: f64,
            h: f64,
            x: &mut f64,
            y: &mut f64,
            z: &mut f64,
        ) -> Result<()>;

        fn geographiclib_version() -> &'static str;

        fn compiler_version() -> &'static str;
    }
}
