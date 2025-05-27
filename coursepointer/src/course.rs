//! Abstract course elements

use coretypes::GeoPoint;
use coretypes::measure::Meters;
use geographic::{GeographicError, solve_inverse};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CourseError {
    #[error("Geographic calculation error")]
    Geographic(#[from] GeographicError),
    #[error("Attempt to access a missing course")]
    MissingCourse,
}

type Result<T> = std::result::Result<T, CourseError>;

pub struct CourseSetBuilder {
    pub courses: Vec<CourseBuilder>,
}

impl CourseSetBuilder {
    pub fn new() -> Self {
        Self {
            courses: Vec::new(),
        }
    }

    pub fn create_course(&mut self) {
        self.courses.push(CourseBuilder::new());
    }

    pub fn current(&self) -> Result<&CourseBuilder> {
        match self.courses.last() {
            Some(course) => Ok(course),
            None => Err(CourseError::MissingCourse),
        }
    }

    pub fn current_mut(&mut self) -> Result<&mut CourseBuilder> {
        match self.courses.last_mut() {
            Some(course) => Ok(course),
            None => Err(CourseError::MissingCourse),
        }
    }
}

impl Default for CourseSetBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// An abstract course.
///
/// Contains records defining the segments of the course on the WGS84 ellipsoid,
/// as well as each record's geodesic distance along the entire course. May
/// optionally contain elevation data.
pub struct Course {
    /// The records that define the course, in order of physical traversal.
    pub records: Vec<Record>,

    /// The name of the course, if given.
    pub name: Option<String>,
}

impl Course {
    /// The total distance of the course.
    pub fn total_distance(&self) -> Meters<f64> {
        self.records
            .iter()
            .last()
            .map(|x| x.distance)
            .unwrap_or(Meters(0.0))
    }

    /// Checks whether elevation data is available in this course.
    pub fn has_elevation(&self) -> bool {
        self.records.iter().all(|r| r.point.ele().is_some())
    }
}

pub struct CourseBuilder {
    records: Vec<Record>,
    name: Option<String>,
}

impl CourseBuilder {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            name: None,
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub fn add_record(&mut self, point: GeoPoint) -> Result<()> {
        match self.records.iter().last() {
            Some(last) => {
                // TODO: Investigate using elevation-corrected distances
                let distance_increment = solve_inverse(&last.point, &point)?.geo_distance;
                self.records.push(Record {
                    point,
                    distance: last.distance + distance_increment,
                })
            }

            None => self.records.push(Record {
                point,
                distance: Meters(0.0),
            }),
        }
        Ok(())
    }

    pub fn records_len(&self) -> usize {
        self.records.len()
    }

    pub fn iter_records(&self) -> impl Iterator<Item = &Record> {
        self.records.iter()
    }

    pub fn total_distance(&self) -> Meters<f64> {
        self.records
            .iter()
            .last()
            .map(|x| x.distance)
            .unwrap_or(Meters(0.0))
    }

    pub fn get_name(&self) -> &str {
        match &self.name {
            Some(name) => name.as_ref(),
            None => "Untitled course",
        }
    }

    pub fn build(&self) -> Course {
        Course {
            records: self.records.clone(),
            name: self.name.clone(),
        }
    }
}

impl Default for CourseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct Record {
    pub point: GeoPoint,
    pub distance: Meters<f64>,
}
