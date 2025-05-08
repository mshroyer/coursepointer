use std::fs::File;
use std::io::prelude::*;

use anyhow::Result;
use clap::Parser;

use coursepointer::{Encode, FitFile};

#[derive(Parser)]
struct Args {
    /// The output file to write to
    #[clap(short, long)]
    output: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut file = File::create(&args.output)?;
    let header = FitFile::new(21170u16, 17032usize)?;
    header.encode(&mut file)?;

    Ok(())
}
