/*!
Abstract course elements
*/

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

pub struct CourseSet {
    pub courses: Vec<Course>,
}

pub struct CourseSetBuilder {
    courses: Vec<CourseBuilder>,
}

#[allow(clippy::new_without_default)]
impl CourseSetBuilder {
    pub fn new() -> Self {
        Self {
            courses: Vec::new(),
        }
    }

    pub fn create_course(&mut self) {
        self.courses.push(CourseBuilder::new());
    }

    pub fn current_mut(&mut self) -> Result<&mut CourseBuilder> {
        match self.courses.last_mut() {
            Some(course) => Ok(course),
            None => Err(CourseError::MissingCourse),
        }
    }

    pub fn build(self) -> CourseSet {
        let mut courses = vec![];
        for course_builder in self.courses {
            courses.push(course_builder.build());
        }
        CourseSet { courses }
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
            .last()
            .map(|x| x.cumulative_distance)
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

#[allow(clippy::new_without_default)]
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
                    cumulative_distance: last.cumulative_distance + distance_increment,
                })
            }

            None => self.records.push(Record {
                point,
                cumulative_distance: Meters(0.0),
            }),
        }
        Ok(())
    }

    pub fn build(self) -> Course {
        Course {
            records: self.records,
            name: self.name,
        }
    }
}

/// A course record.
#[derive(Clone)]
pub struct Record {
    /// Position including optional elevation.
    pub point: GeoPoint,

    /// Cumulative distance along the course.
    pub cumulative_distance: Meters<f64>,
}
