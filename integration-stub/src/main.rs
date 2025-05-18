use std::fs::File;
use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use uom::si::f64::Velocity;
use uom::si::velocity::kilometer_per_hour;

use coursepointer::CourseFile;
use geographic::SurfacePoint;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
    WriteFit {
        /// Path to the JSON course spec file
        #[clap(long)]
        spec: PathBuf,

        /// FIT file output path
        #[clap(long)]
        out: PathBuf,
    },
}

#[derive(Deserialize)]
struct JsonPoint {
    lat: f64,
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

    let mut fit_file = File::create(&out)?;
    let mut course = CourseFile::new(
        21178u16,
        spec.name,
        parse_rfc9557_utc(&spec.start_time)?,
        Velocity::new::<kilometer_per_hour>(20.0),
    );
    for point in &spec.records {
        course.add_record(SurfacePoint::new(point.lat, point.lon))?;
    }
    course.encode(&mut fit_file)?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.cmd {
        Commands::WriteFit { spec, out } => write_fit(spec, out),
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
