use std::ops::{Add, Div, Sub, Mul};

use crate::numbers::Number;


#[derive(Debug, Clone)]
pub enum ScriptValue {
	Number(Number),
	String(String),
	Range {
		start: Number,
		step: Number,
		num: usize,
	},
	Null,
}

macro_rules! math_impl {
	($($t:ty,$i:ident,$op:ident)*) => ($(
		impl $i for $t {
			type Output = ScriptValue;

			fn $op(self, other: $t) -> ScriptValue {
				match (&self, &other) {
					(ScriptValue::Number(a), ScriptValue::Number(b)) => ScriptValue::Number(Number::$op(*a, *b)),
					_ => panic!("Cannot do math on non-numbers"),
				}
			}
		}
	)*)
}

math_impl! {
	ScriptValue, Add, add
	ScriptValue, Sub, sub
	ScriptValue, Mul, mul
	ScriptValue, Div, div
}

impl TryFrom<ScriptValue> for String {
	type Error = &'static str;

	fn try_from(value: ScriptValue) -> Result<Self, Self::Error> {
		match value {
			ScriptValue::String(s) => Ok(s),
			_ => Err("Not a string"),
		}
	}
}