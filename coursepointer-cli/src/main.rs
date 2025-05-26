use anyhow::Result;
use clap::{Parser, Subcommand};
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
            coursepointer::convert_gpx(input.as_ref(), output.as_ref())?
        }
    }

    Ok(())
}
