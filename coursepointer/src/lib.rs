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
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("GPX error: {0}")]
    Gpx(#[from] gpx::GpxError),
    #[error(transparent)]
    Course(#[from] CourseError),
    #[error("unexpected number of courses: {0}")]
    CourseCount(usize),
    #[error("FIT encode error: {0}")]
    FitEncode(#[from] fit::FitEncodeError),
    #[error("type invariant error: {0}")]
    TypeError(#[from] TypeError),
}

type Result<T> = std::result::Result<T, CoursePointerError>;

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
        course_set.courses.iter().last().unwrap(),
        Utc::now(),
        KilometersPerHour(20.0).into(),
    );
    course_file.encode(&mut fit_file)?;

    Ok(())
}
