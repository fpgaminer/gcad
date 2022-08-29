mod engine;
mod gcode;
mod numbers;
mod value;

use anyhow::Result;
use clap::Parser;
use engine::ScriptEngine;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
	/// Name of the person to greet
	#[clap(short, long, value_parser, required = true)]
	output: PathBuf,

	/// Verbose
	#[clap(short, long)]
	verbose: bool,

	/// Input file
	#[clap(required = true)]
	input: PathBuf,
}

fn main() -> Result<()> {
	let args = Args::parse();

	let mut machine = ScriptEngine::new();
	machine.write_header();
	machine.run_file("materials.gcad", args.verbose)?;
	machine.run_file(args.input, args.verbose)?;
	machine.finish(args.output)?;

	Ok(())
}
