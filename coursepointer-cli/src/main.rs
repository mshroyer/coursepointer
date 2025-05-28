use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use coursepointer::CoursePointerError;
use coursepointer::fit::FitEncodeError;
use coursepointer::gpx::GpxError;

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

        /// Force overwrite of any existing file at <OUTPUT>
        #[clap(long, short, action)]
        force: bool,
    },
}

fn convert_gpx_cmd(input: PathBuf, output: PathBuf, force: bool) -> Result<()> {
    let gpx_file = BufReader::new(
        File::open(&input)
            .context("Opening the GPX <INPUT> file. Check that it exists and can be accessed.")?,
    );

    let mut fit_file = BufWriter::new(
        if force {
            File::create(output)
        } else {
            File::create_new(output)
        }
        .context("Creating the <OUTPUT> file")?,
    );

    let res = coursepointer::convert_gpx(gpx_file, &mut fit_file);
    match &res {
        Ok(_) => {
            fit_file.flush()?;
            Ok(())
        }

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
    }
}

fn main() -> Result<()> {
    // Don't wrap in anyhow::Result so we preserve Clap's pretty formatting of usage info.
    let args = Args::parse();

    match args.cmd {
        Commands::ConvertGpx {
            input,
            output,
            force,
        } => convert_gpx_cmd(input, output, force),
    }
}
