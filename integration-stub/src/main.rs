use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::Utc;
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
    records: Vec<JsonPoint>,
}

fn write_fit(spec: PathBuf, out: PathBuf) -> Result<()> {
    let spec_file = File::open(&spec)?;
    let spec: CourseSpec = serde_json::from_reader(spec_file)?;

    let mut fit_file = File::create(&out)?;
    let mut course = CourseFile::new(
        21178u16,
        "Test course".to_string(),
        Utc::now(),
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
