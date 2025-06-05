//! Abstract representation of courses and waypoints
//!
//! Provides [`Course`], an abstract representation of a course with its records
//! and course points (if any). Courses are created by obtaining a
//! [`CourseSetBuilder`] and adding data to it.
//!
//! Once all the data has been added (for example, by parsing it from a GPX
//! file), [`CourseSetBuilder::build`] returns a [`CourseSet`].

use dimensioned::si::{M, Meter};
use log::debug;
use thiserror::Error;

use crate::algorithm::{
    AlgorithmError, FromGeoPoints, NearbySegment, find_nearby_segments, karney_interception,
};
use crate::fit::CoursePointType;
use crate::geographic::{GeographicError, geodesic_inverse};
use crate::gpx::Waypoint;
use crate::types::{GeoPoint, GeoSegment};

#[derive(Error, Debug)]
pub enum CourseError {
    #[error("Geographic calculation error")]
    Geographic(#[from] GeographicError),
    #[error("Attempt to access a missing course")]
    MissingCourse,
    #[error("Error in geographic calculation")]
    Algorithm(#[from] AlgorithmError),
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

    pub fn build(mut self) -> Result<CourseSet> {
        let mut courses = vec![];
        self.process_waypoints()?;
        for course_builder in self.courses {
            courses.push(course_builder.build());
        }
        Ok(CourseSet { courses })
    }

    fn process_waypoints(&mut self) -> Result<()> {
        for waypoint in &self.waypoints {
            for course in &mut self.courses {
                let mut slns = Vec::new();
                let mut course_distance = 0.0 * M;
                for segment in &course.segments {
                    let intercept = karney_interception(segment, &waypoint.point)?;
                    let distance = geodesic_inverse(&waypoint.point, &intercept)?.geo_distance;
                    let offset = geodesic_inverse(&segment.point1, &intercept)?.geo_distance;

                    slns.push(InterceptSolution {
                        intercept_point: intercept,
                        intercept_distance: distance,
                        course_distance: course_distance + offset,
                    });
                    course_distance += segment.geo_distance;
                }

                let near_segments = find_nearby_segments(&slns, 35.0 * M);
                debug!(
                    "Found {} segments near {}",
                    near_segments.len(),
                    waypoint.name
                );

                if !near_segments.is_empty() {
                    // TODO: Handle multiple passbys
                    let sln = near_segments[0];
                    course.course_points.push(CoursePoint {
                        point: sln.intercept_point,
                        distance: sln.course_distance,
                        point_type: CoursePointType::Generic,
                        name: waypoint.name.clone(),
                    })
                }
            }
        }
        Ok(())
    }
}

struct InterceptSolution {
    /// The point of interception.
    intercept_point: GeoPoint,

    /// The geodesic distance between the intercept point and the waypoint.
    intercept_distance: Meter<f64>,

    /// The distance along the entire course at which this point of interception
    /// appears.
    course_distance: Meter<f64>,
}

impl NearbySegment<Meter<f64>> for &InterceptSolution {
    fn waypoint_distance(&self) -> Meter<f64> {
        self.intercept_distance
    }
}

/// A course for navigation.
///
/// This contains the distance data needed as input for a FIT course file, but
/// it does not represent speeds or timestamps.
pub struct Course {
    /// The records (coordinates and cumulative distances) that define the
    /// course, in order of physical traversal.
    pub records: Vec<Record>,

    /// The course points and their cumulative distances. These are derived from
    /// the subset of waypoints provided to [`CourseSetBuilder`] that are
    /// located near the course.
    pub course_points: Vec<CoursePoint>,

    /// The name of the course, if given.
    pub name: Option<String>,
}

impl Course {
    /// The total distance of the course.
    pub fn total_distance(&self) -> Meter<f64> {
        self.records
            .last()
            .map(|x| x.cumulative_distance)
            .unwrap_or(0.0 * M)
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
    course_points: Vec<CoursePoint>,
    num_releated_points_skipped: usize,
}

#[allow(clippy::new_without_default)]
impl CourseBuilder {
    fn new() -> Self {
        Self {
            segments: Vec::new(),
            prev_point: None,
            name: None,
            course_points: Vec::new(),
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

    fn build(self) -> Course {
        match &self.name {
            Some(name) => debug!("Building course {}", name),
            None => debug!("Building untitled course"),
        }
        let mut records = Vec::new();
        let mut cumulative_distance = 0.0 * M;
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
        debug!(
            "Processed {} segments with a total distance of {}",
            records.len(),
            cumulative_distance
        );
        debug!(
            "{} repeated adjacent points were excluded from the conversion",
            self.num_releated_points_skipped
        );
        Course {
            records,
            course_points: self.course_points,
            name: self.name,
        }
    }
}

/// A course record.
#[derive(Clone, PartialEq, Debug)]
pub struct Record {
    /// Position including optional elevation.
    pub point: GeoPoint,

    /// Cumulative distance along the course.
    pub cumulative_distance: Meter<f64>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct CoursePoint {
    /// Position of the point's interception with the course.
    pub point: GeoPoint,

    /// Distance at which the point appears along the total course.
    pub distance: Meter<f64>,

    /// Course point type.
    pub point_type: CoursePointType,

    /// Name.
    pub name: String,
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use approx::assert_relative_eq;
    use dimensioned::si::M;

    use crate::course::{CourseBuilder, CourseSetBuilder};
    use crate::gpx::Waypoint;
    use crate::{geo_point, geo_points};

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
        Ok(())
    }

    #[test]
    fn test_intercept_long_segments() -> Result<()> {
        let mut builder = CourseSetBuilder::new();
        builder.create_course();
        let course = builder.current_mut()?;
        course.add_route_point(geo_point!(35.5252717091331, -101.2856451853322))?;
        course.add_route_point(geo_point!(36.05200980326534, -90.02610043506964))?;
        course.add_route_point(geo_point!(38.13369722302025, -78.51238236506529))?;

        builder.add_waypoint(Waypoint {
            name: "MyWaypoint".to_owned(),
            cmt: None,
            sym: None,
            type_: None,
            point: geo_point!(35.951314, -94.973085),
        });

        let course_set = builder.build()?;
        assert_eq!(course_set.courses.len(), 1);
        let course = course_set.courses.first().unwrap();
        assert_eq!(course.course_points.len(), 1);
        let course_point = course.course_points.first().unwrap();
        assert_relative_eq!(
            course_point.distance,
            572863.0 * M,
            max_relative = 0.0001 * M
        );
        Ok(())
    }
}
