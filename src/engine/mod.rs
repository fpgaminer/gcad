mod builtins;

use std::{collections::HashMap, path::Path};

use pest::{
	iterators::Pair,
	prec_climber::{Assoc, PrecClimber},
	Parser,
};
use pest_derive::Parser;

use crate::{gcode::GcodeState, numbers::Number, value::ScriptValue};


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
	pub fn new<P: AsRef<Path>>(output_path: P) -> Self {
		let gcode = GcodeState::new(output_path);

		Self {
			global_vars: HashMap::new(),
			materials: HashMap::new(),
			gcode,
		}
	}

	pub fn run(&mut self, source: &str) {
		let pairs = ScriptParser::parse(Rule::program, source).unwrap();

		self.format_parse_tree(pairs.clone(), 0);

		for pair in pairs {
			match pair.as_rule() {
				Rule::expr | Rule::forLoop => {
					self.exec(pair);
				},
				Rule::EOI => {},
				_ => panic!("Unexpected rule: {:?}", pair.as_rule()),
			}
		}
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

	pub fn write_header(&mut self) {
		self.gcode.write_header();
	}

	pub fn finish(&mut self) {
		self.gcode.finish();
	}

	fn exec(&mut self, pair: pest::iterators::Pair<Rule>) -> ScriptValue {
		println!("build_ast_from_expr: {:?}", pair);
		println!("Rule: {:?}", pair.as_rule());

		match pair.as_rule() {
			Rule::expr => self.exec(pair.into_inner().next().unwrap()),
			Rule::assign => {
				let mut pair = pair.into_inner();
				let ident = pair.next().unwrap();
				let expr = pair.next().unwrap();
				let expr = self.exec(expr);

				self.global_vars.insert(ident.as_str().to_string(), expr.clone());

				expr
			},
			Rule::mathExpr => CLIMBER.climb(
				pair.into_inner(),
				|pair: Pair<Rule>| self.exec(pair),
				|lhs: ScriptValue, op: Pair<Rule>, rhs: ScriptValue| match op.as_rule() {
					Rule::add => lhs + rhs,
					Rule::subtract => lhs - rhs,
					Rule::multiply => lhs * rhs,
					Rule::divide => lhs / rhs,
					_ => unreachable!(),
				},
			),
			Rule::string => {
				let str = &pair.as_str();
				let str = &str[1..str.len() - 1];
				let str = str.replace("''", "'");
				ScriptValue::String(str.to_string())
			},
			Rule::funcCall => {
				let mut pair = pair.into_inner();
				let ident = pair.next().unwrap();
				let ident = ident.as_str();
				let (args, nargs) = self.parse_func_parameters(pair.next().unwrap());

				if let Some(ret) = self.call_builtin(ident, &args, &nargs) {
					ret
				} else {
					panic!("Function not found: {:?}", ident);
				}
			},
			Rule::unitless_number => {
				let mut pair = pair.into_inner();
				let value = pair.next().unwrap();
				let value = match value.as_rule() {
					Rule::integer => Number::from_int(value.as_str().parse::<i64>().unwrap()),
					Rule::decimal => Number::from_float(value.as_str().parse::<f64>().unwrap()),
					_ => panic!("Unexpected rule: {:?}", value.as_rule()),
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
					_ => panic!("Unexpected rule: {:?}", value.as_rule()),
				};

				ScriptValue::Number(value)
			},
			Rule::ident => {
				let ident = pair.as_str();
				if let Some(value) = self.global_vars.get(ident) {
					value.clone()
				} else {
					panic!("Variable not found: {:?}", ident);
				}
			},
			Rule::forLoop => {
				let mut pair = pair.into_inner();
				let loop_variable = pair.next().unwrap().as_str();
				let range = self.exec(pair.next().unwrap());
				let block = pair.next().unwrap();

				println!("For loop: {} in {:?}", loop_variable, range);

				if let ScriptValue::Range { start, step, num } = range {
					for i in 0..num {
						self.global_vars
							.insert(loop_variable.to_string(), ScriptValue::Number(start + step * (i as i64).into()));
						self.exec(block.clone());
					}
				} else {
					panic!("Invalid range");
				}

				ScriptValue::Null
			},
			Rule::block => {
				for pair in pair.into_inner() {
					self.exec(pair);
				}

				ScriptValue::Null
			},
			unknown_expr => panic!("Unexpected expr: {:?}", unknown_expr),
		}
	}

	fn parse_func_parameters(&mut self, pair: pest::iterators::Pair<Rule>) -> (Vec<ScriptValue>, HashMap<String, ScriptValue>) {
		let mut positional_args = Vec::new();
		let mut named_args = HashMap::new();

		for arg in pair.into_inner() {
			match arg.as_rule() {
				Rule::positionalParam => positional_args.push(self.exec(arg.into_inner().next().unwrap())),
				Rule::namedParam => {
					let mut pair = arg.into_inner();
					let ident = pair.next().unwrap();
					let expr = pair.next().unwrap();
					named_args.insert(ident.as_str().to_string(), self.exec(expr));
				},
				unknown_param => panic!("Unexpected param: {:?}", unknown_param),
			}
		}

		(positional_args, named_args)
	}
}


struct Material {
	stepover: f64,
	depth_per_pass: f64,
	feed_rate: f64,
	plunge_rate: f64,
	rpm: f64,
}
