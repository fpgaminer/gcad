mod numbers;
mod value;
mod engine;
mod gcode;

use std::{fs, path::PathBuf};
use clap::Parser;
use engine::ScriptEngine;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
   /// Name of the person to greet
   #[clap(short, long, value_parser, required = true)]
   output: PathBuf,

   /// Input file
   #[clap(required = true)]
   input: PathBuf,
}

fn main() {
	let args = Args::parse();
	
	let unparsed_file = fs::read_to_string(args.input).expect("Failed to read input file");
	let mut machine = ScriptEngine::new(args.output);

	machine.run(&fs::read_to_string("materials.gcad").expect("Could not read 'materials.gcad'"));
	machine.write_header();
	machine.run(&unparsed_file);
	machine.finish();
}