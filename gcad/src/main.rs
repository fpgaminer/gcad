use anyhow::{Context, Result};
use clap::Parser;
use libgcad::{ScriptEngine, BUILTIN_MATERIALS};
use std::{fs::File, io::BufWriter, path::PathBuf};

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
	machine.run(BUILTIN_MATERIALS, args.verbose)?;
	machine.run_file(args.input, args.verbose)?;

	let mut output_file = File::create(&args.output).with_context(|| format!("Failed to create file: {}", args.output.display()))?;
	let writer = BufWriter::new(&mut output_file);
	machine.finish(writer)?;

	Ok(())
}
