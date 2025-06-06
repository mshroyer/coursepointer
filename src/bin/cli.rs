use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use coursepointer::{CourseOptions, CoursePointerError, FitEncodeError};
use dimensioned::f64prefixes::KILO;
use dimensioned::si::{HR, M};

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Parser)]
struct ConvertGpxArgs {
    /// GPX input path
    input: PathBuf,

    /// FIT file output path
    output: PathBuf,

    /// Force overwrite the output file, if it already exists
    #[clap(long, short, action)]
    force: bool,

    /// Max distance from course at which a waypoint is considered a course
    /// point, in meters
    #[clap(long, short, default_value = "35.0")]
    threshold: f64,

    /// Speed in kilometers per hour, as used for the "virtual partner" on
    /// devices that support it
    #[clap(long, short, default_value = "20.0")]
    speed: f64,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a GPX file to a FIT course
    ///
    /// Given a GPX file containing a single track, converts the track to a
    /// Garmin FIT course file.
    ConvertGpx(ConvertGpxArgs),
}

fn convert_gpx_cmd(args: ConvertGpxArgs) -> Result<()> {
    log::debug!("convert-gpx: {:?} -> {:?}", args.input, args.output);
    let gpx_file = BufReader::new(
        File::open(&args.input)
            .context("Opening the GPX <INPUT> file. Check that it exists and can be accessed.")?,
    );

    let fit_file = BufWriter::new(
        if args.force {
            File::create(args.output)
        } else {
            File::create_new(args.output)
        }
        .context("Creating the <OUTPUT> file")?,
    );

    let options = CourseOptions {
        threshold: args.threshold * M,
        speed: args.speed * KILO * M / HR,
    };

    let res = coursepointer::convert_gpx(gpx_file, fit_file, options);
    match &res {
        Err(CoursePointerError::Gpx(_)) => {
            res.context("The <INPUT> is not a valid GPX file. Check that it is correct.")
        }

        Err(CoursePointerError::CourseCount(0)) => res.context(concat!(
            "No course was found in the <INPUT> file. Ensure it is a valid GPX ",
            "file containing at least one track or route."
        )),

        Err(CoursePointerError::FitEncode(FitEncodeError::Io(_))) => res.context(concat!(
            "Writing the FIT output to the filesystem. Ensure the output path exists and ",
            "that you have access permissions to write there."
        )),

        _ => res.map_err(anyhow::Error::from),
    }?;

    println!("Done.");

    Ok(())
}

fn main() -> Result<()> {
    // Don't wrap in anyhow::Result so we preserve Clap's pretty formatting of usage
    // info.
    let args = Args::parse();

    env_logger::init();

    match args.cmd {
        Commands::ConvertGpx(sub_args) => convert_gpx_cmd(sub_args),
    }
}
