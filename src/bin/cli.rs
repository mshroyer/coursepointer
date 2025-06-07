use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::builder::styling::Styles;
use clap::{ColorChoice, Parser, Subcommand, command};
use clap_cargo::style::{ERROR, HEADER, INVALID, LITERAL, PLACEHOLDER, USAGE, VALID};
use coursepointer::{CourseOptions, CoursePointerError, FitEncodeError, InterceptStrategy};
use dimensioned::f64prefixes::KILO;
use dimensioned::si::{HR, M};

pub const CLAP_STYLING: Styles = Styles::styled()
    .header(HEADER)
    .usage(USAGE)
    .literal(LITERAL)
    .placeholder(PLACEHOLDER)
    .error(ERROR)
    .valid(VALID)
    .invalid(INVALID);

#[derive(Parser)]
#[command(name = "coursepointer", version, about, color = ColorChoice::Auto, styles = CLAP_STYLING)]
struct Args {
    #[command(subcommand)]
    cmd: Commands,

    /// Configure log verbosity overriding the RUST_LOG environment variable
    ///
    /// Set to `error`, `warn`, `info`, `debug`, or `trace`, or use a more
    /// complex filter expression as supported env_logger.
    #[clap(long)]
    log: Option<String>,
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
    #[clap(long, short, default_value = "5.0")]
    speed: f64,

    /// Strategy for handling duplicate intercepts (within threshold) of the
    /// course from a waypoint.
    #[clap(long, short = 'r', default_value_t = InterceptStrategy::Nearest)]
    strategy: InterceptStrategy,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a GPX file to a FIT file with course points
    ///
    /// Given a GPX file containing a single track, converts the track to a
    /// Garmin FIT course file.
    ConvertGpx(ConvertGpxArgs),
}

fn convert_gpx_cmd(args: ConvertGpxArgs) -> Result<()> {
    log::debug!("convert-gpx: {:?} -> {:?}", args.input, args.output);

    if args.threshold < 0.0 {
        bail!("Threshold cannot be negative");
    }

    if args.speed < 0.001 {
        bail!("Speeds too low can cause some Garmin devices to crash");
    }

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

    let course_options = CourseOptions {
        threshold: args.threshold * M,
        strategy: args.strategy,
    };
    let fit_speed = args.speed * KILO * M / HR;

    let res = coursepointer::convert_gpx(gpx_file, fit_file, course_options, fit_speed);
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

fn init_logging(rust_log: &Option<String>) {
    let mut builder = env_logger::Builder::new();
    if let Some(filters) = rust_log {
        builder.parse_filters(&filters);
    }
    builder.init();
}

fn main() -> Result<()> {
    // Don't wrap in anyhow::Result so we preserve Clap's pretty formatting of usage
    // info.
    let args = Args::parse();

    init_logging(&args.log);

    match args.cmd {
        Commands::ConvertGpx(sub_args) => convert_gpx_cmd(sub_args),
    }
}
