use std::io::{BufRead, Write};

use chrono::Utc;
use coretypes::TypeError;
use coretypes::measure::KilometersPerHour;
use thiserror::Error;

use crate::gpx::GpxItem;

pub mod algorithm;
pub mod course;
pub mod fit;
pub mod gpx;

pub use fit::{CourseFile, PROFILE_VERSION};
pub use gpx::GpxReader;

use crate::course::{CourseError, CourseSetBuilder};

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

    let course_set = builder.build();
    if course_set.courses.len() != 1usize {
        return Err(CoursePointerError::CourseCount(course_set.courses.len()));
    }
    let course = course_set.courses.first().unwrap();
    let course_file = CourseFile::new(course, Utc::now(), KilometersPerHour(20.0).into());
    course_file.encode(fit_output)?;

    Ok(())
}
