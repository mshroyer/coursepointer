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

/// Prototypes the new intercept floor function on top of some code copied out
/// of algorithm.rs.
mod new_floor {
    use std::ops::{Mul, Sub};

    use anyhow::Result;
    use coursepointer::internal::{GeoAndXyzPoint, GeoSegment};
    use dimensioned::si::{M, Meter};

    const A: f64 = 6378137.0;
    const F: f64 = 1.0 / 298.257223563;
    const B: f64 = A * (1.0 - F);

    pub fn intercept_distance_floor(
        segment: &GeoSegment<GeoAndXyzPoint>,
        point: &GeoAndXyzPoint,
    ) -> Result<Meter<f64>> {
        let dist = euclidean_intercept_distance(segment, point)?;

        let l = segment.geo_length.value_unsafe;
        let de_max = (B * B / A) * (1.0 - ((A / (2.0 * B * B)) * l).cos()) * M;
        // Ok(dist - (de_max + 0.000_001 * M))
        Ok(dist - de_max)
    }

    fn euclidean_intercept_distance(
        segment: &GeoSegment<GeoAndXyzPoint>,
        point: &GeoAndXyzPoint,
    ) -> Result<Meter<f64>> {
        let b = subtract_xyzpoints(&segment.end, &segment.start);
        let a = subtract_xyzpoints(point, &segment.start);

        // The same calculation used to find segment intercepts in each
        // iteration of [`karney_intercept`], just in 3D space!
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

    // Copy-and-paste a bunch of stuff from algorithm.rs so we don't have to
    // make all its internals public just to test the new algorithm here...

    fn subtract_xyzpoints(a: &GeoAndXyzPoint, b: &GeoAndXyzPoint) -> Vec3 {
        Vec3 {
            x: a.xyz.x.value_unsafe - b.xyz.x.value_unsafe,
            y: a.xyz.y.value_unsafe - b.xyz.y.value_unsafe,
            z: a.xyz.z.value_unsafe - b.xyz.z.value_unsafe,
        }
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

    fn dot3(a: Vec3, b: Vec3) -> f64 {
        a.x * b.x + a.y * b.y + a.z * b.z
    }

    fn norm3(vec: Vec3) -> f64 {
        dot3(vec, vec).sqrt()
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

    /// Validate the original intercept floor calculation
    OriginalFloor {
        /// The number of test cases to check
        n: usize,
    },

    /// Validate the new intercept floor calculation
    NewFloor {
        /// The number of test cases to check
        n: usize,
    },

    /// Output raw samples of the Sobol sequence for examination
    RawSample {
        /// The number of samples to take
        n: usize,

        /// Where to write output; stdout is used if unset
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
}

fn verify_intercept_floor<F>(f: F, n: usize) -> Result<()>
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

fn cmd_original_floor(n: usize) -> Result<()> {
    verify_intercept_floor(
        |segment, point| {
            let floor = intercept_distance_floor(segment, point)?;
            Ok(floor)
        },
        n,
    )
}

fn cmd_new_floor(n: usize) -> Result<()> {
    verify_intercept_floor(new_floor::intercept_distance_floor, n)
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

/// Unscaled [0, 1) values for the test case parameters straight from the Sobol
/// sequence.
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

fn main() -> Result<()> {
    let args = Args::parse();

    match args.cmd {
        Commands::Generate {} => cmd_generate(),
        Commands::OriginalFloor { n } => cmd_original_floor(n),
        Commands::NewFloor { n } => cmd_new_floor(n),
        Commands::RawSample { n, out } => cmd_raw_samples(n, out),
    }
}
