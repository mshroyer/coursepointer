use std::fs::File;
use std::io;
use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use coursepointer::internal::{
    FromGeoPoints, GeoAndXyzPoint, GeoSegment, geodesic_direct, geodesic_inverse,
    intercept_distance_floor, karney_interception,
};
use coursepointer::{DEG, Degree, GeoPoint};
use dimensioned::si::{M, Meter};
use indicatif::ParallelProgressIterator;
use rayon::prelude::*;
use sobol::Sobol;
use sobol::params::JoeKuoD6;

#[derive(Debug)]
struct GeoTestCase {
    pub segment: GeoSegment<GeoAndXyzPoint>,
    pub point: GeoAndXyzPoint,
}

impl GeoTestCase {
    pub fn try_from_sample(range: Meter<f64>, sample: [f64; 5]) -> Result<Self> {
        let s_lat = sample[0] * 90.0 * DEG;
        let s_azi = sample[1] * 90.0 * DEG;
        let s_len = sample[2] * range;
        let p_off_azi = sample[3] * 360.0 * DEG;
        let p_off_len = sample[4] * range;

        let sm = GeoPoint::new(s_lat, 0.0 * DEG, None)?;
        let geo_p = geodesic_direct(&sm, p_off_azi, p_off_len)?.point2;
        let p = GeoAndXyzPoint::try_from(geo_p)?;
        let s0 = GeoAndXyzPoint::try_from(
            geodesic_direct(&sm, reverse_azimuth(s_azi), s_len / 2.0)?.point2,
        )?;
        let s1 = GeoAndXyzPoint::try_from(geodesic_direct(&sm, s_azi, s_len / 2.0)?.point2)?;
        let s = GeoSegment::from_geo_points(s0, s1)?;
        Ok(Self {
            segment: s,
            point: p,
        })
    }
}

fn reverse_azimuth(azi: Degree<f64>) -> Degree<f64> {
    ((azi.value_unsafe + 180.0) % 360.0) * DEG
}

mod adaptive {
    use std::ops::Mul;

    use anyhow::Result;
    use coursepointer::GeoPoint;
    use coursepointer::internal::{
        GeoAndXyzPoint, GeoSegment, XyPoint, XyzPoint, geocentric_forward, geodesic_direct,
        gnomonic_forward, gnomonic_reverse,
    };
    use dimensioned::si::{M, Meter};

    /// Compute gnomonic interception with adaptive precision
    ///
    /// A modified version of `karney_interception` from algorithm.rs that iterates until the point
    /// of interception converges to within some specified Euclidean precision.  The number of
    /// iterations performed is returned alongside the resulting [`GeoPoint`].
    pub fn karney_interception(
        precision: Meter<f64>,
        segment: &GeoSegment<GeoAndXyzPoint>,
        point: &GeoAndXyzPoint,
    ) -> Result<(GeoPoint, usize)> {
        // Start with an initial guess of an intercept at the geodesic's
        // midpoint:
        let mut intercept = geodesic_direct(
            &segment.start.geo,
            segment.start_azimuth,
            segment.geo_length / 2.0,
        )?
        .point2;

        let mut prev_xyzpoint: Option<XyzPoint> = None;

        for iteration in 1..11usize {
            let start = gnomonic_forward(&intercept, &segment.start.geo)?;
            let end = gnomonic_forward(&intercept, &segment.end.geo)?;
            let p = gnomonic_forward(&intercept, &point.geo)?;
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

            let cur = geocentric_forward(&intercept)?;
            if let Some(prev) = prev_xyzpoint {
                let d = ((cur.x.value_unsafe - prev.x.value_unsafe).powi(2)
                    + (cur.y.value_unsafe - prev.y.value_unsafe).powi(2)
                    + (cur.z.value_unsafe - prev.z.value_unsafe).powi(2))
                .sqrt()
                    * M;

                if d <= precision {
                    return Ok((intercept, iteration));
                }
            }
            prev_xyzpoint = Some(cur);
        }

        Ok((intercept, 11usize))
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
}

/// Validates algorithm correctness using random sampling.
#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate and print an example test case
    Generate {},

    /// Validate the intercept floor calculation
    Floor {
        /// The number of test cases to check
        n: usize,
    },

    /// Validate the new intercept floor calculation
    /// Output raw samples of the Sobol sequence for examination
    RawSample {
        /// The number of samples to take
        n: usize,

        /// Where to write output; stdout is used if unset
        #[arg(short, long)]
        out: Option<PathBuf>,
    },

    /// Experiment with adaptive-precision Karney interceptions, testing how many iterations are
    /// required to reach any given level of precision.
    AdaptiveInterception {
        /// The number of samples to take
        n: usize,

        /// The desired result precision, in meters.
        precision_m: f64,

        /// Where to write output; stdout is used if unset
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
}

fn verify_floor<F>(f: F, n: usize) -> Result<()>
where
    F: Fn(&GeoSegment<GeoAndXyzPoint>, &GeoAndXyzPoint) -> Result<Meter<f64>> + Sync,
{
    let params = JoeKuoD6::minimal();
    let seq = Sobol::<f64>::new(5, &params);

    seq.take(n)
        .par_bridge()
        .progress_count(n as u64)
        .try_for_each(|sample| {
            let tc = GeoTestCase::try_from_sample(
                100_000.0 * M,
                sample
                    .try_into()
                    .expect("Expected a five-element f64 array"),
            )
            .unwrap();

            let intercept_point = karney_interception(&tc.segment, &tc.point)?;
            let intercept_dist = geodesic_inverse(&intercept_point, &tc.point.geo)?.geo_distance;

            if f(&tc.segment, &tc.point)? > intercept_dist {
                bail!(
                    "Incorrect intercept distance floor for {:?} and {:?}",
                    &tc.segment,
                    &tc.point
                );
            }
            Ok(())
        })
}

fn cmd_generate() -> Result<()> {
    let params = JoeKuoD6::minimal();
    let seq = Sobol::<f64>::new(5, &params);

    let tc = seq
        .take(2)
        .skip(1)
        .map(|sample| {
            GeoTestCase::try_from_sample(
                17_000_000.0 * M,
                sample
                    .try_into()
                    .expect("Expected a five-element f64 array"),
            )
            .unwrap()
        })
        .collect::<Vec<_>>()
        .remove(0);

    println!("sample: {:?}", tc);
    Ok(())
}

fn cmd_floor(n: usize) -> Result<()> {
    verify_floor(
        |segment, point| {
            let floor = intercept_distance_floor(segment, point)?;
            Ok(floor)
        },
        n,
    )
}

fn cmd_raw_samples(n: usize, out: Option<PathBuf>) -> Result<()> {
    let writer: Box<dyn io::Write> = match out {
        Some(path) => Box::new(File::create(path)?),
        None => Box::new(io::stdout()),
    };
    let mut writer = csv::Writer::from_writer(writer);

    let params = JoeKuoD6::minimal();
    let seq = Sobol::<f64>::new(5, &params);

    seq.take(n).try_for_each(|sample| {
        let s = RawSample::from_array(sample.try_into().unwrap());
        writer.serialize(&s)?;
        Ok(())
    })
}

/// Unscaled [0, 1) values for the test case parameters straight from the Sobol sequence.
#[derive(serde::Serialize)]
struct RawSample {
    raw_s_lat: f64,
    raw_s_azi: f64,
    raw_s_len: f64,
    raw_p_off_azi: f64,
    raw_p_off_len: f64,
}

impl RawSample {
    fn from_array(a: [f64; 5]) -> Self {
        Self {
            raw_s_lat: a[0],
            raw_s_azi: a[1],
            raw_s_len: a[2],
            raw_p_off_azi: a[3],
            raw_p_off_len: a[4],
        }
    }
}

fn cmd_adaptive_interception(n: usize, precision: Meter<f64>, out: Option<PathBuf>) -> Result<()> {
    let writer: Box<dyn io::Write> = match out {
        Some(path) => Box::new(File::create(path)?),
        None => Box::new(io::stdout()),
    };
    let mut writer = csv::Writer::from_writer(writer);

    let params = JoeKuoD6::minimal();
    let seq = Sobol::<f64>::new(5, &params);

    seq.take(n)
        .par_bridge()
        .progress_count(n as u64)
        .map(|sample| {
            let tc = GeoTestCase::try_from_sample(
                100_000.0 * M,
                sample
                    .try_into()
                    .expect("Expected a five-element f64 array"),
            )
            .unwrap();

            let (_, iterations) = adaptive::karney_interception(precision, &tc.segment, &tc.point)?;
            Ok(iterations)
        })
        .collect::<Result<Vec<usize>>>()?
        .iter()
        .try_for_each(|iterations| {
            let result = AdaptiveIterationResult {
                iterations: *iterations,
            };
            writer.serialize(&result)?;
            Ok(())
        })
}

#[derive(serde::Serialize)]
struct AdaptiveIterationResult {
    iterations: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.cmd {
        Commands::Generate {} => cmd_generate(),
        Commands::Floor { n } => cmd_floor(n),
        Commands::RawSample { n, out } => cmd_raw_samples(n, out),
        Commands::AdaptiveInterception {
            n,
            precision_m,
            out,
        } => cmd_adaptive_interception(n, precision_m * M, out),
    }
}
