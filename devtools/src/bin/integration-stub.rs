use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use coursepointer::course::{CourseSetBuilder, CourseSetOptions};
use coursepointer::{CourseFile, DEG, FitCourseOptions, GeoPoint, Sport};
use dimensioned::f64prefixes::KILO;
use dimensioned::si::M;
use dimensioned::si::f64consts::HR;
use serde::Deserialize;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Write a specified course into a FIT file
    WriteFit {
        /// Path to the JSON course spec file
        #[clap(long)]
        spec: PathBuf,

        /// FIT file output path
        #[clap(long)]
        out: PathBuf,
    },

    /// Show the library's Garmin global profile version
    ShowProfileVersion {},
}

#[derive(Deserialize)]
struct JsonPoint {
    /// Latitude in decimal degrees.
    lat: f64,

    /// Longitude in decimal degrees.
    lon: f64,
}

#[derive(Deserialize)]
struct CourseSpec {
    /// Course name.
    name: String,

    /// Start timestamp in RFC3339 format.
    start_time: String,

    /// Ordered points along the course, i.e. trackpoints.
    records: Vec<JsonPoint>,
}

fn parse_rfc9557_utc(s: &str) -> Result<DateTime<Utc>> {
    let ts = DateTime::parse_from_rfc3339(s)?;
    Ok(ts.with_timezone(&Utc))
}

fn write_fit(spec: PathBuf, out: PathBuf) -> Result<()> {
    let spec_file = File::open(&spec)?;
    let spec: CourseSpec = serde_json::from_reader(spec_file)?;

    let mut fit_file = BufWriter::new(File::create(&out)?);
    let mut builder = CourseSetBuilder::new(CourseSetOptions::default());
    builder.add_route();
    builder
        .get_route_mut(builder.num_routes().saturating_sub(1))
        .ok_or(anyhow!("Missing route builder"))?
        .with_name(spec.name);
    for point in &spec.records {
        builder
            .get_route_mut(builder.num_routes().saturating_sub(1))
            .ok_or(anyhow!("Missing route builder"))?
            .with_route_point(GeoPoint::new(point.lat * DEG, point.lon * DEG, None)?);
    }
    let course_set = builder.build()?;
    let course = course_set.courses.first().unwrap();
    let course_file = CourseFile::new(
        &course,
        FitCourseOptions::default()
            .with_start_time(parse_rfc9557_utc(spec.start_time.as_str())?)
            .with_speed(18.0 * (KILO * M) / HR)
            .with_sport(Sport::Cycling),
    );
    course_file.encode(&mut fit_file)?;
    Ok(())
}

fn show_profile_version() -> Result<()> {
    println!("{}", coursepointer::internal::PROFILE_VERSION);
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.cmd {
        Commands::WriteFit { spec, out } => write_fit(spec, out),
        Commands::ShowProfileVersion {} => show_profile_version(),
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chrono::prelude::*;

    use super::parse_rfc9557_utc;

    #[test]
    fn test_parse_rfc9557_utc() -> Result<()> {
        let ts = parse_rfc9557_utc("2025-05-17T01:02:03Z")?;
        assert_eq!(ts, Utc.with_ymd_and_hms(2025, 5, 17, 1, 2, 3).unwrap());
        Ok(())
    }
}
