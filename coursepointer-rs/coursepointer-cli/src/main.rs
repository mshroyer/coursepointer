use std::fs::File;

use anyhow::Result;
use clap::Parser;

use coursepointer::CourseFile;

#[derive(Parser)]
struct Args {
    /// The output file to write to
    #[clap(short, long)]
    output: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut file = File::create(&args.output)?;
    let course_file = CourseFile::new(21178u16);
    course_file.encode(&mut file)?;

    Ok(())
}
