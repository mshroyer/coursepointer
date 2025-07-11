//! Algorithms for geodesic interception
//!
//! To compute course points' distances and positions along a course, we need to
//! solve the "interception problem" between the course point and the segments
//! of the route. This module implements algorithms to do that, building on the
//! C++ version of GeographicLib.

use std::ops::{Mul, Sub};

use dimensioned::si::{M, Meter};
use thiserror::Error;

use crate::geographic::{
    GeographicError, geodesic_direct, geodesic_inverse, gnomonic_forward, gnomonic_reverse,
};
use crate::types::{GeoAndXyzPoint, GeoPoint, GeoSegment, HasGeoPoint, HasXyzPoint, XyPoint};

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum AlgorithmError {
    #[error("Geographic computation")]
    Geographic(#[from] GeographicError),
}

type Result<T> = std::result::Result<T, AlgorithmError>;

/// Compute a point of interception along a geodesic segment
///
/// Given a geodesic segment and another point, returns the point on the segment
/// with the minimum geodesic distance from the other point. Depending on the
/// relative positions of the input segment and point, the point of interception
/// may be located at one of the geodesic's endpoints.
///
/// Note that because of this function's reliance on the gnomonic projection, it
/// can give incorrect results for points very far away from each other.
///
/// # Algorithm description
///
/// Charles Karney gave an illustration of this problem as an anti-aircraft
/// battery identifying the point along an enemy plane's trajectory at which it
/// would be nearest to its missiles.
///
/// At a high level, this function:
///
/// 1. Takes an initial guess at the point of interception on the geodesic.
/// 2. Uses a gnomonic projection centered on the guess to put the geodesic
///    segment and the other point on a 2D plane.
/// 3. Uses 2D geometry to get a better guess at the interception point.
/// 4. Repeats a few times, each time re-centering the projection on the updated
///    guess.
///
/// # References
///
/// Karney described this approach in a forum post here:
/// <https://sourceforge.net/p/geographiclib/discussion/1026621/thread/21aaff9f/#8a93>
///
/// (However, here I use different linear algebra to find the interception than
/// in his example code.)
///
/// For a more detailed description, see: <http://arxiv.org/abs/1102.1215>
pub fn karney_interception<P>(segment: &GeoSegment<P>, point: &P) -> Result<GeoPoint>
where
    P: HasGeoPoint,
{
    // Start with an initial guess of an intercept at the geodesic's midpoint:
    let mut intercept = geodesic_direct(
        segment.start.geo(),
        segment.start_azimuth,
        segment.geo_length / 2.0,
    )?
    .point2;

    // TODO: Experiment with different numbers of gnomonic iterations
    //
    // I'm just using 10 iterations here because it's what Karney suggested in
    // his example solution to the interception problem, and because it works ok
    // in practice.  But it'd be worth checking whether a smaller number might
    // work.
    for _ in 0..10 {
        let start = gnomonic_forward(&intercept, segment.start.geo())?;
        let end = gnomonic_forward(&intercept, segment.end.geo())?;
        let p = gnomonic_forward(&intercept, point.geo())?;
        let b = subtract_xypoints(&end, &start);
        let a = subtract_xypoints(&p, &start);

        let v = if dot2(a, b) <= 0.0 {
            Vec2 { x: 0.0, y: 0.0 }
        } else {
            let a_proj = b * (dot2(a, b) / dot2(b, b));
            if dot2(a_proj, a_proj) >= dot2(b, b) {
                b
            } else {
                a_proj
            }
        };

        intercept = gnomonic_reverse(
            &intercept,
            &XyPoint {
                x: (start.x.value_unsafe + v.x) * M,
                y: (start.y.value_unsafe + v.y) * M,
            },
        )?;
    }

    Ok(intercept)
}

/// Returns a floor for geodesic interception distance
///
/// Given a geodesic segment and a separate point, this computes a lower bound
/// for the minimum distance between the two.  When processing a lot of
/// waypoints and route segments but only interested in segments within a
/// certain distance of points, this provides a very significant speedup
/// compared to running [`karney_interception`] between every combination of
/// segment and point.
///
/// This takes into account edge cases in which a very long segment, combined
/// with a very nearby waypoint, may result in the cartesian intercept distance
/// actually being longer than the real geodesic intercept distance.
pub fn intercept_distance_floor<P>(
    segment: &GeoSegment<GeoAndXyzPoint>,
    point: &P,
) -> Result<Meter<f64>>
where
    P: HasXyzPoint,
{
    let dist = cartesian_intercept_distance(segment, point)?;
    let depth = max_chord_depth(segment);

    // Fuzzing with quickcheck revealed that--presumably due to limits of
    // numeric precision--in some cases where test points were nearby one of a
    // short segment's endpoints, the floor would actually be larger by about a
    // nanometer than the result computed by karney_intercept.
    //
    // So on top of the chord depth, here we also subtract a micrometer of
    // padding from our lower bound for the intercept distance, after which I
    // can no longer reproduce that error.
    Ok(dist - (depth + 0.000_001 * M))
}

const WGS84_A: f64 = 6378137.0;
const WGS84_F: f64 = 1.0 / 298.257223563;
const WGS84_B: f64 = WGS84_A * (1.0 - WGS84_F);

/// The maximum possible depth on WGS84 of a chord with a given length
///
/// See the Mathematica notebook for rationale.
fn max_chord_depth(segment: &GeoSegment<GeoAndXyzPoint>) -> Meter<f64> {
    let chord_length = norm3(subtract_xyzpoints(&segment.start.xyz(), &segment.end.xyz()));
    WGS84_A * (1.0 - (1.0 - chord_length * chord_length / (4.0 * WGS84_B * WGS84_B)).sqrt()) * M
}

fn cartesian_intercept_distance<P>(
    segment: &GeoSegment<GeoAndXyzPoint>,
    point: &P,
) -> Result<Meter<f64>>
where
    P: HasXyzPoint,
{
    let b = subtract_xyzpoints(&segment.end.xyz(), &segment.start.xyz());
    let a = subtract_xyzpoints(point, segment.start.xyz());

    // The same calculation used to find segment intercepts in each iteration of
    // [`karney_intercept`], just in 3D space!
    let intercept = if dot3(a, b) <= 0.0 {
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    } else {
        let a_proj = b * (dot3(a, b) / dot3(b, b));
        if dot3(a_proj, a_proj) >= dot3(b, b) {
            b
        } else {
            a_proj
        }
    };
    Ok(norm3(a - intercept) * M)
}

/// A segment of a course whose distance from a waypoint has been measured.
pub trait NearbySegment<D>
where
    Self: Copy,
    D: Copy + PartialOrd,
{
    /// The segment's minimum geodesic distance from the waypoint.
    fn waypoint_distance(self) -> D;
}

/// Identifies the course segments within some threshold distance of a waypoint.
///
/// Operates on a sequence of segments that together describe a course. The
/// [`NearbySegment`] trait is used to determine each segment's distance from
/// the point.  After identifying spans of the segment collection that pass
/// within `threshold` of whatever waypoint they've been measured against, we
/// return the segment that passes the closest within each span.
///
/// # Example
///
/// ```rust,ignore
/// use coursepointer::algorithm::NearbySegment;
/// use coursepointer::algorithm::find_nearby_segments;
///
/// #[derive(PartialEq, Debug)]
/// struct Seg(char, i32);
///
/// impl NearbySegment<i32> for Seg {
///     fn waypoint_distance(&self) -> i32 {
///        self.1
///    }
/// }
///
/// let segments = vec![
///     Seg('a', 9),
///     Seg('b', 5), // <-- Course passes within threshold starting here
///     Seg('c', 2), // <-- Span minimum here
///     Seg('d', 4), // <-- Still below threshold
///     Seg('e', 7),
///     Seg('f', 4), // <-- Minimum for a new span of segments below threshold
///     Seg('g', 6),
/// ];
/// let result = find_nearby_segments(segments, 5);
/// assert_eq!(result, vec![Seg('c', 2), Seg('f', 4)]);
/// ```
pub fn find_nearby_segments<I, T, D>(segments: I, threshold: D) -> Vec<T>
where
    T: NearbySegment<D>,
    I: IntoIterator<Item = T>,
    D: Copy + PartialOrd,
{
    let mut result: Vec<T> = Vec::new();
    let mut span_min: Option<T> = None;
    for segment in segments.into_iter() {
        if segment.waypoint_distance() <= threshold {
            match &span_min {
                Some(current_min) => {
                    if segment.waypoint_distance() < current_min.waypoint_distance() {
                        span_min = Some(segment);
                    }
                }
                None => span_min = Some(segment),
            }
        } else if let Some(current_min) = span_min {
            result.push(current_min);
            span_min = None;
        }
    }
    if let Some(current_min) = span_min {
        result.push(current_min);
    }
    result
}

pub trait FromGeoPoints<'a, P>
where
    Self: Sized,
    P: HasGeoPoint + TryFrom<GeoPoint>,
{
    fn from_geo_points(start: &'a P, end: &'a P) -> std::result::Result<Self, GeographicError>;
}

impl<'a, P> FromGeoPoints<'a, P> for GeoSegment<'a, P>
where
    P: HasGeoPoint + TryFrom<GeoPoint>,
{
    fn from_geo_points(point1: &'a P, point2: &'a P) -> std::result::Result<Self, GeographicError> {
        let inverse = geodesic_inverse(point1.geo(), point2.geo())?;
        Ok(GeoSegment {
            start: point1,
            end: point2,
            geo_length: inverse.geo_distance,
            start_azimuth: inverse.azimuth1,
        })
    }
}

#[derive(Clone, Copy)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Mul<f64> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

fn subtract_xypoints(a: &XyPoint, b: &XyPoint) -> Vec2 {
    Vec2 {
        x: a.x.value_unsafe - b.x.value_unsafe,
        y: a.y.value_unsafe - b.y.value_unsafe,
    }
}

fn dot2(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

#[derive(Clone, Copy, Debug)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Mul<f64> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl Sub<Vec3> for Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: Vec3) -> Self::Output {
        Vec3 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

fn subtract_xyzpoints<P, Q>(a: &P, b: &Q) -> Vec3
where
    P: HasXyzPoint,
    Q: HasXyzPoint,
{
    Vec3 {
        x: a.xyz().x.value_unsafe - b.xyz().x.value_unsafe,
        y: a.xyz().y.value_unsafe - b.xyz().y.value_unsafe,
        z: a.xyz().z.value_unsafe - b.xyz().z.value_unsafe,
    }
}

fn dot3(a: Vec3, b: Vec3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn norm3(vec: Vec3) -> f64 {
    dot3(vec, vec).sqrt()
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;
    use std::path::PathBuf;

    use anyhow::Result;
    use approx::assert_relative_eq;
    use dimensioned::si::{M, Meter};
    use quickcheck::{Arbitrary, Gen, TestResult};
    use quickcheck_macros::quickcheck;
    use serde::Deserialize;

    use super::{
        FromGeoPoints, NearbySegment, cartesian_intercept_distance, find_nearby_segments,
        intercept_distance_floor, karney_interception,
    };
    use crate::geographic::{geocentric_forward, geodesic_direct, geodesic_inverse};
    use crate::measure::DEG;
    use crate::types::{GeoAndXyzPoint, GeoPoint, GeoSegment, XyzPoint};

    #[derive(Deserialize)]
    struct InterceptsDatum {
        geo_start_lat: f64,
        geo_start_lon: f64,
        geo_end_lat: f64,
        geo_end_lon: f64,
        p_lat: f64,
        p_lon: f64,
        intercept_lat: f64,
        intercept_lon: f64,
        _d: f64,
    }

    #[test]
    fn test_karney_interception() -> Result<()> {
        // Random test cases from the `docs/Course Point Distances.nb`
        // Mathematica notebook:
        let intercepts_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("testdata")
            .join("intercepts.csv");

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_path(intercepts_path)?;
        for case in rdr.deserialize() {
            let datum: InterceptsDatum = case?;
            let geo_start =
                GeoPoint::new(datum.geo_start_lat * DEG, datum.geo_start_lon * DEG, None)?;
            let geo_end = GeoPoint::new(datum.geo_end_lat * DEG, datum.geo_end_lon * DEG, None)?;
            let p = GeoPoint::new(datum.p_lat * DEG, datum.p_lon * DEG, None)?;
            let intercept =
                GeoPoint::new(datum.intercept_lat * DEG, datum.intercept_lon * DEG, None)?;

            let seg = GeoSegment::from_geo_points(&geo_start, &geo_end)?;
            let result = karney_interception(&seg, &p)?;

            assert_relative_eq!(result, intercept, epsilon = 0.000_001);
        }

        Ok(())
    }

    #[test]
    fn test_karney_interception_zero_length_segment() -> Result<()> {
        let seg_point = GeoPoint::new(3.0 * DEG, 4.0 * DEG, None)?;
        let seg = GeoSegment::from_geo_points(&seg_point, &seg_point)?;
        let p = GeoPoint::new(3.5 * DEG, 4.5 * DEG, None)?;
        let intercept = karney_interception(&seg, &p)?;

        // For a zero-length segment, the intercept should be the segment's
        // start and end point.
        assert_relative_eq!(intercept, seg_point);
        Ok(())
    }

    #[test]
    fn test_karney_interception_point_on_segment() -> Result<()> {
        let point1 = GeoPoint::new(3.0 * DEG, 4.0 * DEG, None)?;
        let point2 = GeoPoint::new(3.5 * DEG, 4.5 * DEG, None)?;
        let seg = GeoSegment::from_geo_points(&point1, &point2)?;
        let intercept = karney_interception(&seg, &point1)?;

        assert_relative_eq!(intercept, point1);
        Ok(())
    }

    impl NearbySegment<i32> for (char, i32) {
        fn waypoint_distance(self) -> i32 {
            self.1
        }
    }

    #[test]
    fn test_intercepted_segments_multiple_matches() {
        let segments = vec![
            ('a', 10),
            ('b', 8), // <-- Local minimum above threshold
            ('c', 11),
            ('d', 7),
            ('e', 4),
            ('f', 2), // <-- Local minimum below threshold
            ('g', 5),
            ('h', 7),
            ('i', 7),
            ('j', 8),
            ('k', 2),
            ('l', 1), // <-- Another minimum below the threshold
            ('m', 2),
            ('n', 1),
        ];
        let result = find_nearby_segments(segments, 5)
            .into_iter()
            .map(|(c, _)| c)
            .collect::<Vec<_>>();
        assert_eq!(result, vec!['f', 'l']);
    }

    #[test]
    fn test_intercepted_segments_empty() {
        let segments: Vec<(char, i32)> = Vec::new();
        let result = find_nearby_segments(segments, 5)
            .into_iter()
            .map(|(c, _)| c)
            .collect::<Vec<_>>();
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_intercepted_segments_single_match() {
        let segments = vec![('a', 10), ('b', 8), ('c', 5), ('d', 6)];
        let result = find_nearby_segments(segments, 5)
            .into_iter()
            .map(|(c, _)| c)
            .collect::<Vec<_>>();
        assert_eq!(result, vec!['c']);
    }

    #[test]
    fn test_intercepted_segments_ending_match() {
        let segments = vec![('a', 10), ('b', 8), ('c', 6), ('d', 4)];
        let result = find_nearby_segments(segments, 5)
            .into_iter()
            .map(|(c, _)| c)
            .collect::<Vec<_>>();
        assert_eq!(result, vec!['d']);
    }

    #[test]
    fn test_cartesian_intercept_distance() -> Result<()> {
        let intercepts_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("testdata")
            .join("intercepts_near.csv");

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_path(intercepts_path)?;
        for case in rdr.deserialize() {
            let datum: InterceptsDatum = case?;
            let geo_start: GeoAndXyzPoint =
                GeoPoint::new(datum.geo_start_lat * DEG, datum.geo_start_lon * DEG, None)?
                    .try_into()?;
            let geo_end: GeoAndXyzPoint =
                GeoPoint::new(datum.geo_end_lat * DEG, datum.geo_end_lon * DEG, None)?
                    .try_into()?;
            let intercept =
                GeoPoint::new(datum.intercept_lat * DEG, datum.intercept_lon * DEG, None)?;
            let geo_point = GeoPoint::new(datum.p_lat * DEG, datum.p_lon * DEG, None)?;
            let intercept_distance = geodesic_inverse(&geo_point, &intercept)?.geo_distance;

            let p_geo = GeoPoint::new(datum.p_lat * DEG, datum.p_lon * DEG, None)?;
            let p_xyz = geocentric_forward(&p_geo)?;
            let seg = GeoSegment::from_geo_points(&geo_start, &geo_end)?;
            let result = cartesian_intercept_distance(&seg, &p_xyz)?;

            // For nearby points and geodesics, the linear estimate and the
            // actual geodesic distance should be somewhat close.
            assert_relative_eq!(
                result.value_unsafe,
                intercept_distance.value_unsafe,
                max_relative = 0.001
            );
        }
        Ok(())
    }

    // Tests below are to verify the crucial property that the distance returned
    // by intercept_distance_floor is in fact no greater than the geodesic
    // intercept distance produced by karney_intercept.

    #[test]
    fn test_intercept_distance_floor() -> Result<()> {
        let intercepts_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("testdata")
            .join("intercepts.csv");

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_path(intercepts_path)?;
        for case in rdr.deserialize() {
            let datum: InterceptsDatum = case?;
            let geo_start: GeoAndXyzPoint =
                GeoPoint::new(datum.geo_start_lat * DEG, datum.geo_start_lon * DEG, None)?
                    .try_into()?;
            let geo_end: GeoAndXyzPoint =
                GeoPoint::new(datum.geo_end_lat * DEG, datum.geo_end_lon * DEG, None)?
                    .try_into()?;
            let p = GeoPoint::new(datum.p_lat * DEG, datum.p_lon * DEG, None)?;
            let intercept =
                GeoPoint::new(datum.intercept_lat * DEG, datum.intercept_lon * DEG, None)?;
            let geo_point = GeoPoint::new(datum.p_lat * DEG, datum.p_lon * DEG, None)?;
            let intercept_distance = geodesic_inverse(&geo_point, &intercept)?.geo_distance;

            let seg = GeoSegment::from_geo_points(&geo_start, &geo_end)?;
            let floor = intercept_distance_floor(&seg, &XyzPoint::try_from(p)?)?;

            assert!(
                floor <= intercept_distance,
                "floor = {floor}, intercept_distance = {intercept_distance}"
            );
        }

        Ok(())
    }

    impl Arbitrary for GeoPoint {
        fn arbitrary(_g: &mut Gen) -> Self {
            let lat = rand::random_range(-90.0..=90.0) * DEG;
            let lon = rand::random_range(-180.0..=180.0) * DEG;
            GeoPoint::new(lat, lon, None).unwrap()
        }
    }

    /// A setup for solving the intercept problem
    ///
    /// Consists of a geodesic segment's start and end points, and a third point
    /// somewhere.
    #[derive(Copy, Clone, Debug)]
    struct InterceptProblem {
        /// Geodesic segment start point
        s1: GeoPoint,

        /// Geodesic segment end point
        s2: GeoPoint,

        /// Intercepting point
        p: GeoPoint,
    }

    /// A type that can provide an [`InterceptProblem`]
    ///
    /// Different implementations will have different approaches to generating
    /// arbitrary problems for quickcheck.
    trait HasInterceptProblem<'a> {
        fn prob(&'a self) -> &'a InterceptProblem;
    }

    impl<'a> HasInterceptProblem<'a> for GlobalInterceptProblem {
        fn prob(&'a self) -> &'a InterceptProblem {
            &self.prob
        }
    }

    fn check_intercept_problem<H>(h: H) -> Result<TestResult>
    where
        for<'a> H: HasInterceptProblem<'a>,
    {
        let s1_xyz: GeoAndXyzPoint = h.prob().s1.try_into()?;
        let s2_xyz: GeoAndXyzPoint = h.prob().s2.try_into()?;
        let seg = GeoSegment::<GeoAndXyzPoint>::from_geo_points(&s1_xyz, &s2_xyz)?;
        let p = GeoAndXyzPoint::try_from(h.prob().p)?;

        let intercept_point = karney_interception(&seg, &p)?;
        let intercept_dist = geodesic_inverse(&intercept_point, &h.prob().p)?.geo_distance;

        if intercept_distance_floor(&seg, &p)? > intercept_dist {
            Ok(TestResult::failed())
        } else {
            Ok(TestResult::passed())
        }
    }

    /// An intercept problem where the points are anywhere on the globe
    #[derive(Clone, Debug)]
    struct GlobalInterceptProblem {
        prob: InterceptProblem,
    }

    impl Arbitrary for GlobalInterceptProblem {
        fn arbitrary(g: &mut Gen) -> Self {
            Self {
                prob: InterceptProblem {
                    s1: GeoPoint::arbitrary(g),
                    s2: GeoPoint::arbitrary(g),
                    p: GeoPoint::arbitrary(g),
                },
            }
        }
    }

    #[quickcheck]
    fn qc_global_intercept_floor(h: GlobalInterceptProblem) -> Result<TestResult> {
        // In the case of the global problem, we must separately make sure the
        // points are all within at most 6,000 km of each other or the gnomonic
        // projection starts to break down.
        fn dis(p1: &GeoPoint, p2: &GeoPoint) -> Result<bool> {
            Ok(geodesic_inverse(p1, p2)?.geo_distance > 6_000_000.0 * M)
        }
        if dis(&h.prob.s1, &h.prob.s2)?
            || dis(&h.prob.s1, &h.prob.p)?
            || dis(&h.prob.s2, &h.prob.p)?
        {
            return Ok(TestResult::discard());
        }
        check_intercept_problem(h)
    }

    /// An intercept problem where the points are within a local radius
    #[derive(Clone, Debug)]
    struct LocalInterceptProblem<R: Radius> {
        problem: InterceptProblem,
        r: PhantomData<R>,
    }

    trait Radius: Clone + 'static {
        fn radius() -> Meter<f64>;
    }

    impl<R: Radius> LocalInterceptProblem<R> {
        fn arbitrary_point_within(p0: &GeoPoint) -> Result<GeoPoint> {
            let d = rand::random_range(0.0..R::radius().value_unsafe) * M;
            let azimuth = rand::random_range(0.0..360.0) * DEG;
            Ok(geodesic_direct(p0, azimuth, d)?.point2)
        }
    }

    impl<'a, R: Radius> HasInterceptProblem<'a> for LocalInterceptProblem<R> {
        fn prob(&'a self) -> &'a InterceptProblem {
            &self.problem
        }
    }

    impl<R: Radius> Arbitrary for LocalInterceptProblem<R> {
        fn arbitrary(g: &mut Gen) -> Self {
            let p0 = GeoPoint::arbitrary(g);
            Self {
                problem: InterceptProblem {
                    s1: Self::arbitrary_point_within(&p0).unwrap(),
                    s2: Self::arbitrary_point_within(&p0).unwrap(),
                    p: Self::arbitrary_point_within(&p0).unwrap(),
                },
                r: PhantomData::<R>,
            }
        }
    }

    macro_rules! qc_local_intercept_floor {
        ($name:tt, $radius:expr) => {
            paste::paste! {
                #[derive(Clone, Debug)]
                struct [<Radius $name>] {}

                impl Radius for [<Radius $name>] {
                    fn radius() -> Meter<f64> {
                        $radius
                    }
                }

                #[quickcheck]
                fn [<qc_ $name _intercept_floor>](prob: LocalInterceptProblem<[<Radius $name>]>) -> Result<TestResult> {
                    check_intercept_problem(prob)
                }
            }
        }
    }

    qc_local_intercept_floor![10cm, 0.1 * M];
    qc_local_intercept_floor![1m, 1.0 * M];
    qc_local_intercept_floor![10m, 10.0 * M];
    qc_local_intercept_floor![100m, 100.0 * M];
    qc_local_intercept_floor![1km, 1_000.0 * M];
    qc_local_intercept_floor![10km, 10_000.0 * M];
}
