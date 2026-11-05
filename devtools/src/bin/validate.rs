use anyhow::bail;
use clap::{Parser, Subcommand};
use coursepointer::internal::{
    FromGeoPoints, GeoAndXyzPoint, GeoSegment, geodesic_direct, geodesic_inverse,
    intercept_distance_floor, karney_interception,
};
use coursepointer::{DEG, GeoPoint};
use dimensioned::si::M;
use indicatif::ParallelProgressIterator;
use rayon::prelude::*;
use sobol::Sobol;
use sobol::params::JoeKuoD6;

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
    OriginalIntercept {
        /// The number of test cases to check
        n: usize,
    },
}

/// A test case for validating that our filter function in fact provides a lower
/// bound on the geodesic distance between a point and a segment.
///
/// Gives us a geodesic segment and a point whose distance we check using both
/// Karney's method and our linear approximation.
#[derive(Debug)]
struct TestCase {
    /// The latitude of the centerpoint of the segment under test, in degrees in
    /// [0, 90].
    s_lat: f64,

    /// The azimuth of the segment under test, in degrees in [0, 90).
    s_azi: f64,

    /// The geodesic length of the segment under test, in meters.
    s_len: f64,

    /// The azimuth at which the point under test is to be offset from the
    /// segment's center, in [0, 360).
    p_off_azi: f64,

    /// The distance in meters at which the point under test is offset from the
    /// segment's center.
    p_off_len: f64,
}

impl TestCase {
    fn sample(sample: [f64; 5]) -> Self {
        assert_eq!(sample.len(), 5, "Incorrect Sobol sample dimensionality");
        Self {
            s_lat: 90.0 * sample[0],
            s_azi: 90.0 * sample[1],
            s_len: 17_000_000.0 * sample[2],
            p_off_azi: 360.0 * sample[3],
            p_off_len: 17_000_000.0 * sample[4],
        }
    }
}

fn cmd_generate() -> anyhow::Result<()> {
    let params = JoeKuoD6::standard();
    let seq = Sobol::<f64>::new(5, &params);

    let tc: TestCase = seq
        .take(2)
        .skip(1)
        .map(|p| p.try_into().expect("Expected a five-element f64 array"))
        .map(TestCase::sample)
        .collect::<Vec<_>>()
        .remove(0);

    let sm = GeoPoint::new(tc.s_lat * DEG, 0.0 * DEG, None)?;
    let p = geodesic_direct(&sm, tc.p_off_azi * DEG, tc.p_off_len * M)?.point2;
    let s0 = GeoAndXyzPoint::try_from(
        geodesic_direct(
            &sm,
            ((tc.s_azi + 180.0) % 360.0) * DEG,
            (tc.s_len / 2.0) * M,
        )?
        .point2,
    )?;
    let s1 = GeoAndXyzPoint::try_from(
        geodesic_direct(&sm, tc.s_azi * DEG, (tc.s_len / 2.0) * M)?.point2,
    )?;
    let s = GeoSegment::from_geo_points(&s0, &s1)?;

    println!("sample: {:?}", tc);
    println!("point: {:?}", p);
    println!("segment: {:?}", s);

    Ok(())
}

fn cmd_original_intercept(n: usize) -> anyhow::Result<()> {
    let params = JoeKuoD6::minimal();
    let seq = Sobol::<f64>::new(5, &params);

    let result: anyhow::Result<Vec<_>> = seq
        .take(n)
        .par_bridge()
        .progress_count(n as u64)
        .map(|sample| {
            let tc = TestCase::sample(
                sample
                    .try_into()
                    .expect("Expected a five-element f64 array"),
            );

            let sm = GeoPoint::new(tc.s_lat * DEG, 0.0 * DEG, None)?;
            let geo_p = geodesic_direct(&sm, tc.p_off_azi * DEG, tc.p_off_len * M)?.point2;
            let p = GeoAndXyzPoint::try_from(geo_p)?;
            let s0 = GeoAndXyzPoint::try_from(
                geodesic_direct(
                    &sm,
                    ((tc.s_azi + 180.0) % 360.0) * DEG,
                    (tc.s_len / 2.0) * M,
                )?
                .point2,
            )?;
            let s1 = GeoAndXyzPoint::try_from(
                geodesic_direct(&sm, tc.s_azi * DEG, (tc.s_len / 2.0) * M)?.point2,
            )?;
            let s = GeoSegment::from_geo_points(&s0, &s1)?;

            let intercept_point = karney_interception(&s, &p)?;
            let intercept_dist = geodesic_inverse(&intercept_point, &p.geo)?.geo_distance;

            if intercept_distance_floor(&s, &p)? > intercept_dist {
                bail!("Incorrect intercept distance floor for {:?} and {:?}", s, p);
            }
            Ok(())
        })
        .collect();

    result.map(|_| ())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.cmd {
        Commands::Generate {} => cmd_generate(),
        Commands::OriginalIntercept { n } => cmd_original_intercept(n),
    }
}
