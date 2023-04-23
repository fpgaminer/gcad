use std::ops::{Add, Div, Mul, Neg, Sub};

use crate::numbers::Number;


#[derive(Debug, Clone)]
pub enum ScriptValue {
	Number(Number),
	String(String),
	Range { start: Number, step: Number, num: usize },
	Null,
}

impl ScriptValue {
	pub fn pow(&self, other: &ScriptValue) -> ScriptValue {
		match (self, other) {
			(ScriptValue::Number(a), ScriptValue::Number(b)) => ScriptValue::Number(a.pow(b)),
			_ => panic!("Cannot do math on non-numbers"),
		}
	}

	pub fn factorial(&self) -> ScriptValue {
		match self {
			ScriptValue::Number(a) => ScriptValue::Number(a.factorial()),
			_ => panic!("Cannot do math on non-numbers"),
		}
	}
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

impl Neg for ScriptValue {
	type Output = ScriptValue;

	fn neg(self) -> ScriptValue {
		match self {
			ScriptValue::Number(a) => ScriptValue::Number(-a),
			_ => panic!("Cannot do math on non-numbers"),
		}
	}
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
