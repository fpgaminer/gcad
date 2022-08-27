use std::{ops::{Add, Div, Sub, Mul}, str::FromStr};

use crate::value::ScriptValue;


#[derive(PartialEq, Debug, Clone, Copy)]
pub struct Number {
	pub value: InnerValue,
	pub unit: Unit,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Unit {
	MM, CM, M,
	FT, IN, YD,
	None,
}

impl FromStr for Unit {
	type Err = ();
	
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"mm" => Ok(Unit::MM),
			"cm" => Ok(Unit::CM),
			"m" => Ok(Unit::M),
			"ft" => Ok(Unit::FT),
			"in" => Ok(Unit::IN),
			"yd" => Ok(Unit::YD),
			_ => Err(()),
		}
	}
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum InnerValue {
	Integer(i64),
	Float(f64),
}

impl InnerValue {
	pub fn as_float(&self) -> f64 {
		match self {
			InnerValue::Integer(i) => *i as f64,
			InnerValue::Float(f) => *f,
		}
	}
}

impl Add for InnerValue {
	type Output = InnerValue;
	
	fn add(self, other: InnerValue) -> InnerValue {
		match (self, other) {
			(InnerValue::Integer(i), InnerValue::Integer(j)) => InnerValue::Integer(i + j),
			(InnerValue::Integer(i), InnerValue::Float(j)) => InnerValue::Float(i as f64 + j),
			(InnerValue::Float(i), InnerValue::Float(j)) => InnerValue::Float(i + j),
			(InnerValue::Float(i), InnerValue::Integer(j)) => InnerValue::Float(i + j as f64),
		}
	}
}

impl Sub for InnerValue {
	type Output = InnerValue;
	
	fn sub(self, other: InnerValue) -> InnerValue {
		match (self, other) {
			(InnerValue::Integer(i), InnerValue::Integer(j)) => InnerValue::Integer(i - j),
			(InnerValue::Integer(i), InnerValue::Float(j)) => InnerValue::Float(i as f64 - j),
			(InnerValue::Float(i), InnerValue::Float(j)) => InnerValue::Float(i - j),
			(InnerValue::Float(i), InnerValue::Integer(j)) => InnerValue::Float(i - j as f64),
		}
	}
}

impl Mul for InnerValue {
	type Output = InnerValue;
	
	fn mul(self, other: InnerValue) -> InnerValue {
		match (self, other) {
			(InnerValue::Integer(i), InnerValue::Integer(j)) => InnerValue::Integer(i * j),
			(InnerValue::Integer(i), InnerValue::Float(j)) => InnerValue::Float(i as f64 * j),
			(InnerValue::Float(i), InnerValue::Float(j)) => InnerValue::Float(i * j),
			(InnerValue::Float(i), InnerValue::Integer(j)) => InnerValue::Float(i * j as f64),
		}
	}
}

impl Div for InnerValue {
	type Output = InnerValue;
	
	fn div(self, other: InnerValue) -> InnerValue {
		match (self, other) {
			(InnerValue::Integer(i), InnerValue::Integer(j)) => InnerValue::Float(i as f64 / j as f64),
			(InnerValue::Integer(i), InnerValue::Float(j)) => InnerValue::Float(i as f64 / j),
			(InnerValue::Float(i), InnerValue::Float(j)) => InnerValue::Float(i / j),
			(InnerValue::Float(i), InnerValue::Integer(j)) => InnerValue::Float(i / j as f64),
		}
	}
}

impl Number {
	pub fn from_float_and_unit(f: f64, unit: &str) -> Number {
		Number {
			value: InnerValue::Float(f),
			unit: unit.parse().expect("Could not parse unit"),
		}
	}

	pub fn from_int_and_unit(i: i64, unit: &str) -> Number {
		Number {
			value: InnerValue::Integer(i),
			unit: unit.parse().expect("Could not parse unit"),
		}
	}

	pub fn from_float(f: f64) -> Number {
		Number {
			value: InnerValue::Float(f),
			unit: Unit::None,
		}
	}

	pub fn from_int(i: i64) -> Number {
		Number {
			value: InnerValue::Integer(i),
			unit: Unit::None,
		}
	}

	pub fn as_float(&self) -> Option<f64> {
		match (self.value, self.unit) {
			(InnerValue::Integer(i), Unit::None) => Some(i as f64),
			(InnerValue::Float(f), Unit::None) => Some(f),
			_ => None,
		}
	}

	pub fn convert_unit(&self, unit: Unit) -> Number {
		let value = self.value.as_float();

		let value = match (self.unit, unit) {
			(Unit::None, _) => self.value,
			(_, Unit::None) => self.value,
			(Unit::MM, Unit::MM) => self.value,
			(Unit::MM, Unit::CM) => InnerValue::Float(value / 10.0),
			(Unit::MM, Unit::M) => InnerValue::Float(value / 1000.0),
			(Unit::MM, Unit::IN) => InnerValue::Float(value / 25.4),
			(Unit::MM, Unit::FT) => InnerValue::Float(value / 304.8),
			(Unit::MM, Unit::YD) => InnerValue::Float(value / 914.4),

			(Unit::CM, Unit::MM) => InnerValue::Float(value * 10.0),
			(Unit::CM, Unit::CM) => self.value,
			(Unit::CM, Unit::M) => InnerValue::Float(value / 100.0),
			(Unit::CM, Unit::IN) => InnerValue::Float(value / 2.54),
			(Unit::CM, Unit::FT) => InnerValue::Float(value / 30.48),
			(Unit::CM, Unit::YD) => InnerValue::Float(value / 91.44),

			(Unit::M, Unit::MM) => InnerValue::Float(value * 1000.0),
			(Unit::M, Unit::CM) => InnerValue::Float(value * 100.0),
			(Unit::M, Unit::M) => self.value,
			(Unit::M, Unit::IN) => InnerValue::Float(value / 0.0254),
			(Unit::M, Unit::FT) => InnerValue::Float(value / 0.3048),
			(Unit::M, Unit::YD) => InnerValue::Float(value / 0.9144),

			(Unit::IN, Unit::MM) => InnerValue::Float(value * 25.4),
			(Unit::IN, Unit::CM) => InnerValue::Float(value * 2.54),
			(Unit::IN, Unit::M) => InnerValue::Float(value * 0.0254),
			(Unit::IN, Unit::IN) => self.value,
			(Unit::IN, Unit::FT) => InnerValue::Float(value / 12.0),
			(Unit::IN, Unit::YD) => InnerValue::Float(value / 36.0),

			(Unit::FT, Unit::MM) => InnerValue::Float(value * 12.0 * 25.4),
			(Unit::FT, Unit::CM) => InnerValue::Float(value * 12.0 * 2.54),
			(Unit::FT, Unit::M) => InnerValue::Float(value * 12.0 * 0.0254),
			(Unit::FT, Unit::IN) => InnerValue::Float(value * 12.0),
			(Unit::FT, Unit::FT) => self.value,
			(Unit::FT, Unit::YD) => InnerValue::Float(value / 3.0),

			(Unit::YD, Unit::MM) => InnerValue::Float(value * 3.0 * 12.0 * 25.4),
			(Unit::YD, Unit::CM) => InnerValue::Float(value * 3.0 * 12.0 * 2.54),
			(Unit::YD, Unit::M) => InnerValue::Float(value * 3.0 * 12.0 * 0.0254),
			(Unit::YD, Unit::IN) => InnerValue::Float(value * 3.0 * 12.0),
			(Unit::YD, Unit::FT) => InnerValue::Float(value * 3.0),
			(Unit::YD, Unit::YD) => self.value,
		};

		Number {
			value,
			unit,
		}
	}
}

fn convert_units_for_math(lhs: &Number, rhs: &Number) -> (Number, Number) {
	// If only one of the numbers has a unit, use that unit.
	// Otherwise, use the unit of the first number (lhs).
	let dst_unit = if lhs.unit == Unit::None {
		rhs.unit
	} else {
		lhs.unit
	};

	(lhs.convert_unit(dst_unit), rhs.convert_unit(dst_unit))
}

macro_rules! math_impl {
	($($t:ty,$i:ident,$op:ident)*) => ($(
		impl $i for $t {
			type Output = Number;

			fn $op(self, other: $t) -> Number {
				let (lhs, rhs) = convert_units_for_math(&self, &other);

				Number {
					value: lhs.value.$op(rhs.value),
					unit: lhs.unit,
				}
			}
		}
	)*)
}

math_impl! {
	Number, Add, add
	Number, Sub, sub
	Number, Mul, mul
	Number, Div, div
}

impl TryFrom<ScriptValue> for Number {
	type Error = &'static str;
	
	fn try_from(value: ScriptValue) -> Result<Self, Self::Error> {
		match value {
			ScriptValue::Number(n) => Ok(n),
			_ => Err("Not a number"),
		}
	}
}

impl From<Number> for f64 {
	fn from(n: Number) -> f64 {
		n.value.into()
	}
}

impl From<InnerValue> for f64 {
	fn from(value: InnerValue) -> f64 {
		match value {
			InnerValue::Float(f) => f,
			InnerValue::Integer(i) => i as f64,
		}
	}
}

impl From<i64> for Number {
	fn from(value: i64) -> Number {
		Number {
			value: InnerValue::Integer(value),
			unit: Unit::None,
		}
	}
}

impl From<f64> for Number {
	fn from(value: f64) -> Number {
		Number {
			value: InnerValue::Float(value),
			unit: Unit::None,
		}
	}
}

impl TryFrom<Number> for i64 {
	type Error = &'static str;
	
	fn try_from(n: Number) -> Result<Self, Self::Error> {
		match n.value {
			InnerValue::Integer(i) => Ok(i),
			_ => Err("Not an integer"),
		}
	}
}