//! Algorithms for geodesic interception
//!
//! To compute course points' distances and positions along a course, we need to
//! solve the "interception problem" between the course point and the segments
//! of the route. This module implements algorithms to do that, building on the
//! C++ version of GeographicLib.

use std::ops::Mul;

use coretypes::measure::Meters;
use coretypes::{GeoPoint, GeoSegment, XYPoint};
use geographic::{
    GeographicError, geodesic_direct, geodesic_inverse, gnomonic_forward, gnomonic_reverse,
};
use thiserror::Error;

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
pub fn karney_interception(geodesic: &GeoSegment, point: &GeoPoint) -> Result<GeoPoint> {
    // TODO: Remove duplicate solution of geodesic inverse
    let seg = geodesic_inverse(&geodesic.point1, &geodesic.point2)?;

    // Start with an initial guess of an intercept at the geodesic's midpoint:
    let mut intercept =
        geodesic_direct(&geodesic.point1, seg.azimuth1, seg.geo_distance / 2.0)?.point2;

    for _ in 0..10 {
        let start = gnomonic_forward(&intercept, &geodesic.point1)?;
        let end = gnomonic_forward(&intercept, &geodesic.point2)?;
        let p = gnomonic_forward(&intercept, point)?;
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
            &XYPoint {
                x: Meters(start.x.0 + v.x),
                y: Meters(start.y.0 + v.y),
            },
        )?;
    }

    Ok(intercept)
}

/// A segment of a course whose distance from a point has been measured.
pub trait MeasuredSegment<D>
where
    D: Copy + PartialOrd,
{
    fn measure(&self) -> D;
}

/// Finds the segments of a course intercepted within some threshold of distance
/// from a point.
///
/// Operates on a sequence of segments describing a course.  The
/// [`MeasuredSegment`] trait is used to determine each segment's distance from
/// the point, which
///
/// # Example
///
/// ```
/// use coursepointer::algorithm::MeasuredSegment;
/// use coursepointer::algorithm::intercepted_segments;
///
/// #[derive(PartialEq, Debug)]
/// struct Seg(char, i32);
///
/// impl MeasuredSegment<i32> for Seg {
///     fn measure(&self) -> i32 {
///        self.1
///    }
/// }
///
/// let segments = vec![
///     Seg('a', 11),
///     Seg('b', 7),
///     Seg('c', 5), // <-- Course passes within threshold starting here
///     Seg('d', 2), // <-- Local minimum here
///     Seg('e', 4),
///     Seg('f', 7),
/// ];
/// let result = intercepted_segments(segments, 5);
/// assert_eq!(result, vec![Seg('d', 2)]);
/// ```
pub fn intercepted_segments<I, T, D>(segments: I, threshold: D) -> Vec<T>
where
    T: MeasuredSegment<D>,
    I: IntoIterator<Item = T>,
    D: Copy + PartialOrd,
{
    let mut result: Vec<T> = Vec::new();
    let mut span_min: Option<T> = None;
    for segment in segments.into_iter() {
        if segment.measure() <= threshold {
            match &span_min {
                Some(current_min) => {
                    if segment.measure() < current_min.measure() {
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

#[derive(Clone, Copy)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Mul<f64> for Vec2 {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self {
            x: self.x * other,
            y: self.y * other,
        }
    }
}

fn subtract_xypoints(a: &XYPoint, b: &XYPoint) -> Vec2 {
    Vec2 {
        x: a.x.0 - b.x.0,
        y: a.y.0 - b.y.0,
    }
}

fn dot2(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use anyhow::Result;
    use approx::assert_relative_eq;
    use coretypes::measure::Degrees;
    use coretypes::{GeoPoint, GeoSegment};
    use serde::Deserialize;

    use crate::algorithm::{MeasuredSegment, intercepted_segments, karney_interception};

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
            let geo_start = GeoPoint::new(
                Degrees(datum.geo_start_lat),
                Degrees(datum.geo_start_lon),
                None,
            )?;
            let geo_end =
                GeoPoint::new(Degrees(datum.geo_end_lat), Degrees(datum.geo_end_lon), None)?;
            let p = GeoPoint::new(Degrees(datum.p_lat), Degrees(datum.p_lon), None)?;
            let intercept = GeoPoint::new(
                Degrees(datum.intercept_lat),
                Degrees(datum.intercept_lon),
                None,
            )?;

            let seg = GeoSegment {
                point1: geo_start,
                point2: geo_end,
            };
            let result = karney_interception(&seg, &p)?;

            assert_relative_eq!(result, intercept, max_relative = 0.000_000_100);
        }

        Ok(())
    }

    #[test]
    fn test_karney_interception_zero_length_segment() -> Result<()> {
        let seg_point = GeoPoint::new(Degrees(3.0), Degrees(4.0), None)?;
        let seg = GeoSegment {
            point1: seg_point,
            point2: seg_point,
        };
        let p = GeoPoint::new(Degrees(3.5), Degrees(4.5), None)?;
        let intercept = karney_interception(&seg, &p)?;

        // For a zero-length segment, the intercept should be the segment's
        // start and end point.
        assert_relative_eq!(intercept, seg_point);
        Ok(())
    }

    #[test]
    fn test_karney_interception_point_on_segment() -> Result<()> {
        let point1 = GeoPoint::new(Degrees(3.0), Degrees(4.0), None)?;
        let point2 = GeoPoint::new(Degrees(3.5), Degrees(4.5), None)?;
        let seg = GeoSegment { point1, point2 };
        let intercept = karney_interception(&seg, &point1)?;

        assert_relative_eq!(intercept, point1);
        Ok(())
    }

    impl MeasuredSegment<i32> for (char, i32) {
        fn measure(&self) -> i32 {
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
        let result = intercepted_segments(segments, 5)
            .into_iter()
            .map(|(c, _)| c)
            .collect::<Vec<_>>();
        assert_eq!(result, vec!['f', 'l']);
    }

    #[test]
    fn test_intercepted_segments_empty() {
        let segments: Vec<(char, i32)> = Vec::new();
        let result = intercepted_segments(segments, 5)
            .into_iter()
            .map(|(c, _)| c)
            .collect::<Vec<_>>();
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_intercepted_segments_single_match() {
        let segments = vec![('a', 10), ('b', 8), ('c', 5), ('d', 6)];
        let result = intercepted_segments(segments, 5)
            .into_iter()
            .map(|(c, _)| c)
            .collect::<Vec<_>>();
        assert_eq!(result, vec!['c']);
    }

    #[test]
    fn test_intercepted_segments_ending_match() {
        let segments = vec![('a', 10), ('b', 8), ('c', 6), ('d', 4)];
        let result = intercepted_segments(segments, 5)
            .into_iter()
            .map(|(c, _)| c)
            .collect::<Vec<_>>();
        assert_eq!(result, vec!['d']);
    }
}
