mod builtins;

use std::{collections::HashMap, path::Path};

use pest::{
	iterators::Pair,
	prec_climber::{Assoc, PrecClimber},
	Parser,
};
use pest_derive::Parser;

use crate::{gcode::GcodeState, numbers::Number, value::ScriptValue};
use anyhow::{bail, Context, Result};


#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct ScriptParser;

static CLIMBER: PrecClimber<Rule> = PrecClimber::new_const(&[
	(Rule::add, 1, Assoc::Left),
	(Rule::subtract, 1, Assoc::Left),
	(Rule::multiply, 2, Assoc::Left),
	(Rule::divide, 2, Assoc::Left),
]);

pub struct ScriptEngine {
	global_vars: HashMap<String, ScriptValue>,
	materials: HashMap<String, Material>,
	gcode: GcodeState,
}

impl ScriptEngine {
	pub fn new<P: AsRef<Path>>(output_path: P) -> Result<Self> {
		let gcode = GcodeState::new(output_path)?;

		Ok(Self {
			global_vars: HashMap::new(),
			materials: HashMap::new(),
			gcode,
		})
	}

	pub fn run_file<P: AsRef<Path>>(&mut self, path: P, verbose: bool) -> Result<()> {
		let unparsed_file = std::fs::read_to_string(path.as_ref()).with_context(|| format!("Failed to read file: {}", path.as_ref().display()))?;

		self.run(&unparsed_file, verbose)
	}

	pub fn run(&mut self, source: &str, verbose: bool) -> Result<()> {
		let pairs = ScriptParser::parse(Rule::program, source)?;

		if verbose {
			self.format_parse_tree(pairs.clone(), 0);
		}

		for pair in pairs {
			match pair.as_rule() {
				Rule::expr | Rule::forLoop => {
					self.exec(pair)?;
				},
				Rule::EOI => {},
				_ => bail!("Unexpected rule: {:?}", pair.as_rule()),
			}
		}

		Ok(())
	}

	fn format_parse_tree(&self, pairs: pest::iterators::Pairs<Rule>, indent: usize) {
		for pair in pairs {
			let indent_str = "|    ".repeat(indent.saturating_sub(1)) + if indent > 0 { "|----" } else { "" };
			let span = pair.as_span();
			let rule = pair.as_rule();
			let inner = pair.into_inner();

			print!("{}{:?}", indent_str, rule);

			if inner.clone().count() > 0 {
				println!();
				self.format_parse_tree(inner, indent + 1);
			} else {
				println!(": {}", span.as_str());
			}
		}
	}

	pub fn write_header(&mut self) -> Result<()> {
		self.gcode.write_header()
	}

	pub fn finish(&mut self) -> Result<()> {
		self.gcode.finish()
	}

	fn exec(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<ScriptValue> {
		Ok(match pair.as_rule() {
			Rule::expr => self.exec(pair.into_inner().next().unwrap())?,
			Rule::assign => {
				let mut pair = pair.into_inner();
				let ident = pair.next().unwrap();
				let expr = pair.next().unwrap();
				let expr = self.exec(expr)?;

				self.global_vars.insert(ident.as_str().to_string(), expr.clone());

				expr
			},
			Rule::mathExpr => CLIMBER.climb(
				pair.into_inner(),
				|pair: Pair<Rule>| self.exec(pair),
				|lhs: Result<ScriptValue>, op: Pair<Rule>, rhs: Result<ScriptValue>| {
					let lhs = lhs?;
					let rhs = rhs?;

					Ok(match op.as_rule() {
						Rule::add => lhs + rhs,
						Rule::subtract => lhs - rhs,
						Rule::multiply => lhs * rhs,
						Rule::divide => lhs / rhs,
						_ => unreachable!(),
					})
				},
			)?,
			Rule::string => {
				let str = &pair.as_str();
				let str = &str[1..str.len() - 1];
				let str = str.replace("''", "'");
				ScriptValue::String(str.to_string())
			},
			Rule::funcCall => {
				let span = pair.as_span();
				let mut pair = pair.into_inner();
				let ident = pair.next().unwrap();
				let ident_span = ident.as_span();
				let ident = ident.as_str();
				let (args, nargs) = self.parse_func_parameters(pair.next().unwrap())?;
				let ret = self
					.call_builtin(ident, &args, &nargs)
					.map_err(|e| pest::error::Error::new_from_span(pest::error::ErrorVariant::<()>::CustomError { message: e.to_string() }, span))?;

				if let Some(ret) = ret {
					ret
				} else {
					return Err(pest::error::Error::new_from_span(
						pest::error::ErrorVariant::<()>::CustomError {
							message: format!("Function not found"),
						},
						ident_span,
					)
					.into());
				}
			},
			Rule::unitless_number => {
				let mut pair = pair.into_inner();
				let value = pair.next().unwrap();
				let value = match value.as_rule() {
					Rule::integer => Number::from_int(value.as_str().parse::<i64>().unwrap()),
					Rule::decimal => Number::from_float(value.as_str().parse::<f64>().unwrap()),
					_ => unreachable!(),
				};

				ScriptValue::Number(value)
			},
			Rule::unit_number => {
				let mut pair = pair.into_inner();
				let value = pair.next().unwrap();
				let unit = pair.next().unwrap();
				let value = match value.as_rule() {
					Rule::integer => Number::from_int_and_unit(value.as_str().parse().unwrap(), unit.as_str()),
					Rule::decimal => Number::from_float_and_unit(value.as_str().parse().unwrap(), unit.as_str()),
					_ => unreachable!(),
				};

				ScriptValue::Number(value)
			},
			Rule::ident => {
				let ident = pair.as_str();
				if let Some(value) = self.global_vars.get(ident) {
					value.clone()
				} else {
					return Err(pest::error::Error::new_from_span(
						pest::error::ErrorVariant::<()>::CustomError {
							message: "Variable not found".to_string(),
						},
						pair.as_span(),
					)
					.into());
				}
			},
			Rule::forLoop => {
				let mut pair = pair.into_inner();
				let loop_variable = pair.next().unwrap().as_str();
				let range = pair.next().unwrap();
				let range_span = range.as_span();
				let range = self.exec(range)?;
				let block = pair.next().unwrap();

				if let ScriptValue::Range { start, step, num } = range {
					for i in 0..num {
						self.global_vars
							.insert(loop_variable.to_string(), ScriptValue::Number(start + step * (i as i64).into()));
						self.exec(block.clone())?;
					}
				} else {
					return Err(pest::error::Error::new_from_span(
						pest::error::ErrorVariant::<()>::CustomError {
							message: "Expected range".to_string(),
						},
						range_span,
					)
					.into());
				}

				ScriptValue::Null
			},
			Rule::block => {
				for pair in pair.into_inner() {
					self.exec(pair)?;
				}

				ScriptValue::Null
			},
			unknown_expr => panic!("Unexpected expr: {:?}", unknown_expr),
		})
	}

	fn parse_func_parameters(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<(Vec<ScriptValue>, HashMap<String, ScriptValue>)> {
		let mut positional_args = Vec::new();
		let mut named_args = HashMap::new();

		for arg in pair.into_inner() {
			match arg.as_rule() {
				Rule::positionalParam => positional_args.push(self.exec(arg.into_inner().next().unwrap())?),
				Rule::namedParam => {
					let mut pair = arg.into_inner();
					let ident = pair.next().unwrap();
					let expr = pair.next().unwrap();
					named_args.insert(ident.as_str().to_string(), self.exec(expr)?);
				},
				_ => unreachable!(),
			}
		}

		Ok((positional_args, named_args))
	}
}


struct Material {
	stepover: f64,
	depth_per_pass: f64,
	feed_rate: f64,
	plunge_rate: f64,
	rpm: f64,
}
