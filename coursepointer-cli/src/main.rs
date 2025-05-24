use std::fs::File;

use anyhow::Result;
use chrono::Utc;
use clap::Parser;

use coursepointer::CourseFile;
use coursepointer::measure::KilometersPerHour;

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
        "Test course".to_string(),
        Utc::now(),
        KilometersPerHour(20.0).into(),
    );
    course_file.encode(&mut file)?;

    Ok(())
}
