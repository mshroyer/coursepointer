use std::cmp::min;
use std::fmt::{Display, Write};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf, absolute};

use anyhow::{Context, Result, bail};
use clap::builder::styling::Styles;
use clap::{Args, ColorChoice, Parser, Subcommand, ValueEnum, command};
use clap_cargo::style::{ERROR, HEADER, INVALID, LITERAL, PLACEHOLDER, USAGE, VALID};
use coursepointer::internal::{CoursePointType, Kilometer, Mile};
use coursepointer::{
    ConversionInfo, CourseOptions, CoursePointerError, FitEncodeError, InterceptStrategy,
};
use dimensioned::f64prefixes::KILO;
use dimensioned::si::{HR, M, Meter};
use strum::Display;
use sys_locale::get_locale;
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

/// Convert waypoints into Garmin FIT course points
///
/// Given a route and a set of waypoints, produces a Garmin FIT course file
/// containing the route, with course points corresponding to any of the
/// waypoints that are located approximately along the route.
///
/// https://github.com/mshroyer/coursepointer/
#[derive(Parser)]
#[command(
    name = "coursepointer",
    version,
    about,
    color = ColorChoice::Auto,
    styles = CLAP_STYLING,
)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,

    /// Configure diagnostic logging level
    ///
    /// Set to DEBUG to see a performance summary following execution, but be
    /// aware this has a non-negligible performance impact on debug builds.
    #[clap(long, short = 'L', default_value_t = Level::ERROR)]
    log_level: Level,

    /// The unit of distance used in output on the command line.
    ///
    /// If unspecified, this will default to either km or mi based on your
    /// system locale.
    #[clap(long, short = 'u', default_value_t = DistUnit::Autodetect)]
    distance_unit: DistUnit,
}

#[derive(Copy, Clone, Display, ValueEnum)]
#[strum(serialize_all = "kebab-case")]
#[clap(rename_all = "kebab-case")]
enum DistUnit {
    Autodetect,
    M,
    Km,
    Mi,
}

impl DistUnit {
    fn get(self) -> DistUnit {
        match self {
            Self::Autodetect => Self::auto_detect(),
            _ => self,
        }
    }

    fn auto_detect() -> DistUnit {
        let locale = get_locale().unwrap_or_else(|| String::from("en-US"));
        match locale.as_str() {
            "en-US" | "en-UK" => Self::Mi,
            _ => Self::Km,
        }
    }
}

#[derive(Args, Debug)]
struct ConvertArgs {
    /// GPX input path
    input: PathBuf,

    /// FIT file output path
    ///
    /// If unspecified, defaults to <INPUT>.fit in the same directory as the
    /// input file.
    #[clap(long, short)]
    output: Option<PathBuf>,

    /// Force overwriting the output file, if it already exists.
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

#[derive(Args, Debug)]
struct SampleCoursePointsArgs {
    /// Course name. This will be used as both the filename prefix and the FIT
    /// course name.
    name: String,

    /// Starting latitude
    #[clap(long, default_value_t = 0.0)]
    lat: f64,

    /// Starting longitude
    #[clap(long, default_value_t = 0.0)]
    lon: f64,

    /// Longitude increment between course points
    #[clap(long, default_value_t = 0.1)]
    increment: f64,

    /// Course point type to start with
    #[clap(long, default_value_t = CoursePointType::Generic)]
    start_type: CoursePointType,

    /// Number of point types to put in this file
    #[clap(long, short, default_value_t = 54)]
    num_types: usize,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a GPX file to a FIT file with course points
    ///
    /// Given a GPX file containing a single track, converts the track to a
    /// Garmin FIT course file.
    Convert(ConvertArgs),

    /// Print software license info
    License,
}

#[instrument(level = "trace", skip_all)]
fn convert_gpx_cmd(args: &Cli, sub_args: &ConvertArgs) -> Result<String> {
    debug!("convert args: {:?}", sub_args);

    if sub_args.threshold < 0.0 {
        bail!("Threshold cannot be negative");
    }

    if sub_args.speed < 0.01 {
        bail!("Speeds too low can cause some Garmin devices to crash");
    }

    let gpx_file = BufReader::new(
        File::open(&sub_args.input)
            .context("Opening the GPX <INPUT> file. Check that it exists and can be accessed.")?,
    );
    info!("Opened GPX input file: {:?}", absolute(&sub_args.input)?);

    let output = match &sub_args.output {
        Some(p) => p,
        None => &sub_args.input.with_extension("fit"),
    };

    if ((sub_args.force && enabled!(Level::WARN)) || (!sub_args.force && enabled!(Level::ERROR)))
        && output.exists()
    {
        if sub_args.force {
            warn!(
                "Output file exists and will be overwritten: {:?}",
                sub_args.output
            );
        } else {
            error!(
                "Output file already exists and may not be overwritten: {:?}",
                sub_args.output
            );
        }
    }
    let fit_file = BufWriter::new(
        if sub_args.force {
            File::create(output)
        } else {
            File::create_new(output)
        }
        .context("Creating the <OUTPUT> file")?,
    );
    info!("Created FIT output file: {:?}", absolute(output)?);

    let course_options = CourseOptions {
        threshold: sub_args.threshold * M,
        strategy: sub_args.strategy,
    };
    let fit_speed = sub_args.speed * KILO * M / HR;

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

    match args.distance_unit.get() {
        DistUnit::M => generate_conversion_report::<Meter<f64>>(info, output),
        DistUnit::Km => generate_conversion_report::<Kilometer<f64>>(info, output),
        DistUnit::Mi => generate_conversion_report::<Mile<f64>>(info, output),
        _ => {
            error!(
                "Failed to detect distance unit for report: {}",
                args.distance_unit
            );
            Ok("".to_string())
        }
    }
}

fn generate_conversion_report<T>(info: ConversionInfo, output: &Path) -> Result<String>
where
    T: From<Meter<f64>> + Display,
{
    // Build a report to print after the tracing span surrounding this function
    // has exited. If debug logging is enabled, this ensures the report to
    // STDOUT will be printed after all the tracing stuff.
    let mut r = String::new();
    match info.course_name {
        Some(name) => writeln!(
            &mut r,
            "Converted course {:?} of length {:.02}\n",
            name,
            T::from(info.total_distance)
        )?,
        None => writeln!(
            &mut r,
            "Converted an unnamed course of length {:.02}\n",
            T::from(info.total_distance)
        )?,
    };
    writeln!(
        &mut r,
        "Processed {} waypoints, {} of which {}{}",
        info.num_waypoints,
        info.course_points.len(),
        if info.course_points.len() == 1 {
            "was identified as a course point"
        } else {
            "were identified as course points"
        },
        if !info.course_points.is_empty() {
            ":"
        } else {
            ""
        },
    )?;
    let max_listing = 24usize;
    for i in 0..min(max_listing, info.course_points.len()) {
        let point = &info.course_points[i];
        writeln!(
            &mut r,
            "- {} at {:.02}{}",
            point.name,
            T::from(point.distance),
            if i == 0 { " along the course" } else { "" }
        )?;
    }
    if info.course_points.len() > max_listing {
        writeln!(&mut r, "(and others)")?;
    }
    writeln!(
        &mut r,
        "\nOutput is in {}",
        absolute(output)
            .unwrap_or(output.to_path_buf())
            .to_string_lossy()
    )?;
    Ok(r)
}

fn license_cmd() -> Result<String> {
    let mut r = include_str!("../../LICENSE.txt").to_string();
    writeln!(
        &mut r,
        r#"
===

This executable contains code from third-party open source projects, whose
licenses are shown here:

https://github.com/mshroyer/coursepointer/blob/main/docs/third_party_licenses.md
"#
    )?;
    Ok(r)
}

fn main() -> Result<()> {
    // Intentionally avoid wrapping argument parsing errors in anyhow::Result so
    // we preserve Clap's pretty formatting of usage info.
    let args = Cli::parse();

    // Enable the TRACE-level span tree layer for fmt logging level DEBUG.
    let fmt_layer = fmt::Layer::new()
        .with_target(false)
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .with_filter(LevelFilter::from_level(args.log_level));
    if args.log_level >= Level::DEBUG {
        let span_tree_layer = tracing_span_tree::SpanTree::default().aggregate(true);
        tracing::subscriber::set_global_default(
            Registry::default().with(fmt_layer).with(span_tree_layer),
        )?;
    } else {
        tracing::subscriber::set_global_default(Registry::default().with(fmt_layer))?;
    }

    let report = match &args.cmd {
        Commands::Convert(sub_args) => convert_gpx_cmd(&args, sub_args),
        Commands::License => license_cmd(),
    }?;

    print!("{}", report);
    Ok(())
}
