//! Algorithms for geodesic interception
//!
//! To compute course points' distances and positions along a course, we need to
//! solve the "interception problem" between the course point and the segments
//! of the route. This module implements algorithms to do that, building on the
//! C++ version of GeographicLib.

use std::ops::{Mul, Sub};

use dimensioned::si::{M, Meter};
use thiserror::Error;
use tracing::instrument;

use crate::course::CourseError;
use crate::geographic::{
    GeographicError, geodesic_direct, geodesic_inverse, gnomonic_forward, gnomonic_reverse,
};
use crate::types::{GeoAndXyzPoint, GeoPoint, GeoSegment, HasGeoPoint, HasXyzPoint, XyPoint};

#[derive(Error, Debug)]
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
#[instrument(level = "trace", skip_all)]
pub fn karney_interception<P>(segment: &GeoSegment<P>, point: &P) -> Result<GeoPoint>
where
    P: HasGeoPoint,
    CourseError: From<<P as TryFrom<GeoPoint>>::Error>,
{
    // Start with an initial guess of an intercept at the geodesic's midpoint:
    let mut intercept = geodesic_direct(
        segment.point1.geo(),
        segment.azimuth1,
        segment.geo_distance / 2.0,
    )?
    .point2;

    for _ in 0..10 {
        let start = gnomonic_forward(&intercept, segment.point1.geo())?;
        let end = gnomonic_forward(&intercept, segment.point2.geo())?;
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
    Ok(dist - depth)
}

const WGS84_A: f64 = 6378137.0;
const WGS84_F: f64 = 1.0 / 298.257223563;
const WGS84_B: f64 = WGS84_A * (1.0 - WGS84_F);

fn max_chord_depth(segment: &GeoSegment<GeoAndXyzPoint>) -> Meter<f64> {
    let chord_length = norm3(subtract_xyzpoints(
        &segment.point1.xyz(),
        &segment.point2.xyz(),
    ));
    WGS84_A * (1.0 - (1.0 - chord_length * chord_length / (4.0 * WGS84_B * WGS84_B)).sqrt()) * M
}

fn cartesian_intercept_distance<P>(
    segment: &GeoSegment<GeoAndXyzPoint>,
    point: &P,
) -> Result<Meter<f64>>
where
    P: HasXyzPoint,
{
    let b = subtract_xyzpoints(&segment.point2.xyz(), &segment.point1.xyz());
    let a = subtract_xyzpoints(point, segment.point1.xyz());
    let intercept = if dot3(a, b) <= 0.0 {
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    } else {
        let a_proj = b * (dot3(a, b) / dot3(b, b));
        if dot3(a_proj, a_proj) < dot3(b, b) {
            a_proj
        } else {
            b
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
/// ```
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

pub trait FromGeoPoints<P>
where
    Self: Sized,
    P: HasGeoPoint,
    CourseError: From<<P as TryFrom<GeoPoint>>::Error>,
{
    fn from_geo_points(point1: P, point2: P) -> std::result::Result<Self, GeographicError>;
}

impl<P> FromGeoPoints<P> for GeoSegment<P>
where
    P: HasGeoPoint,
    CourseError: From<<P as TryFrom<GeoPoint>>::Error>,
{
    fn from_geo_points(point1: P, point2: P) -> std::result::Result<Self, GeographicError> {
        let inverse = geodesic_inverse(point1.geo(), point2.geo())?;
        Ok(GeoSegment {
            point1,
            point2,
            geo_distance: inverse.geo_distance,
            azimuth1: inverse.azimuth1,
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
    use std::path::PathBuf;

    use anyhow::Result;
    use approx::assert_relative_eq;
    use serde::Deserialize;

    use super::{
        FromGeoPoints, NearbySegment, cartesian_intercept_distance, find_nearby_segments,
        intercept_distance_floor, karney_interception,
    };
    use crate::geographic::{geocentric_forward, geodesic_inverse};
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

            let seg = GeoSegment::from_geo_points(geo_start, geo_end)?;
            let result = karney_interception(&seg, &p)?;

            assert_relative_eq!(result, intercept, epsilon = 0.000_001);
        }

        Ok(())
    }

    #[test]
    fn test_karney_interception_zero_length_segment() -> Result<()> {
        let seg_point = GeoPoint::new(3.0 * DEG, 4.0 * DEG, None)?;
        let seg = GeoSegment::from_geo_points(seg_point, seg_point)?;
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
        let seg = GeoSegment::from_geo_points(point1, point2)?;
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
            let seg = GeoSegment::from_geo_points(geo_start, geo_end)?;
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

            let seg = GeoSegment::from_geo_points(geo_start, geo_end)?;
            let floor = intercept_distance_floor(&seg, &XyzPoint::try_from(p)?)?;

            assert!(
                floor <= intercept_distance,
                "floor = {}, intercept_distance = {}",
                floor,
                intercept_distance
            );
        }

        Ok(())
    }
}
