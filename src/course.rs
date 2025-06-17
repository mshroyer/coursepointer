//! Abstract representation of courses and waypoints
//!
//! Provides [`Course`], an abstract representation of a course with its records
//! and course points (if any). Courses are created by obtaining a
//! [`CourseSetBuilder`] and adding data to it.
//!
//! Once all the data has been added (for example, by parsing it from a GPX
//! file), [`CourseSetBuilder::build`] returns a [`CourseSet`].

use std::cmp::Ordering;

use dimensioned::si::{M, Meter};
use thiserror::Error;
use tracing::{Level, debug, info, span};

use crate::algorithm::{
    AlgorithmError, FromGeoPoints, NearbySegment, find_nearby_segments, intercept_distance_floor,
    karney_interception,
};
use crate::fit::CoursePointType;
use crate::geographic::{GeographicError, geodesic_inverse};
use crate::types::{GeoAndXyzPoint, GeoPoint, GeoSegment};

#[derive(Error, Debug)]
pub enum CourseError {
    #[error("Geographic calculation error")]
    Geographic(#[from] GeographicError),
    #[error("Attempt to access a missing course")]
    MissingCourse,
    #[error("Error in geographic calculation")]
    Algorithm(#[from] AlgorithmError),
    #[error("Distance is NaN")]
    NaNDistance,
}

type Result<T> = std::result::Result<T, CourseError>;

/// Strategy for handling duplicate intercepts from a waypoint.
///
/// Duplicate interception can happen in an out-and-back course, for example.
/// This strategy determines what to do in the case that duplicate intercepts
/// are available.
#[cfg_attr(feature = "cli", derive(clap::ValueEnum, strum::Display))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "cli", strum(serialize_all = "kebab-case"))]
#[cfg_attr(feature = "cli", clap(rename_all = "kebab-case"))]
pub enum InterceptStrategy {
    /// The nearest intercept should be chosen as the course point.
    Nearest,

    /// The first intercept by distance along the course should be chosen.
    First,

    /// All available intercepts should be chosen as duplicate course points.
    All,
}

/// Options for building a course.
pub struct CourseOptions {
    /// The threshold distance of a waypoint from the segments of a course,
    /// below which the course will be considered "intercepted" by the waypoint,
    /// turning it into a course point.
    pub threshold: Meter<f64>,

    /// What strategy should be applied in the case of duplicate course
    /// intercepts from a waypoint.
    pub strategy: InterceptStrategy,
}

impl Default for CourseOptions {
    fn default() -> Self {
        Self {
            threshold: 35.0 * M,
            strategy: InterceptStrategy::Nearest,
        }
    }
}

pub struct CourseSet {
    pub courses: Vec<Course>,
}

pub struct CourseSetBuilder {
    options: CourseOptions,
    courses: Vec<CourseBuilder>,
    waypoints: Vec<Waypoint>,
}

impl CourseSetBuilder {
    pub fn new(options: CourseOptions) -> Self {
        Self {
            options,
            courses: Vec::new(),
            waypoints: Vec::new(),
        }
    }

    pub fn add_course(&mut self) -> &mut CourseBuilder {
        self.courses.push(CourseBuilder::new());
        self.last_course_mut().unwrap()
    }

    pub fn last_course_mut(&mut self) -> Result<&mut CourseBuilder> {
        match self.courses.last_mut() {
            Some(course) => Ok(course),
            None => Err(CourseError::MissingCourse),
        }
    }

    pub fn add_waypoint(&mut self, waypoint: Waypoint) -> &mut Self {
        self.waypoints.push(waypoint);
        self
    }

    pub fn num_courses(&self) -> usize {
        self.courses.len()
    }

    pub fn build(mut self) -> Result<CourseSet> {
        let mut courses = vec![];
        self.process_waypoints()?;
        for course_builder in self.courses {
            courses.push(course_builder.build());
        }
        Ok(CourseSet { courses })
    }

    #[tracing::instrument(level = "debug", name = "process", skip_all)]
    fn process_waypoints(&mut self) -> Result<()> {
        for waypoint in &self.waypoints {
            let span = span!(Level::DEBUG, "waypoint", name = %waypoint.name);
            let _enter = span.enter();
            for course in &mut self.courses {
                let mut slns = Vec::new();
                let mut course_distance = 0.0 * M;
                for segment in &course.segments {
                    let floor = intercept_distance_floor(segment, &waypoint.point)?;
                    if floor > self.options.threshold {
                        slns.push(InterceptSolution::Far);
                    } else {
                        let intercept = karney_interception(segment, &waypoint.point)?;
                        let distance =
                            geodesic_inverse(&waypoint.point.geo, &intercept)?.geo_distance;
                        if distance.value_unsafe.is_nan() {
                            return Err(CourseError::NaNDistance);
                        }
                        let offset =
                            geodesic_inverse(&segment.point1.geo, &intercept)?.geo_distance;

                        slns.push(InterceptSolution::Near(NearIntercept {
                            intercept_point: intercept,
                            intercept_distance: distance,
                            course_distance: course_distance + offset,
                        }));
                    }
                    course_distance += segment.geo_distance;
                }

                let mut near_intercepts = find_nearby_segments(&slns, self.options.threshold)
                    .iter()
                    .filter_map(|sln| match sln {
                        InterceptSolution::Near(near) => Some(near),
                        InterceptSolution::Far => None,
                    })
                    .collect::<Vec<_>>();
                info!(
                    intercepts = near_intercepts.len(),
                    "Processed {:?}", waypoint.name,
                );
                for seg in &near_intercepts {
                    info!(
                        intercept_dist = ?seg.intercept_distance,
                        course_dist = %seg.course_distance,
                        "Intercept",
                    );
                }

                if !near_intercepts.is_empty() {
                    match self.options.strategy {
                        InterceptStrategy::Nearest => {
                            near_intercepts.sort_by(|a, b| {
                                a.intercept_distance
                                    .partial_cmp(&b.intercept_distance)
                                    .unwrap()
                            });
                            Self::add_course_point(
                                &mut course.course_points,
                                near_intercepts[0],
                                waypoint,
                            );
                        }

                        InterceptStrategy::First => {
                            Self::add_course_point(
                                &mut course.course_points,
                                near_intercepts[0],
                                waypoint,
                            );
                        }

                        InterceptStrategy::All => {
                            for sln in near_intercepts {
                                Self::add_course_point(&mut course.course_points, sln, waypoint);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn add_course_point(
        course_points: &mut Vec<CoursePoint>,
        sln: &NearIntercept,
        waypoint: &Waypoint,
    ) {
        course_points.push(CoursePoint {
            point: sln.intercept_point,
            distance: sln.course_distance,
            point_type: waypoint.point_type,
            name: waypoint.name.clone(),
        });
    }
}

#[derive(Clone, Copy, Debug)]
struct NearIntercept {
    /// The point of interception.
    intercept_point: GeoPoint,

    /// The geodesic distance between the intercept point and the waypoint.
    intercept_distance: Meter<f64>,

    /// The distance along the entire course at which this point of interception
    /// appears.
    course_distance: Meter<f64>,
}

impl PartialEq for NearIntercept {
    fn eq(&self, other: &Self) -> bool {
        self.intercept_distance.eq(&other.intercept_distance)
    }
}

impl PartialOrd for NearIntercept {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.intercept_distance
            .partial_cmp(&other.intercept_distance)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum InterceptSolution {
    Near(NearIntercept),
    Far,
}

impl PartialOrd for InterceptSolution {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self {
            InterceptSolution::Near(self_near) => match other {
                InterceptSolution::Near(other_near) => self_near.partial_cmp(&other_near),
                InterceptSolution::Far => Some(Ordering::Less),
            },
            InterceptSolution::Far => match other {
                InterceptSolution::Near(_) => Some(Ordering::Greater),
                InterceptSolution::Far => None,
            },
        }
    }
}

impl NearbySegment<Meter<f64>> for &InterceptSolution {
    fn waypoint_distance(self) -> Meter<f64> {
        match self {
            InterceptSolution::Near(near) => near.intercept_distance,
            InterceptSolution::Far => f64::INFINITY * M,
        }
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
    segments: Vec<GeoSegment<GeoAndXyzPoint>>,
    prev_point: Option<GeoAndXyzPoint>,
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

    pub fn with_name(&mut self, name: String) -> &mut Self {
        self.name = Some(name);
        self
    }

    pub fn with_route_point(&mut self, point: GeoPoint) -> Result<&mut Self> {
        let with_xyz = GeoAndXyzPoint::try_from(point)?;
        match self.prev_point {
            Some(prev) => {
                if prev == with_xyz {
                    self.num_releated_points_skipped += 1
                } else {
                    // TODO: Investigate using elevation-corrected distances
                    self.segments
                        .push(GeoSegment::from_geo_points(prev, with_xyz)?);
                    self.prev_point = Some(with_xyz);
                }
            }

            None => self.prev_point = Some(with_xyz),
        }
        Ok(self)
    }

    fn build(mut self) -> Course {
        match &self.name {
            Some(name) => info!("Building course {}", name),
            None => info!("Building untitled course"),
        }
        let mut records = Vec::new();
        let mut cumulative_distance = 0.0 * M;
        match (self.segments.first(), self.prev_point) {
            (Some(first), _) => records.push(Record {
                point: first.point1.geo,
                cumulative_distance,
            }),
            (None, Some(point)) => records.push(Record {
                point: point.geo,
                cumulative_distance,
            }),
            (None, None) => (),
        }
        let num_segments = self.segments.len();
        for segment in self.segments {
            cumulative_distance += segment.geo_distance;
            records.push(Record {
                point: segment.point2.geo,
                cumulative_distance,
            });
        }
        info!(
            "Processed {} course records ({} segments) with a total distance of {:.2}",
            records.len(),
            num_segments,
            cumulative_distance
        );
        debug!(
            "{} repeated records (zero-length segments) were excluded from the conversion",
            self.num_releated_points_skipped
        );

        // Unwrap is safe here because we check for NaN distances when adding
        // course points in the set builder
        self.course_points
            .sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());

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

/// A waypoint to be considered as a course point.
///
/// In contrast with [`GpxWaypoint`], this type specifies a FIT
/// [`CoursePointType`] instead of a set of optional GPX attributes. And in
/// contrast with a [`CoursePoint`], a Waypoint is not known to necessarily lie
/// along the course and lacks a known course distance.
pub struct Waypoint {
    /// Position of the waypoint.
    pub point: GeoAndXyzPoint,

    /// The type of course point this should be considered, if it does turn out
    /// to be one.
    pub point_type: CoursePointType,

    /// Name.
    pub name: String,
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
    use dimensioned::si::{M, Meter};

    use crate::course::{
        CourseBuilder, CourseSetBuilder, InterceptSolution, NearIntercept, Waypoint,
    };
    use crate::fit::CoursePointType;
    use crate::types::GeoPoint;
    use crate::{CourseOptions, geo_point, geo_points};

    #[test]
    fn test_course_builder_empty() -> Result<()> {
        let course = CourseBuilder::new().build();
        assert_eq!(course.records, vec![]);
        Ok(())
    }

    #[test]
    fn test_course_builder_single_point() -> Result<()> {
        let mut builder = CourseBuilder::new();
        builder.with_route_point(geo_point!(1.0, 2.0))?;
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
        builder
            .with_route_point(geo_point!(1.0, 2.0))?
            .with_route_point(geo_point!(1.1, 2.2))?;
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
        builder
            .with_route_point(geo_point!(1.0, 2.0))?
            .with_route_point(geo_point!(1.0, 2.0))?
            .with_route_point(geo_point!(1.1, 2.2))?
            .with_route_point(geo_point!(1.1, 2.2))?
            .with_route_point(geo_point!(1.2, 2.1))?
            .with_route_point(geo_point!(1.1, 2.2))?
            .with_route_point(geo_point!(1.1, 2.2))?;

        let course = builder.build();
        let record_points = course.records.iter().map(|r| r.point).collect::<Vec<_>>();

        let expected_points = geo_points![(1.0, 2.0), (1.1, 2.2), (1.2, 2.1), (1.1, 2.2)];

        assert_eq!(record_points, expected_points);
        Ok(())
    }

    #[test]
    fn test_intercept_long_segments() -> Result<()> {
        let mut builder = CourseSetBuilder::new(CourseOptions::default());
        builder
            .add_course()
            .with_route_point(geo_point!(35.5252717091331, -101.2856451853322))?
            .with_route_point(geo_point!(36.05200980326534, -90.02610043506964))?
            .with_route_point(geo_point!(38.13369722302025, -78.51238236506529))?;

        builder.add_waypoint(Waypoint {
            name: "MyWaypoint".to_owned(),
            point_type: CoursePointType::Generic,
            point: geo_point!(35.951314, -94.973085).try_into()?,
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

    #[test]
    fn test_intercept_distance_ordering() {
        fn near(distance: Meter<f64>) -> NearIntercept {
            NearIntercept {
                intercept_point: GeoPoint::default(),
                intercept_distance: distance,
                course_distance: 0.0 * M,
            }
        }

        assert!(InterceptSolution::Near(near(10.0 * M)) < InterceptSolution::Near(near(12.0 * M)));
        assert!(InterceptSolution::Near(near(15.0 * M)) > InterceptSolution::Near(near(12.0 * M)));
        assert!(InterceptSolution::Far > InterceptSolution::Near(near(12.0 * M)));
        assert!(InterceptSolution::Near(near(10.0 * M)) < InterceptSolution::Far);
        assert!(!(InterceptSolution::Far < InterceptSolution::Far));
        assert!(!(InterceptSolution::Far > InterceptSolution::Far));
        assert_eq!(InterceptSolution::Far, InterceptSolution::Far);
        assert_eq!(
            InterceptSolution::Near(near(10.0 * M)),
            InterceptSolution::Near(near(10.0 * M))
        );
    }
}
