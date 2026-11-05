use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use coursepointer::internal::{
    FromGeoPoints, GeoAndXyzPoint, GeoSegment, geodesic_direct, geodesic_inverse,
    intercept_distance_floor, karney_interception,
};
use coursepointer::{Degree, GeoPoint, DEG};
use dimensioned::si::{Meter, M};
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
            geodesic_direct(
                &sm,
                reverse_azimuth(s_azi),
                s_len / 2.0,
            )?
                .point2,
        )?;
        let s1 = GeoAndXyzPoint::try_from(
            geodesic_direct(&sm, s_azi, s_len / 2.0)?.point2,
        )?;
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

fn cmd_generate() -> Result<()> {
    let params = JoeKuoD6::standard();
    let seq = Sobol::<f64>::new(5, &params);

    let tc = seq
        .take(2)
        .skip(1)
        .map(|sample|
            GeoTestCase::try_from_sample(
                17_000_000.0 * M,
                sample
                    .try_into()
                    .expect("Expected a five-element f64 array"),
            ).unwrap())
        .collect::<Vec<_>>()
        .remove(0);

    println!("sample: {:?}", tc);
    Ok(())
}

fn cmd_original_intercept(n: usize) -> Result<()> {
    let params = JoeKuoD6::minimal();
    let seq = Sobol::<f64>::new(5, &params);

    let result: Result<Vec<_>> = seq
        .take(n)
        .par_bridge()
        .progress_count(n as u64)
        .map(|sample| {
            let tc = GeoTestCase::try_from_sample(
                17_000_000.0 * M,
                sample
                    .try_into()
                    .expect("Expected a five-element f64 array"),
            ).unwrap();

            let intercept_point = karney_interception(&tc.segment, &tc.point)?;
            let intercept_dist = geodesic_inverse(&intercept_point, &tc.point.geo)?.geo_distance;

            if intercept_distance_floor(&tc.segment, &tc.point)? > intercept_dist {
                bail!("Incorrect intercept distance floor for {:?} and {:?}", &tc.segment, &tc.point);
            }
            Ok(())
        })
        .collect();

    result.map(|_| ())
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.cmd {
        Commands::Generate {} => cmd_generate(),
        Commands::OriginalIntercept { n } => cmd_original_intercept(n),
    }
}
