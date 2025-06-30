//! Types for composing courses that contain course points
//!
//! # Course points and courses, waypoints and routes
//!
//! This module borrows terminology from GPX and Garmin FIT.  Here a route, like
//! a GPX route, consists of a sequence of route points with latitude,
//! longitude, and optionally elevation.  And a waypoint is just a named
//! location, which may or may not be associated with any given route.
//!
//! (GPX also has a notion of tracks and track points, but here they are
//! represented as routes and route points.  This module has no concept
//! corresponding to a GPX track segment.)
//!
//! These GPX-based concepts have FIT analogues, which contain additional
//! distance information:
//!
//! | GPX term    | FIT term     | FIT additional data                        |
//! | ----------- | ------------ | ------------------------------------------ |
//! | route       | course       | Total distance                             |
//! | route point | record       | Distance along the course                  |
//! | waypoint    | course point | Association with and distance along course |
//!
//! The role of this module is to calculate this additional information needed
//! to turn a set of routes and waypoints into courses and associated course
//! points.
//!
//! Unlike in a Garmin FIT course file, here course records do not contain
//! timestamps, but those can be trivially computed from records' distances
//! based on a given start time and speed.
//!
//! # Module overview
//!
//! This module provides [`CourseSetBuilder`], which is used to build a set of
//! routes and waypoints into a set of courses and their now-associated course
//! points.
//!
//! Individual routes are added with [`CourseSetBuilder::add_route`], and their
//! [`RouteBuilder::with_name`] and [`RouteBuilder::with_route_point`] methods
//! are used to add names and route points to them.
//!
//! Waypoints are added to the set (not to a particular route) with
//! [`CourseSetBuilder::add_waypoint`].
//!
//! Once your routes and waypoints have been assembled, running
//! [`CourseSetBuilder::build`] does the work needed to determine which
//! waypoints qualify as course points along any of the given routes.  It
//! returns a [`CourseSet`] containing [`Course`] instances, which in turn will
//! contain any identified [`CoursePoint`] instances.
//!
//! # Units of measure
//!
//! Courses and related types here us zero-cost unit of measure types from
//! [dimensioned](https://docs.rs/dimensioned/latest/dimensioned/) to avoid type
//! confusion of speed and distance quantities, and analogous types implemented
//! here for angular degrees.
//!
//! You can obtain a dimensional quantity by multiplying a constant representing
//! the unit of measure, for example:
//!
//! ```
//! let distance: Meter<f64> = 5.0 * M;
//! let latitude: Degree<f64> = 36.3 * DEG;
//! ```
//!
//! And then to get the magnitude from a quantity with a unit of measure:
//!
//! ```
//! let magnitude: f64 = distance.value_unsafe;
//! ```

use std::cmp::Ordering;

use dimensioned::si::{M, Meter};
#[cfg(feature = "rayon")]
use rayon::prelude::*;
use thiserror::Error;
use tracing::{debug, error, info};

use crate::algorithm::{
    AlgorithmError, FromGeoPoints, NearbySegment, find_nearby_segments, intercept_distance_floor,
    karney_interception,
};
use crate::geographic::{GeographicError, geodesic_inverse};
use crate::types::{GeoAndXyzPoint, GeoSegment, HasGeoPoint};
use crate::{CoursePointType, GeoPoint};

/// An error computing a [`CourseSet`]
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

#[cfg(not(feature = "rayon"))]
macro_rules! iter_work {
    ($i:expr) => {
        $i.iter()
    };
}

#[cfg(feature = "rayon")]
macro_rules! iter_work {
    ($i:expr) => {
        $i.par_iter()
    };
}

/// Options for building a course set
pub struct CourseSetOptions {
    /// The maximum distance between a waypoint and a route, within which the
    /// waypoint will be considered to be a course point along the corresponding
    /// course.
    pub threshold: Meter<f64>,

    /// What strategy to apply when a waypoint intercepts a single route
    /// multiple times.
    pub strategy: InterceptStrategy,
}

/// A strategy for handling duplicate intercepts from a waypoint.
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

impl Default for CourseSetOptions {
    fn default() -> Self {
        Self {
            threshold: 35.0 * M,
            strategy: InterceptStrategy::Nearest,
        }
    }
}

impl CourseSetOptions {
    pub fn with_threshold(self, threshold: Meter<f64>) -> Self {
        Self {
            threshold,
            strategy: self.strategy,
        }
    }

    pub fn with_strategy(self, strategy: InterceptStrategy) -> Self {
        Self {
            threshold: self.threshold,
            strategy,
        }
    }
}

/// A set of [`Course`]s and their associated course points.
pub struct CourseSet {
    /// Courses with any associated course points.
    pub courses: Vec<Course>,

    /// The total number of waypoints that were specified to the builder.  This
    /// may be greater than the number of course points that were actually
    /// resolved for the course(s).
    pub num_waypoints: usize,
}

/// A navigation course
///
/// This contains the distance data needed as input for a FIT course file, but
/// it does not represent speeds or timestamps.
pub struct Course {
    /// The records (coordinates and cumulative distances) that define the
    /// course, in order of physical traversal.
    pub records: Vec<Record>,

    /// The course points and their course distances. These are derived from the
    /// subset of waypoints provided to [`CourseSetBuilder`] that are located
    /// near the course.
    pub course_points: Vec<CoursePoint>,

    /// The name of the course, if given.
    pub name: Option<String>,
}

impl Course {
    /// The total distance of the course
    pub fn total_distance(&self) -> Meter<f64> {
        self.records
            .last()
            .map(|x| x.cumulative_distance)
            .unwrap_or(0.0 * M)
    }

    /// Checks whether elevation data is available in this course
    pub fn has_elevation(&self) -> bool {
        self.records.iter().all(|r| r.point.ele().is_some())
    }
}

/// Builds routes and waypoints into courses with associated course points
///
/// In general, the property-setting methods like
/// [`CourseSetBuilder::add_route`] and [`CourseSetBuilder::add_waypoint`] are
/// "cheap", and the CPU-bound work is confined to [`CourseSetBuilder::build`].
pub struct CourseSetBuilder {
    options: CourseSetOptions,
    route_builders: Vec<RouteBuilder>,
    waypoints: Vec<Waypoint<GeoPoint>>,
}

impl CourseSetBuilder {
    /// Returns a new builder using the given [`CourseSetOptions`].
    pub fn new(options: CourseSetOptions) -> Self {
        Self {
            options,
            route_builders: Vec::new(),
            waypoints: Vec::new(),
        }
    }

    /// Adds a new [`RouteBuilder`] to this set builder
    pub fn add_route(&mut self) -> &mut RouteBuilder {
        self.route_builders.push(RouteBuilder::new());
        self.last_route_mut().unwrap()
    }

    /// Returns a mutable reference to the most recently-added route
    pub fn last_route_mut(&mut self) -> Result<&mut RouteBuilder> {
        match self.route_builders.last_mut() {
            Some(course) => Ok(course),
            None => Err(CourseError::MissingCourse),
        }
    }

    /// Adds a waypoint to the set of routes
    ///
    /// The [`CoursePointType`] given here determines the type that will be used
    /// should the waypoint be identified as a course point.
    pub fn add_waypoint(
        &mut self,
        point: GeoPoint,
        point_type: CoursePointType,
        name: String,
    ) -> &mut Self {
        self.waypoints.push(Waypoint::<GeoPoint> {
            point,
            point_type,
            name,
        });
        self
    }

    /// Returns the number of routes currently contained in this builder
    pub fn num_routes(&self) -> usize {
        self.route_builders.len()
    }

    /// Build the courses
    ///
    /// The geodesic calculations happen in here.
    pub fn build(mut self) -> Result<CourseSet> {
        let mut courses = Vec::new();
        let mut course_builders = std::mem::take(&mut self.route_builders);
        let mut segmented_courses = course_builders
            .iter_mut()
            .map(|c| c.segment())
            .collect::<Result<Vec<_>>>()?;
        self.process_waypoints(&mut segmented_courses)?;
        for segmented_course in segmented_courses {
            courses.push(segmented_course.build()?);
        }
        Ok(CourseSet {
            courses,
            num_waypoints: self.waypoints.len(),
        })
    }

    fn solve_near_intercept(
        segment: &GeoSegment<GeoAndXyzPoint>,
        waypoint: &Waypoint<GeoAndXyzPoint>,
        course_distance: Meter<f64>,
    ) -> Result<InterceptSolution> {
        let intercept = karney_interception(segment, &waypoint.point)?;
        let distance = geodesic_inverse(waypoint.point.geo(), &intercept)?.geo_distance;
        if distance.value_unsafe.is_nan() {
            return Err(CourseError::NaNDistance);
        }
        let offset = geodesic_inverse(segment.start.geo(), &intercept)?.geo_distance;

        Ok(InterceptSolution::Near(NearIntercept {
            intercept_point: intercept,
            intercept_distance: distance,
            course_distance: course_distance + offset,
        }))
    }

    fn process_single_waypoint(
        waypoint: &Waypoint<GeoAndXyzPoint>,
        course: &SegmentedCourseBuilder,
        threshold: Meter<f64>,
    ) -> Result<Vec<NearIntercept>> {
        let mut slns = Vec::new();
        for (segment, start_distance) in &course.segments_and_distances {
            slns.push(
                if intercept_distance_floor(segment, &waypoint.point)? > threshold {
                    InterceptSolution::Far
                } else {
                    Self::solve_near_intercept(segment, waypoint, *start_distance)?
                },
            );
        }

        let near_intercepts = find_nearby_segments(&slns, threshold)
            .iter()
            .filter_map(|sln| match sln {
                InterceptSolution::Near(near) => Some(*near),
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
        Ok(near_intercepts)
    }

    #[tracing::instrument(level = "debug", name = "process_waypoints", skip_all)]
    fn process_waypoints(&self, segmented_courses: &mut Vec<SegmentedCourseBuilder>) -> Result<()> {
        for segmented_course in segmented_courses {
            let waypoints_and_intercepts = iter_work!(&self.waypoints)
                .map(|waypoint| {
                    let xyz_waypoint: Waypoint<GeoAndXyzPoint> = waypoint.clone().try_into()?;
                    let near_intercepts = Self::process_single_waypoint(
                        &xyz_waypoint,
                        segmented_course,
                        self.options.threshold,
                    )?;
                    Ok((xyz_waypoint, near_intercepts))
                })
                .collect::<Result<Vec<_>>>()?;

            for (waypoint, near_intercepts) in waypoints_and_intercepts.iter() {
                if !near_intercepts.is_empty() {
                    match self.options.strategy {
                        InterceptStrategy::Nearest => {
                            let mut near_sorted = near_intercepts.clone();
                            near_sorted.sort_by(|a, b| {
                                a.intercept_distance
                                    .partial_cmp(&b.intercept_distance)
                                    .unwrap()
                            });
                            Self::add_course_point(
                                &mut segmented_course.course_points,
                                &near_sorted[0],
                                waypoint,
                            );
                        }

                        InterceptStrategy::First => {
                            Self::add_course_point(
                                &mut segmented_course.course_points,
                                &near_intercepts[0],
                                waypoint,
                            );
                        }

                        InterceptStrategy::All => {
                            for sln in near_intercepts {
                                Self::add_course_point(
                                    &mut segmented_course.course_points,
                                    sln,
                                    waypoint,
                                );
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
        waypoint: &Waypoint<GeoAndXyzPoint>,
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
                InterceptSolution::Near(other_near) => self_near.partial_cmp(other_near),
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

/// Builds information from a route into a [`Course`]
///
/// Used as part of [`CourseSetBuilder`] for composing a course. Within that
/// context, obtain a new instance with [`CourseSetBuilder::add_route`].
pub struct RouteBuilder {
    route_points: Vec<GeoPoint>,
    xyz_points: Vec<GeoAndXyzPoint>,
    name: Option<String>,
    num_repeated_points_skipped: usize,
}

#[allow(clippy::new_without_default)]
impl RouteBuilder {
    fn new() -> Self {
        Self {
            route_points: Vec::new(),
            xyz_points: Vec::new(),
            name: None,
            num_repeated_points_skipped: 0,
        }
    }

    /// Sets the course's name
    pub fn with_name(&mut self, name: String) -> &mut Self {
        self.name = Some(name);
        self
    }

    /// Adds a route point to the course
    ///
    /// Adds GPX route points (or equivalently, track points) to the course in
    /// order of traversal.
    pub fn with_route_point(&mut self, point: GeoPoint) -> &mut Self {
        if let Some(prev) = self.route_points.last() {
            if *prev == point {
                self.num_repeated_points_skipped += 1;
                return self;
            }
        }
        self.route_points.push(point);
        self
    }

    /// Segments the course
    ///
    /// Does the initial geodesic calculations of solving the indirect problem
    /// between adjacent points, and lifting points into instances the type
    /// parameter `P` (such as [`XyzPoint`]).
    fn segment(&mut self) -> Result<SegmentedCourseBuilder> {
        self.xyz_points = iter_work!(self.route_points)
            .map(|p| GeoAndXyzPoint::try_from(*p))
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let index_pairs = (0..self.xyz_points.len().saturating_sub(1))
            .map(|i| (i, i + 1))
            .collect::<Vec<_>>();
        let segments = iter_work!(index_pairs)
            .map(|(i, j)| GeoSegment::from_geo_points(&self.xyz_points[*i], &self.xyz_points[*j]))
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let segments_and_distances: Vec<(GeoSegment<GeoAndXyzPoint>, Meter<f64>)> = segments
            .into_iter()
            .scan(0.0 * M, |dist, s| {
                let start_dist = *dist;
                *dist += s.geo_length;
                Some((s, start_dist))
            })
            .collect::<Vec<_>>();

        Ok(SegmentedCourseBuilder {
            xyz_points: &self.xyz_points,
            segments_and_distances,
            name: self.name.clone(),
            course_points: Vec::new(),
            num_repeated_points_skipped: self.num_repeated_points_skipped,
        })
    }
}

/// A segmented [`Course`] builder.
///
/// This represents an intermediate stage of building a [`Course`]: The initial
/// work of processing route points into geodesic segments along with computing
/// distance information and [`XyPoint`] values has been done.
///
/// This builder is used internally within [`CourseSetBuilder`] to gather
/// process waypoints into course points.
struct SegmentedCourseBuilder<'a> {
    xyz_points: &'a Vec<GeoAndXyzPoint>,
    segments_and_distances: Vec<(GeoSegment<'a, GeoAndXyzPoint>, Meter<f64>)>,
    name: Option<String>,
    course_points: Vec<CoursePoint>,
    num_repeated_points_skipped: usize,
}

impl<'a> SegmentedCourseBuilder<'a> {
    fn build(mut self) -> Result<Course> {
        match &self.name {
            Some(name) => info!("Building course {}", name),
            None => info!("Building untitled course"),
        }
        let mut records = Vec::new();
        let num_segments = self.segments_and_distances.len();
        let total_distance = match self.segments_and_distances.last() {
            Some((s, start_distance)) => *start_distance + s.geo_length,
            None => 0.0 * M,
        };
        for (segment, start_distance) in self.segments_and_distances {
            records.push(Record {
                point: *segment.start.geo(),
                cumulative_distance: start_distance,
            })
        }
        if let Some(rp) = self.xyz_points.last() {
            records.push(Record {
                point: *rp.geo(),
                cumulative_distance: total_distance,
            })
        }

        info!(
            "Processed {} course records ({} segments) with a total distance of {:.2}",
            records.len(),
            num_segments,
            total_distance,
        );
        debug!(
            "{} repeated records (zero-length segments) were excluded from the conversion",
            self.num_repeated_points_skipped
        );

        // Unwrap is safe here because we check for NaN distances when adding
        // course points in the set builder
        self.course_points
            .sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());

        Ok(Course {
            records,
            course_points: self.course_points,
            name: self.name,
        })
    }
}

/// A course record
///
/// Represents a single point along a course, analogous to a route point or a
/// track point.  However, a course record additionally contains the points'
/// cumulative distances along the course.
#[derive(Clone, PartialEq, Debug)]
pub struct Record {
    /// Position including optional elevation
    pub point: GeoPoint,

    /// Cumulative distance along the course
    pub cumulative_distance: Meter<f64>,
}

/// A waypoint to be considered as a course point
///
/// In contrast with `GpxWaypoint`, this type specifies a FIT
/// [`CoursePointType`] instead of a set of optional GPX attributes. And in
/// contrast with a `CoursePoint`, a Waypoint is not known to necessarily lie
/// along the course and lacks a known course distance.
#[derive(Clone)]
struct Waypoint<P: HasGeoPoint> {
    /// Position of the waypoint.
    point: P,

    /// The type of course point this should be considered, if it does turn out
    /// to be one.
    point_type: CoursePointType,

    /// Name.
    name: String,
}

impl TryFrom<Waypoint<GeoPoint>> for Waypoint<GeoAndXyzPoint> {
    type Error = GeographicError;

    fn try_from(value: Waypoint<GeoPoint>) -> std::result::Result<Self, Self::Error> {
        Ok(Waypoint::<GeoAndXyzPoint> {
            point: value.point.try_into()?,
            point_type: value.point_type,
            name: value.name,
        })
    }
}

/// A course point
///
/// Represents a point of interest along a course.  Unlike the waypoint from
/// which this is derived, a course point is known to be specifically *along*
/// the course, and its distance down the course is known.
#[derive(Clone, PartialEq, Debug)]
pub struct CoursePoint {
    /// Position of the point's interception with the course
    ///
    /// Note that this is calculated as interception from the waypoint, so the
    /// course point's position can be different from that of the original
    /// waypoint.
    pub point: GeoPoint,

    /// Distance at which the point appears along the total course
    pub distance: Meter<f64>,

    /// Course point type
    pub point_type: CoursePointType,

    /// Name
    pub name: String,
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use approx::assert_relative_eq;
    use dimensioned::si::{M, Meter};

    use crate::course::{CourseSetBuilder, InterceptSolution, NearIntercept, RouteBuilder};
    use crate::fit::CoursePointType;
    use crate::types::GeoPoint;
    use crate::{CourseSetOptions, geo_point, geo_points};

    #[test]
    fn test_route_builder_empty() -> Result<()> {
        let course = RouteBuilder::new().segment()?.build()?;
        assert_eq!(course.records, vec![]);
        Ok(())
    }

    #[test]
    fn test_route_builder_single_point() -> Result<()> {
        let mut builder = RouteBuilder::new();
        builder.with_route_point(geo_point!(1.0, 2.0)?);
        let record_points = builder
            .segment()?
            .build()?
            .records
            .iter()
            .map(|r| r.point)
            .collect::<Vec<_>>();

        let expected_points = geo_points![(1.0, 2.0)]?;

        assert_eq!(record_points, expected_points);
        Ok(())
    }

    #[test]
    fn test_route_builder_two_points() -> Result<()> {
        let mut builder = RouteBuilder::new();
        builder
            .with_route_point(geo_point!(1.0, 2.0)?)
            .with_route_point(geo_point!(1.1, 2.2)?);
        let record_points = builder
            .segment()?
            .build()?
            .records
            .iter()
            .map(|r| r.point)
            .collect::<Vec<_>>();

        let expected_points = geo_points![(1.0, 2.0), (1.1, 2.2)]?;

        assert_eq!(record_points, expected_points);
        Ok(())
    }

    #[test]
    fn test_repeated_points() -> Result<()> {
        let mut builder = RouteBuilder::new();
        builder
            .with_route_point(geo_point!(1.0, 2.0)?)
            .with_route_point(geo_point!(1.0, 2.0)?)
            .with_route_point(geo_point!(1.1, 2.2)?)
            .with_route_point(geo_point!(1.1, 2.2)?)
            .with_route_point(geo_point!(1.2, 2.1)?)
            .with_route_point(geo_point!(1.1, 2.2)?)
            .with_route_point(geo_point!(1.1, 2.2)?);

        let course = builder.segment()?.build()?;
        let record_points = course.records.iter().map(|r| r.point).collect::<Vec<_>>();

        let expected_points = geo_points![(1.0, 2.0), (1.1, 2.2), (1.2, 2.1), (1.1, 2.2)]?;

        assert_eq!(record_points, expected_points);
        Ok(())
    }

    #[test]
    fn test_intercept_long_segments() -> Result<()> {
        let mut builder = CourseSetBuilder::new(CourseSetOptions::default());
        builder
            .add_route()
            .with_route_point(geo_point!(35.5252717091331, -101.2856451853322)?)
            .with_route_point(geo_point!(36.05200980326534, -90.02610043506964)?)
            .with_route_point(geo_point!(38.13369722302025, -78.51238236506529)?);

        builder.add_waypoint(
            geo_point!(35.951314, -94.973085)?,
            CoursePointType::Generic,
            "MyWaypoint".to_owned(),
        );

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
    fn test_multiple_routes() -> Result<()> {
        // A pair of routes and two waypoints, one of which intercepts one
        // course and the other intercepts both.
        let mut builder =
            CourseSetBuilder::new(CourseSetOptions::default().with_threshold(100.0 * M));
        builder
            .add_route()
            .with_route_point(geo_point!(37.25579, -122.19817)?)
            .with_route_point(geo_point!(37.25997, -122.18813)?)
            .with_route_point(geo_point!(37.26310, -122.17985)?);
        builder
            .add_route()
            .with_route_point(geo_point!(37.26924, -122.18951)?)
            .with_route_point(geo_point!(37.25803, -122.19300)?)
            .with_route_point(geo_point!(37.26310, -122.17977)?);
        builder.add_waypoint(
            geo_point!(37.26376, -122.19067)?,
            CoursePointType::Generic,
            "SingleRoute".to_owned(),
        );
        builder.add_waypoint(
            geo_point!(37.26104, -122.18569)?,
            CoursePointType::Generic,
            "DoubleRoute".to_owned(),
        );

        let course_set = builder.build()?;
        assert_eq!(course_set.courses.len(), 2);
        assert_eq!(course_set.courses[0].course_points.len(), 1);
        assert_eq!(course_set.courses[1].course_points.len(), 2);
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
