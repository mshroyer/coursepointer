//! Abstract course elements

use coretypes::measure::Meters;
use coretypes::{GeoPoint, GeoSegment};
use geographic::GeographicError;
use thiserror::Error;

use crate::algorithm::FromGeoPoints;
use crate::fit::CoursePointType;
use crate::gpx::Waypoint;

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
    waypoints: Vec<Waypoint>,
}

#[allow(clippy::new_without_default)]
impl CourseSetBuilder {
    pub fn new() -> Self {
        Self {
            courses: Vec::new(),
            waypoints: Vec::new(),
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

    pub fn add_waypoint(&mut self, waypoint: Waypoint) {
        self.waypoints.push(waypoint);
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

    /// The course points that have been located on the course.
    pub course_points: Vec<CoursePoint>,

    /// The name of the course, if given.
    pub name: Option<String>,

    /// The number of repeated points that were skipped.
    num_repeated_points_skipped: usize,
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
    segments: Vec<GeoSegment>,
    prev_point: Option<GeoPoint>,
    name: Option<String>,
    num_releated_points_skipped: usize,
}

#[allow(clippy::new_without_default)]
impl CourseBuilder {
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            prev_point: None,
            name: None,
            num_releated_points_skipped: 0,
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub fn add_route_point(&mut self, point: GeoPoint) -> Result<()> {
        match self.prev_point {
            Some(prev) => {
                if prev == point {
                    self.num_releated_points_skipped += 1
                } else {
                    // TODO: Investigate using elevation-corrected distances
                    self.segments
                        .push(GeoSegment::from_geo_points(prev, point)?);
                    self.prev_point = Some(point);
                }
            }

            None => self.prev_point = Some(point),
        }
        Ok(())
    }

    pub fn build(self) -> Course {
        let mut records = Vec::new();
        let mut cumulative_distance = Meters(0.0);
        match (self.segments.first(), self.prev_point) {
            (Some(first), _) => records.push(Record {
                point: first.point1,
                cumulative_distance,
            }),
            (None, Some(point)) => records.push(Record {
                point,
                cumulative_distance,
            }),
            (None, None) => (),
        }
        for segment in self.segments {
            cumulative_distance += segment.geo_distance;
            records.push(Record {
                point: segment.point2,
                cumulative_distance,
            });
        }
        Course {
            records,
            course_points: vec![],
            name: self.name,
            num_repeated_points_skipped: self.num_releated_points_skipped,
        }
    }
}

/// A course record.
#[derive(Clone, PartialEq, Debug)]
pub struct Record {
    /// Position including optional elevation.
    pub point: GeoPoint,

    /// Cumulative distance along the course.
    pub cumulative_distance: Meters<f64>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct CoursePoint {
    /// Position of the point's interception with the course.
    pub point: GeoPoint,

    /// Distance at which the point appears along the total course.
    pub distance: Meters<f64>,

    /// Course point type.
    pub point_type: CoursePointType,

    /// Name.
    pub name: String,
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use coretypes::{geo_point, geo_points};

    use crate::course::CourseBuilder;

    #[test]
    fn test_course_builder_empty() -> Result<()> {
        let course = CourseBuilder::new().build();
        assert_eq!(course.records, vec![]);
        Ok(())
    }

    #[test]
    fn test_course_builder_single_point() -> Result<()> {
        let mut builder = CourseBuilder::new();
        builder.add_route_point(geo_point!(1.0, 2.0))?;
        let record_points = builder
            .build()
            .records
            .iter()
            .map(|r| r.point)
            .collect::<Vec<_>>();

        let expected_points = geo_points![(1.0, 2.0)];

        assert_eq!(record_points, expected_points);
        Ok(())
    }

    #[test]
    fn test_course_builder_two_points() -> Result<()> {
        let mut builder = CourseBuilder::new();
        builder.add_route_point(geo_point!(1.0, 2.0))?;
        builder.add_route_point(geo_point!(1.1, 2.2))?;
        let record_points = builder
            .build()
            .records
            .iter()
            .map(|r| r.point)
            .collect::<Vec<_>>();

        let expected_points = geo_points![(1.0, 2.0), (1.1, 2.2)];

        assert_eq!(record_points, expected_points);
        Ok(())
    }

    #[test]
    fn test_repeated_points() -> Result<()> {
        let mut builder = CourseBuilder::new();
        builder.add_route_point(geo_point!(1.0, 2.0))?;
        builder.add_route_point(geo_point!(1.0, 2.0))?;
        builder.add_route_point(geo_point!(1.1, 2.2))?;
        builder.add_route_point(geo_point!(1.1, 2.2))?;
        builder.add_route_point(geo_point!(1.2, 2.1))?;
        builder.add_route_point(geo_point!(1.1, 2.2))?;
        builder.add_route_point(geo_point!(1.1, 2.2))?;
        let course = builder.build();
        let record_points = course.records.iter().map(|r| r.point).collect::<Vec<_>>();

        let expected_points = geo_points![(1.0, 2.0), (1.1, 2.2), (1.2, 2.1), (1.1, 2.2)];

        assert_eq!(record_points, expected_points);
        assert_eq!(course.num_repeated_points_skipped, 3);
        Ok(())
    }
}
