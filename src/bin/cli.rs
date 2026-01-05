use std::fmt::{Display, Write};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf, absolute};
use std::sync::LazyLock;

use anyhow::{Context, Result, anyhow, bail};
use clap::builder::styling::Styles;
use clap::{Args, ColorChoice, Parser, Subcommand, ValueEnum, crate_version};
use clap_cargo::style::{ERROR, HEADER, INVALID, LITERAL, PLACEHOLDER, USAGE, VALID};
use coursepointer::course::{CourseSetOptions, InterceptStrategy};
use coursepointer::internal::{Kilometer, Mile, compiler_version_str, geographiclib_version_str};
use coursepointer::{
    ConversionInfo, CoursePointType, CoursePointerError, FitCourseOptions, FitEncodeError, Sport,
};
use dimensioned::f64prefixes::KILO;
use dimensioned::si::{HR, M, Meter};
use regex::{Match, Regex};
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

static LONG_VERSION: LazyLock<String> = LazyLock::new(|| {
    format!(
        "{} ({}, rustc {}, {})",
        crate_version!(),
        geographiclib_version_str(),
        env!("RUSTC_VERSION"),
        compiler_version_str(),
    )
});

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
    long_version = LONG_VERSION.as_str(),
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

    /// Log to a file
    #[clap(long)]
    log_file: Option<PathBuf>,

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
            "en-US" | "en-GB" => Self::Mi,
            _ => Self::Km,
        }
    }
}

/// Encode a version number for FIT
///
/// Encodes the crate's version number at compilation into a 16-bit unsigned
/// integer that can be stashed in the FIT file_creator message.  Since this can
/// store values up to 65,534, we can segment it as base-10 digits with:
///
/// - Most significant digit for crate major version number
/// - Next two for minor version number
/// - Final two for patch
///
/// This is similar to how Garmin stores their own SDK version in FIT files, and
/// it would let us represent crate major versions up through 5.
fn encode_version_number() -> Result<u16> {
    let re = Regex::new(r"^(\d+)\.(\d+)\.(\d+)")?;

    fn part(s: Option<Match>) -> Result<u16> {
        let val = s
            .ok_or_else(|| anyhow!("Did not get a version part match"))?
            .as_str()
            .parse::<u16>()
            .map_err(|e| anyhow!("Couldn't parse version part as u16: {e}"))?;
        if val > 99 {
            bail!("Can't encode version part with more than two digits");
        }
        Ok(val)
    }

    match re.captures(crate_version!()) {
        Some(caps) => {
            let major = part(caps.get(1))?;
            let minor = part(caps.get(2))?;
            let patch = part(caps.get(3))?;
            Ok(10000 * major + 100 * minor + patch)
        }
        None => Err(anyhow!("Crate version string didn't match regex")),
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

    /// Sport to be designated for the course.
    #[clap(long, short = 'p', default_value = "generic")]
    sport: Sport,

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
    /// Given a GPX file containing a single route or track and (optionally)
    /// waypoints, converts the route or track to a Garmin FIT course file,
    /// including course points for any of the waypoints that are located along
    /// the route.
    Convert(ConvertArgs),

    /// Print software license info
    License,
}

#[instrument(level = "trace", skip_all)]
fn convert_cmd(args: &Cli, sub_args: &ConvertArgs) -> Result<String> {
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

    let course_options = CourseSetOptions::default()
        .with_threshold(sub_args.threshold * M)
        .with_strategy(sub_args.strategy);
    let fit_options = FitCourseOptions::default()
        .with_speed(sub_args.speed * KILO * M / HR)
        .with_sport(sub_args.sport)
        .with_product_name("CoursePointer".to_owned())
        .with_software_version(encode_version_number().unwrap_or_else(|e| {
            warn!("Unable to encode version number to FIT: {e}");
            0u16
        }));

    let res = coursepointer::convert_gpx_to_fit(gpx_file, fit_file, course_options, fit_options);
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
    let mut r = coursepointer::internal::report::conversion_report::<T>(info)?;
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

https://github.com/mshroyer/coursepointer/blob/v{}/docs/third_party_licenses.md
"#,
        crate_version!(),
    )?;
    Ok(r)
}

fn main() -> Result<()> {
    // Intentionally avoid wrapping argument parsing errors in anyhow::Result so
    // we preserve Clap's pretty formatting of usage info.
    let args = Cli::parse();

    let log_w: Box<dyn std::io::Write + Send> = match &args.log_file {
        Some(path) => Box::new(File::create(path).context("Creating the log file")?),
        None => Box::new(std::io::stderr()),
    };
    let (appender, _guard) = tracing_appender::non_blocking(log_w);

    // Enable the TRACE-level span tree layer for fmt logging level DEBUG.
    let fmt_layer = fmt::Layer::new()
        .with_writer(appender)
        .with_ansi(args.log_file.is_none())
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

    debug!("coursepointer {}", clap::crate_version!());

    let report = match &args.cmd {
        Commands::Convert(sub_args) => convert_cmd(&args, sub_args),
        Commands::License => license_cmd(),
    }?;

    print!("{report}");
    Ok(())
}
