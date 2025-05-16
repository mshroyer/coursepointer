use std::fs::File;

use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use uom::si::f64::Velocity;
use uom::si::velocity::kilometer_per_hour;

use coursepointer::CourseFile;

#[derive(Parser)]
struct Args {
    /// The output file to write to
    #[clap(short, long)]
    output: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut file = File::create(&args.output)?;
    let course_file = CourseFile::new(
        21178u16,
        "Test course".to_string(),
        Utc::now(),
        Velocity::new::<kilometer_per_hour>(20.0),
    );
    course_file.encode(&mut file)?;

    Ok(())
}
