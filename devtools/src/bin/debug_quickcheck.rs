use anyhow::bail;
use clap::Parser;
use coursepointer::{DEG, GeoPoint, debug_intercept};
use regex::Regex;

/// Debugs output from failed quickcheck runs
///
/// Takes on stdin the copy-pasted output from failed runs of the qc_ tests and
/// computes the distances between the points in question and the actual
/// magnitude of any discrepancy.
#[derive(Parser)]
struct Cli {}

fn main() -> anyhow::Result<()> {
    let mut input = String::new();
    eprint!("Enter output: ");
    std::io::stdin()
        .read_line(&mut input)
        .expect("Could not read input");

    let re = Regex::new(r"-?\d+\.\d+")?;
    let ns = re
        .find_iter(&input)
        .filter_map(|digits| digits.as_str().parse().ok())
        .collect::<Vec<f64>>();

    if ns.len() != 6 {
        bail!("Expected six numbers in input");
    }

    let s1 = GeoPoint::new(ns[0] * DEG, ns[1] * DEG, None)?;
    let s2 = GeoPoint::new(ns[2] * DEG, ns[3] * DEG, None)?;
    let p = GeoPoint::new(ns[4] * DEG, ns[5] * DEG, None)?;

    debug_intercept(&s1, &s2, &p)?;
    Ok(())
}
