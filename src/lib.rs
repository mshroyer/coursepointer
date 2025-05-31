//! The main library crate for CoursePointer.
//!
//! See the [`convert_gpx`] function, which is used by the CLI, for the main
//! entry point into the library.
//!
//! This contains the bulk of the application logic. But there are two other
//! crates to know about:
//!
//! - [`geographic`] builds the C++ version of GeographicLib and provides FFI.
//! - [`coretypes`] provides simple units of measure and other types used by
//!   both this crate and [`geographic`] to avoid a circular dependency.

pub mod algorithm;
mod coretypes;
mod course;
mod fit;
mod geographic;
mod gpx;
mod measure;
pub mod testonly;

use std::io::{BufRead, Write};

use chrono::Utc;
pub use fit::FitEncodeError;
use thiserror::Error;

use crate::coretypes::TypeError;
use crate::course::{CourseError, CourseSetBuilder};
use crate::fit::CourseFile;
use crate::gpx::{GpxItem, GpxReader};
use crate::measure::KilometersPerHour;

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
    let course_file = CourseFile::new(course, Utc::now(), KilometersPerHour(20.0).into());
    course_file.encode(fit_output)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[cxx::bridge(namespace = "CoursePointer")]
mod ffi {
    unsafe extern "C++" {
        include!("coursepointer/include/shim.hpp");

        fn geodesic_inverse_with_azimuth(
            lat1: f64,
            lon1: f64,
            lat2: f64,
            lon2: f64,
            s12: &mut f64,
            azi1: &mut f64,
            azi2: &mut f64,
        ) -> Result<f64>;

        fn geodesic_direct(
            lat1: f64,
            lon1: f64,
            az1: f64,
            s12: f64,
            lat2: &mut f64,
            lon2: &mut f64,
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
