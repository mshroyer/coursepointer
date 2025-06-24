//! Abstract representation of courses and waypoints
//!
//! Provides [`Course`], an abstract representation of a course with its records
//! and course points (if any). Courses are created by obtaining a
//! [`CourseSetBuilderImpl`] and adding data to it.
//!
//! Once all the data has been added (for example, by parsing it from a GPX
//! file), [`CourseSetBuilderImpl::build`] returns a [`CourseSet`].

use std::cmp::Ordering;
use std::convert::Infallible;

use dimensioned::si::{M, Meter};
#[cfg(feature = "rayon")]
use rayon::prelude::*;
use thiserror::Error;
use tracing::{debug, error, info};

use crate::algorithm::{
    AlgorithmError, FromGeoPoints, NearbySegment, find_nearby_segments, intercept_distance_floor,
    karney_interception,
};
use crate::fit::CoursePointType;
use crate::geographic::{GeographicError, geodesic_inverse};
use crate::types::{GeoAndXyzPoint, GeoPoint, GeoSegment, HasGeoPoint};

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
    #[error("Infallible")]
    Infallible(#[from] Infallible),
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

#[cfg(feature = "floor")]
pub type CourseSetBuilder = CourseSetBuilderImpl<GeoAndXyzPoint>;

#[cfg(not(feature = "floor"))]
pub type CourseSetBuilder = CourseSetBuilderImpl<GeoPoint>;

pub struct CourseSetBuilderImpl<P>
where
    P: HasGeoPoint + TryFrom<GeoPoint> + Send + Sync,
    <P as TryFrom<GeoPoint>>::Error: Send,
    CourseError: From<<P as TryFrom<GeoPoint>>::Error>,
{
    options: CourseOptions,
    course_builders: Vec<CourseBuilder<P>>,
    waypoints: Vec<Waypoint<P>>,
}

impl<P> CourseSetBuilderImpl<P>
where
    Self: SolveIntercept<P>,
    P: HasGeoPoint + TryFrom<GeoPoint> + Send + Sync,
    <P as TryFrom<GeoPoint>>::Error: Send,
    CourseError: From<<P as TryFrom<GeoPoint>>::Error>,
{
    pub fn new(options: CourseOptions) -> Self {
        Self {
            options,
            course_builders: Vec::new(),
            waypoints: Vec::new(),
        }
    }

    pub fn add_course(&mut self) -> &mut CourseBuilder<P> {
        self.course_builders.push(CourseBuilder::new());
        self.last_course_mut().unwrap()
    }

    pub fn last_course_mut(&mut self) -> Result<&mut CourseBuilder<P>> {
        match self.course_builders.last_mut() {
            Some(course) => Ok(course),
            None => Err(CourseError::MissingCourse),
        }
    }

    pub fn add_waypoint(&mut self, waypoint: Waypoint<P>) -> &mut Self {
        self.waypoints.push(waypoint);
        self
    }

    pub fn num_courses(&self) -> usize {
        self.course_builders.len()
    }

    /// Build the courses.
    ///
    /// The geodesic calculations happen in here.
    pub fn build(mut self) -> Result<CourseSet> {
        let mut courses = Vec::new();
        let mut course_builders = std::mem::take(&mut self.course_builders);
        let mut segmented_courses = course_builders
            .iter_mut()
            .map(|c| c.segment())
            .collect::<Result<Vec<_>>>()?;
        self.process_waypoints(&mut segmented_courses)?;
        for segmented_course in segmented_courses {
            courses.push(segmented_course.build()?);
        }
        Ok(CourseSet { courses })
    }

    fn solve_near_intercept(
        segment: &GeoSegment<P>,
        waypoint: &Waypoint<P>,
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
        waypoint: &Waypoint<P>,
        course: &SegmentedCourseBuilder<P>,
        threshold: Meter<f64>,
    ) -> Result<Vec<NearIntercept>> {
        let mut slns = Vec::new();
        for (segment, start_distance) in &course.segments_and_distances {
            slns.push(<Self as SolveIntercept<P>>::solve_intercept(
                segment,
                waypoint,
                threshold,
                *start_distance,
            )?);
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
    fn process_waypoints(
        &self,
        segmented_courses: &mut Vec<SegmentedCourseBuilder<P>>,
    ) -> Result<()> {
        for segmented_course in segmented_courses {
            let waypoints_and_intercepts = iter_work!(&self.waypoints)
                .map(|waypoint| {
                    let near_intercepts = Self::process_single_waypoint(
                        waypoint,
                        segmented_course,
                        self.options.threshold,
                    )?;
                    Ok((waypoint, near_intercepts))
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
        waypoint: &Waypoint<P>,
    ) {
        course_points.push(CoursePoint {
            point: sln.intercept_point,
            distance: sln.course_distance,
            point_type: waypoint.point_type,
            name: waypoint.name.clone(),
        });
    }
}

pub trait SolveIntercept<P>
where
    P: HasGeoPoint + TryFrom<GeoPoint> + Send + Sync,
    CourseError: From<<P as TryFrom<GeoPoint>>::Error>,
{
    fn solve_intercept(
        segment: &GeoSegment<P>,
        waypoint: &Waypoint<P>,
        _threshold: Meter<f64>,
        course_distance: Meter<f64>,
    ) -> Result<InterceptSolution>;
}

impl SolveIntercept<GeoPoint> for CourseSetBuilderImpl<GeoPoint> {
    fn solve_intercept(
        segment: &GeoSegment<GeoPoint>,
        waypoint: &Waypoint<GeoPoint>,
        _threshold: Meter<f64>,
        course_distance: Meter<f64>,
    ) -> Result<InterceptSolution> {
        Self::solve_near_intercept(segment, waypoint, course_distance)
    }
}

impl SolveIntercept<GeoAndXyzPoint> for CourseSetBuilderImpl<GeoAndXyzPoint> {
    fn solve_intercept(
        segment: &GeoSegment<GeoAndXyzPoint>,
        waypoint: &Waypoint<GeoAndXyzPoint>,
        threshold: Meter<f64>,
        course_distance: Meter<f64>,
    ) -> Result<InterceptSolution> {
        if intercept_distance_floor(segment, &waypoint.point)? > threshold {
            Ok(InterceptSolution::Far)
        } else {
            Self::solve_near_intercept(segment, waypoint, course_distance)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NearIntercept {
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
pub enum InterceptSolution {
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

/// A course for navigation.
///
/// This contains the distance data needed as input for a FIT course file, but
/// it does not represent speeds or timestamps.
pub struct Course {
    /// The records (coordinates and cumulative distances) that define the
    /// course, in order of physical traversal.
    pub records: Vec<Record>,

    /// The course points and their cumulative distances. These are derived from
    /// the subset of waypoints provided to [`CourseSetBuilderImpl`] that are
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

/// Builder for [`Course`].
///
/// Used as part of [`CourseSetBuilderImpl`] to allow composing a course.
/// Within that context, obtain a new instance with
/// [`CourseSetBuilderImpl::add_course`].
pub struct CourseBuilder<P>
where
    P: HasGeoPoint + TryFrom<GeoPoint> + Send + Sync,
    for<'a> GeoSegment<'a, P>: FromGeoPoints<'a, P>,
    <P as TryFrom<GeoPoint>>::Error: Send,
    CourseError: From<<P as TryFrom<GeoPoint>>::Error>,
{
    route_points: Vec<GeoPoint>,
    ps: Vec<P>,
    name: Option<String>,
    num_repeated_points_skipped: usize,
}

#[allow(clippy::new_without_default)]
impl<P> CourseBuilder<P>
where
    P: HasGeoPoint + TryFrom<GeoPoint> + Send + Sync,
    for<'a> GeoSegment<'a, P>: FromGeoPoints<'a, P>,
    <P as TryFrom<GeoPoint>>::Error: Send,
    CourseError: From<<P as TryFrom<GeoPoint>>::Error>,
{
    fn new() -> Self {
        Self {
            route_points: Vec::new(),
            ps: Vec::new(),
            name: None,
            num_repeated_points_skipped: 0,
        }
    }

    /// Set the course's name.
    pub fn with_name(&mut self, name: String) -> &mut Self {
        self.name = Some(name);
        self
    }

    /// Add a route point to the course.
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

    /// Segment the course.
    ///
    /// Does the initial geodesic calculations of solving the indirect problem
    /// between adjacent points, and lifting points into instances the type
    /// parameter `P` (such as [`XyzPoint`]).
    fn segment(&mut self) -> Result<SegmentedCourseBuilder<P>> {
        self.ps = iter_work!(self.route_points)
            .map(|p| P::try_from(*p))
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let index_pairs = (0..self.ps.len().saturating_sub(1))
            .map(|i| (i, i + 1))
            .collect::<Vec<_>>();
        let segments = iter_work!(index_pairs)
            .map(|(i, j)| GeoSegment::from_geo_points(&self.ps[*i], &self.ps[*j]))
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let segments_and_distances: Vec<(GeoSegment<P>, Meter<f64>)> = segments
            .into_iter()
            .scan(0.0 * M, |dist, s| {
                let start_dist = *dist;
                *dist += s.geo_length;
                Some((s, start_dist))
            })
            .collect::<Vec<_>>();

        Ok(SegmentedCourseBuilder {
            route_points: &self.ps,
            segments_and_distances,
            name: self.name.clone(),
            course_points: Vec::new(),
            num_repeated_points_skipped: self.num_repeated_points_skipped,
        })
    }
}

/// A segmented [`Course`] builder.
///
/// This represents an intermediate stage of building a [`CourseBuilder`]: The
/// initial work of processing route points into geodesic segments along with
/// distance information, as well as possibly [`XyPoint`] values, has been done.
///
/// This builder is used internally within [`CourseSetBuilderImpl`] to gather
/// course points.
struct SegmentedCourseBuilder<'a, P>
where
    P: HasGeoPoint + TryFrom<GeoPoint> + Send + Sync,
    GeoSegment<'a, P>: FromGeoPoints<'a, P>,
    <P as TryFrom<GeoPoint>>::Error: Send,
    CourseError: From<<P as TryFrom<GeoPoint>>::Error>,
{
    route_points: &'a Vec<P>,
    segments_and_distances: Vec<(GeoSegment<'a, P>, Meter<f64>)>,
    name: Option<String>,
    course_points: Vec<CoursePoint>,
    num_repeated_points_skipped: usize,
}

impl<'a, P> SegmentedCourseBuilder<'a, P>
where
    P: HasGeoPoint + TryFrom<GeoPoint> + Send + Sync,
    GeoSegment<'a, P>: FromGeoPoints<'a, P>,
    <P as TryFrom<GeoPoint>>::Error: Send,
    CourseError: From<<P as TryFrom<GeoPoint>>::Error>,
{
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
        if let Some(rp) = self.route_points.last() {
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
/// In contrast with `GpxWaypoint`, this type specifies a FIT
/// [`CoursePointType`] instead of a set of optional GPX attributes. And in
/// contrast with a `CoursePoint`, a Waypoint is not known to necessarily lie
/// along the course and lacks a known course distance.
pub struct Waypoint<P>
where
    P: HasGeoPoint + TryFrom<GeoPoint> + Send + Sync,
    CourseError: From<<P as TryFrom<GeoPoint>>::Error>,
{
    /// Position of the waypoint.
    pub point: P,

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
    use crate::types::{GeoAndXyzPoint, GeoPoint};
    use crate::{CourseOptions, geo_point, geo_points};

    #[test]
    fn test_course_builder_empty() -> Result<()> {
        let course = CourseBuilder::<GeoAndXyzPoint>::new().segment()?.build()?;
        assert_eq!(course.records, vec![]);
        Ok(())
    }

    #[test]
    fn test_course_builder_single_point() -> Result<()> {
        let mut builder = CourseBuilder::<GeoAndXyzPoint>::new();
        builder.with_route_point(geo_point!(1.0, 2.0));
        let record_points = builder
            .segment()?
            .build()?
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
        let mut builder = CourseBuilder::<GeoAndXyzPoint>::new();
        builder
            .with_route_point(geo_point!(1.0, 2.0))
            .with_route_point(geo_point!(1.1, 2.2));
        let record_points = builder
            .segment()?
            .build()?
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
        let mut builder = CourseBuilder::<GeoAndXyzPoint>::new();
        builder
            .with_route_point(geo_point!(1.0, 2.0))
            .with_route_point(geo_point!(1.0, 2.0))
            .with_route_point(geo_point!(1.1, 2.2))
            .with_route_point(geo_point!(1.1, 2.2))
            .with_route_point(geo_point!(1.2, 2.1))
            .with_route_point(geo_point!(1.1, 2.2))
            .with_route_point(geo_point!(1.1, 2.2));

        let course = builder.segment()?.build()?;
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
            .with_route_point(geo_point!(35.5252717091331, -101.2856451853322))
            .with_route_point(geo_point!(36.05200980326534, -90.02610043506964))
            .with_route_point(geo_point!(38.13369722302025, -78.51238236506529));

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
