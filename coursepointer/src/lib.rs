use std::fs::File;
use std::path::Path;
use chrono::Utc;
use thiserror::Error;

use coretypes::TypeError;
use coretypes::measure::KilometersPerHour;
use crate::gpx::GpxItem;

pub mod fit;
pub mod gpx;
pub mod course;

pub use gpx::GpxReader;
pub use fit::CourseFile;
pub use fit::PROFILE_VERSION;
use crate::course::{CourseError, CourseSet};

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

pub fn convert_gpx(gpx_input: &Path, fit_output: &Path) -> Result<()> {
    let mut course_set = CourseSet::new();
    let gpx_reader = GpxReader::from_path(gpx_input)?;
    for item in gpx_reader {
        let item = item?;
        match item {
            GpxItem::Track => {
                course_set.create_course();
            }

            GpxItem::TrackName(name) => {
                course_set.current_mut()?.set_name(name);
            }

            GpxItem::TrackPoint(p) => {
                course_set.current_mut()?.add_record(p)?;
            }
            
            _ => (),
        }       
    }

    if course_set.courses.len() != 1usize {
        return Err(CoursePointerError::CourseCount(course_set.courses.len()));
    }

    let mut fit_file = File::create(fit_output)?;
    let course_file = CourseFile::new(
        course_set.current()?,
        Utc::now(),
        KilometersPerHour(20.0).into(),
    );
    course_file.encode(&mut fit_file)?;

    Ok(())
}
