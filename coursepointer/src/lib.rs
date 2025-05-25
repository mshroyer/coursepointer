pub mod fit;
pub mod gpx;

use std::fs::File;
use std::path::Path;
use chrono::Utc;
use thiserror::Error;

pub use gpx::GpxReader;
pub use fit::CourseFile;
pub use fit::PROFILE_VERSION;
use coretypes::measure::KilometersPerHour;
use geographic::SurfacePoint;
use crate::gpx::GpxItem;

#[derive(Error, Debug)]
pub enum CoursePointerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("GPX error: {0}")]
    Gpx(#[from] gpx::GpxError),
    #[error("FIT encode error: {0}")]
    FitEncode(#[from] fit::FitEncodeError),
}

type Result<T> = std::result::Result<T, CoursePointerError>;

pub fn convert_gpx(gpx_input: &Path, fit_output: &Path) -> Result<()> {
    let gpx_reader = GpxReader::from_path(gpx_input)?;
    let mut course_name = "Untitled course".to_string();
    let mut track_points = vec![];
    for item in gpx_reader {
        let item = item?;
        match item {
            GpxItem::TrackName(name) => {
                course_name = name;
            }

            GpxItem::TrackPoint(p) => {
                track_points.push(p);               
            }
            
            _ => (),
        }       
    }
    
    let mut fit_file = File::create(fit_output)?;
    let mut course = CourseFile::new(
        course_name,
        Utc::now(),
        KilometersPerHour(20.0).into(),
    );
    for track_point in track_points {
        course.add_record(SurfacePoint::new(track_point.lat.0, track_point.lon.0))?
    }
    course.encode(&mut fit_file)?;

    Ok(())
}
