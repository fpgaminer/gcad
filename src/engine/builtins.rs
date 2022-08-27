use std::collections::HashMap;

use gcad_macro::ffi_func;

use crate::{
	numbers::{Number, Unit},
	value::ScriptValue,
};

use super::{Material, ScriptEngine};

impl ScriptEngine {
	pub fn call_builtin(&mut self, ident: &str, args: &[ScriptValue], nargs: &HashMap<String, ScriptValue>) -> Option<ScriptValue> {
		match ident {
			"rpm" => Some(self.builtin_rpm_ffi(args, nargs)),
			"material" => Some(self.builtin_material_ffi(args, nargs)),
			"cutter_diameter" => Some(self.builtin_cutter_diameter_ffi(args, nargs)),
			"contour_line" => Some(self.builtin_contour_line_ffi(args, nargs)),
			"define_material" => Some(self.builtin_define_material_ffi(args, nargs)),
			"drill" => Some(self.builtin_drill_ffi(args, nargs)),
			"circle_pocket" => Some(self.builtin_circle_pocket_ffi(args, nargs)),
			"groove_pocket" => Some(self.builtin_groove_pocket_ffi(args, nargs)),
			"comment" => Some(self.builtin_comment_ffi(args, nargs)),
			"linspace" => Some(self.builtin_linspace_ffi(args, nargs)),
			_ => None,
		}
	}

	#[ffi_func]
	fn builtin_rpm(&mut self, rpm: Number) -> ScriptValue {
		let rpm = rpm.as_float().expect("rpm argument must be a number");

		self.gcode.set_rpm(rpm);

		ScriptValue::Null
	}

	#[ffi_func]
	fn builtin_material(&mut self, name: String) -> ScriptValue {
		if let Some(material) = self.materials.get(&name) {
			self.gcode.stepover = material.stepover;
			self.gcode.depth_per_pass = material.depth_per_pass;
			self.gcode.feed_rate = material.feed_rate;
			self.gcode.plunge_rate = material.plunge_rate;

			self.gcode.set_rpm(material.rpm);
		} else {
			panic!("Material not found: {}", name);
		}

		ScriptValue::Null
	}

	#[ffi_func]
	fn builtin_cutter_diameter(&mut self, diameter: Number) -> ScriptValue {
		println!("cutter_diameter: {:?}", diameter);
		if diameter.unit == Unit::None {
			panic!("cutter_diameter argument must have a unit");
		}

		self.gcode.cutter_diameter = diameter.convert_unit(Unit::MM).into();

		ScriptValue::Null
	}

	#[ffi_func]
	fn builtin_contour_line(&mut self, x1: Number, y1: Number, x2: Option<Number>, y2: Option<Number>, depth: Number, up: Option<Number>) -> ScriptValue {
		let (x2, y2) = if let Some(up) = up {
			if up.unit == Unit::None {
				panic!("contour_line up argument must have a unit");
			}

			(x1, y1 + up)
		} else if let (Some(x2), Some(y2)) = (x2, y2) {
			(x2, y2)
		} else {
			panic!("contour_line requires either x2/y2 or another argument that derives them like up");
		};

		if x1.unit == Unit::None || y1.unit == Unit::None || x2.unit == Unit::None || y2.unit == Unit::None || depth.unit == Unit::None {
			panic!("contour_line arguments must have a unit");
		}

		self.gcode.contour_line(
			x1.convert_unit(Unit::MM).into(),
			y1.convert_unit(Unit::MM).into(),
			x2.convert_unit(Unit::MM).into(),
			y2.convert_unit(Unit::MM).into(),
			depth.convert_unit(Unit::MM).into(),
		);

		ScriptValue::Null
	}

	#[ffi_func]
	fn builtin_drill(&mut self, x: Number, y: Number, depth: Number) -> ScriptValue {
		if x.unit == Unit::None || y.unit == Unit::None || depth.unit == Unit::None {
			panic!("drill arguments must have a unit");
		}

		self.gcode.drill(
			x.convert_unit(Unit::MM).into(),
			y.convert_unit(Unit::MM).into(),
			depth.convert_unit(Unit::MM).into(),
		);

		ScriptValue::Null
	}

	#[ffi_func]
	fn builtin_circle_pocket(&mut self, cx: Number, cy: Number, diameter: Option<Number>, radius: Option<Number>, depth: Number) -> ScriptValue {
		let diameter = if let Some(diameter) = diameter {
			diameter
		} else if let Some(radius) = radius {
			radius * 2.0.into()
		} else {
			panic!("circle_pocket requires either diameter or radius");
		};

		if cx.unit == Unit::None || cy.unit == Unit::None || diameter.unit == Unit::None || depth.unit == Unit::None {
			panic!("circle_pocket arguments must have a unit");
		}

		self.gcode.circle_pocket(
			cx.convert_unit(Unit::MM).into(),
			cy.convert_unit(Unit::MM).into(),
			diameter.convert_unit(Unit::MM).into(),
			depth.convert_unit(Unit::MM).into(),
		);

		ScriptValue::Null
	}

	#[ffi_func]
	fn builtin_define_material(
		&mut self,
		name: String,
		stepover: Number,
		depth_per_pass: Number,
		feed_rate: Number,
		plunge_rate: Number,
		rpm: Number,
	) -> ScriptValue {
		let material = Material {
			stepover: stepover.as_float().expect("stepover must be a number"),
			depth_per_pass: depth_per_pass.as_float().expect("depth_per_pass must be a number"),
			feed_rate: feed_rate.as_float().expect("feed_rate must be a number"),
			plunge_rate: plunge_rate.as_float().expect("plunge_rate must be a number"),
			rpm: rpm.as_float().expect("rpm must be a number"),
		};

		self.materials.insert(name, material);

		ScriptValue::Null
	}

	#[ffi_func]
	fn builtin_groove_pocket(&mut self, x: Number, y: Number, width: Number, height: Number, depth: Number) -> ScriptValue {
		if x.unit == Unit::None || y.unit == Unit::None || width.unit == Unit::None || height.unit == Unit::None || depth.unit == Unit::None {
			panic!("groove_pocket arguments must have a unit");
		}

		self.gcode.groove_pocket(
			x.convert_unit(Unit::MM).into(),
			y.convert_unit(Unit::MM).into(),
			width.convert_unit(Unit::MM).into(),
			height.convert_unit(Unit::MM).into(),
			depth.convert_unit(Unit::MM).into(),
		);

		ScriptValue::Null
	}

	#[ffi_func]
	fn builtin_comment(&mut self, text: String) -> ScriptValue {
		self.gcode.write_comment(&text);

		ScriptValue::Null
	}

	#[ffi_func]
	fn builtin_linspace(&mut self, start: Number, stop: Number, num: Number) -> ScriptValue {
		if num.unit != Unit::None {
			panic!("linspace num argument must not have a unit");
		}

		if start.unit == Unit::None && stop.unit != Unit::None {
			panic!("linspace start argument must have a unit if stop argument has a unit");
		}

		if start.unit != Unit::None && stop.unit == Unit::None {
			panic!("linspace stop argument must have a unit if start argument has a unit");
		}

		let stop = stop.convert_unit(start.unit);
		let num: i64 = num.try_into().expect("linspace num argument must be an integer");
		let step = (stop - start) / (num - 1).into();
		let num: usize = num.try_into().expect("linspace num argument must be a positive integer");

		ScriptValue::Range { start, step, num }
	}
}
