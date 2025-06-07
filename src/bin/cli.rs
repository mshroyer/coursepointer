use std::cmp::min;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{PathBuf, absolute};

use anyhow::{Context, Result, bail};
use clap::builder::styling::Styles;
use clap::{ColorChoice, Parser, Subcommand, command};
use clap_cargo::style::{ERROR, HEADER, INVALID, LITERAL, PLACEHOLDER, USAGE, VALID};
use coursepointer::{CourseOptions, CoursePointerError, FitEncodeError, InterceptStrategy};
use dimensioned::f64prefixes::KILO;
use dimensioned::si::{HR, M};
use tracing::level_filters::LevelFilter;
use tracing::{Level, debug, enabled, error, info, instrument, warn};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{Layer, Registry, fmt};

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

    /// Configure diagnostic logging level
    ///
    /// Set to DEBUG to see a performance summary following execution, but be
    /// aware this has a non-negligible performance impact on debug builds.
    #[clap(long, default_value_t = Level::ERROR)]
    log: Level,
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

#[instrument(level = "trace", skip_all)]
fn convert_gpx_cmd(args: ConvertGpxArgs) -> Result<()> {
    debug!("convert-gpx: {:?} -> {:?}", args.input, args.output);

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
    info!("Opened GPX input file: {:?}", absolute(args.input)?);

    if (args.force && enabled!(Level::WARN)) || (!args.force && enabled!(Level::ERROR)) {
        if args.output.exists() {
            if args.force {
                warn!(
                    "Output file exists and will be overwritten: {:?}",
                    args.output
                );
            } else {
                error!(
                    "Output file already exists and may not be overwritten: {:?}",
                    args.output
                );
            }
        }
    }
    let fit_file = BufWriter::new(
        if args.force {
            File::create(&args.output)
        } else {
            File::create_new(&args.output)
        }
        .context("Creating the <OUTPUT> file")?,
    );
    info!("Created FIT output file: {:?}", absolute(&args.output)?);

    let course_options = CourseOptions {
        threshold: args.threshold * M,
        strategy: args.strategy,
    };
    let fit_speed = args.speed * KILO * M / HR;

    let res = coursepointer::convert_gpx(gpx_file, fit_file, course_options, fit_speed);
    let info = match &res {
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

    match info.course_name {
        Some(name) => println!(
            "Converted course {:?} of length {:.02}",
            name, info.total_distance
        ),
        None => println!(
            "Converted unnamed course of length {:.02}",
            info.total_distance
        ),
    }
    println!(
        "Processed {} waypoints, {} of which were identified as course points:",
        info.num_waypoints,
        info.course_points.len()
    );
    let max_course_points = 16usize;
    for i in 0..min(max_course_points, info.course_points.len()) {
        let point = &info.course_points[i];
        println!(
            "- {} at {:.02}{}",
            point.name,
            point.distance,
            if i == 0 { " along the course" } else { "" }
        );
    }
    if info.course_points.len() > max_course_points {
        println!("(and others)");
    }
    println!(
        "Output is in {}",
        absolute(&args.output)
            .unwrap_or(args.output)
            .to_string_lossy()
    );
    Ok(())
}

fn main() -> Result<()> {
    // Intentionally avoid wrapping argument parsing errors in anyhow::Result so
    // we preserve Clap's pretty formatting of usage info.
    let args = Args::parse();

    // Enable the TRACE-level span tree layer for fmt logging level DEBUG.
    let fmt_layer = fmt::Layer::new()
        .with_target(false)
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .with_filter(LevelFilter::from_level(args.log));
    if args.log >= Level::DEBUG {
        let span_tree_layer = tracing_span_tree::SpanTree::default().aggregate(true);
        tracing::subscriber::set_global_default(
            Registry::default().with(fmt_layer).with(span_tree_layer),
        )?;
    } else {
        tracing::subscriber::set_global_default(Registry::default().with(fmt_layer))?;
    }

    match args.cmd {
        Commands::ConvertGpx(sub_args) => convert_gpx_cmd(sub_args),
    }
}
