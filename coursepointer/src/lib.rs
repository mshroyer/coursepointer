use std::io::{BufWriter, Write};
use std::path::Path;

use chrono::Utc;
use coretypes::TypeError;
use coretypes::measure::KilometersPerHour;
use thiserror::Error;

use crate::gpx::GpxItem;

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

pub fn convert_gpx<W: Write>(gpx_input: &Path, fit_output: &mut BufWriter<W>) -> Result<()> {
    let mut builder = CourseSetBuilder::new();
    let gpx_reader = GpxReader::from_path(gpx_input)?;
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
                builder.current_mut()?.add_record(p)?;
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
