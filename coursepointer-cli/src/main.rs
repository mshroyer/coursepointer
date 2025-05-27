use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use coursepointer::CoursePointerError;
use coursepointer::gpx::GpxError;
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a GPX file to a FIT course
    ///
    /// Given a GPX file containing a single track, converts the track to a
    /// Garmin FIT course file.
    ConvertGpx {
        /// GPX input path
        input: PathBuf,

        /// Path where to write FIT output
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.cmd {
        Commands::ConvertGpx { input, output } => {
            let res = coursepointer::convert_gpx(input.as_ref(), output.as_ref());
            match &res {
                Err(CoursePointerError::Gpx(GpxError::Io(_))) => res.context(
                    "Reading the GPX <INPUT> file. Check that it exists and can be accessed.",
                ),

                Err(CoursePointerError::Gpx(_)) => {
                    res.context("The <INPUT> is not a valid GPX file. Check that it is correct.")
                }

                Err(CoursePointerError::CourseCount(0usize)) => res.context(concat!(
                    "No course was found in the <INPUT> file. Ensure it is a valid GPX ",
                    "file containing at least one track or route."
                )),

                _ => res.map_err(anyhow::Error::from),
            }?
        }
    }

    Ok(())
}
