use std::collections::HashMap;

use gcad_macro::ffi_func;

use anyhow::{anyhow, bail, Result};
use nalgebra::{Matrix3, Vector2};

use crate::{
	numbers::{Number, Unit},
	value::ScriptValue,
};

use super::{Material, ScriptEngine};

impl ScriptEngine {
	pub fn call_builtin(&mut self, ident: &str, args: &[ScriptValue], nargs: &HashMap<String, ScriptValue>) -> Result<Option<ScriptValue>> {
		Ok(match ident {
			"rpm" => Some(self.builtin_rpm_ffi(args, nargs)?),
			"material" => Some(self.builtin_material_ffi(args, nargs)?),
			"cutter_diameter" => Some(self.builtin_cutter_diameter_ffi(args, nargs)?),
			"contour_line" => Some(self.builtin_contour_line_ffi(args, nargs)?),
			"define_material" => Some(self.builtin_define_material_ffi(args, nargs)?),
			"drill" => Some(self.builtin_drill_ffi(args, nargs)?),
			"circle_pocket" => Some(self.builtin_circle_pocket_ffi(args, nargs)?),
			"groove_pocket" => Some(self.builtin_groove_pocket_ffi(args, nargs)?),
			"comment" => Some(self.builtin_comment_ffi(args, nargs)?),
			"linspace" => Some(self.builtin_linspace_ffi(args, nargs)?),
			"scale" => Some(self.builtin_scale_ffi(args, nargs)?),
			_ => None,
		})
	}

	#[ffi_func]
	fn builtin_rpm(&mut self, rpm: Number) -> Result<ScriptValue> {
		let rpm = rpm.as_float().ok_or(anyhow!("rpm: argument 0 must be a number"))?;

		self.gcode.set_rpm(rpm);

		Ok(ScriptValue::Null)
	}

	#[ffi_func]
	fn builtin_material(&mut self, name: String) -> Result<ScriptValue> {
		if let Some(material) = self.materials.get(&name) {
			self.gcode.stepover = material.stepover;
			self.gcode.depth_per_pass = material.depth_per_pass;
			self.gcode.feed_rate = material.feed_rate;
			self.gcode.plunge_rate = material.plunge_rate;

			self.gcode.set_rpm(material.rpm);
		} else {
			bail!("Unknown material: {}", name);
		}

		Ok(ScriptValue::Null)
	}

	#[ffi_func]
	fn builtin_cutter_diameter(&mut self, diameter: Number) -> Result<ScriptValue> {
		if diameter.unit == Unit::None {
			bail!("diameter must have a unit");
		}

		self.gcode.cutter_diameter = diameter.convert_unit(Unit::MM).into();

		Ok(ScriptValue::Null)
	}

	#[ffi_func]
	fn builtin_contour_line(
		&mut self,
		x1: Number,
		y1: Number,
		x2: Option<Number>,
		y2: Option<Number>,
		depth: Number,
		up: Option<Number>,
	) -> Result<ScriptValue> {
		let (x2, y2) = if let Some(up) = up {
			if up.unit == Unit::None {
				bail!("up must have a unit");
			}

			(x1, y1 + up)
		} else if let (Some(x2), Some(y2)) = (x2, y2) {
			(x2, y2)
		} else {
			bail!("Either x2/y2 must be specified or another argument like up");
		};

		if x1.unit == Unit::None || y1.unit == Unit::None || x2.unit == Unit::None || y2.unit == Unit::None || depth.unit == Unit::None {
			bail!("All arguments must have a unit");
		}

		self.gcode.contour_line(
			x1.convert_unit(Unit::MM).into(),
			y1.convert_unit(Unit::MM).into(),
			x2.convert_unit(Unit::MM).into(),
			y2.convert_unit(Unit::MM).into(),
			depth.convert_unit(Unit::MM).into(),
		);

		Ok(ScriptValue::Null)
	}

	#[ffi_func]
	fn builtin_drill(&mut self, x: Number, y: Number, depth: Number) -> Result<ScriptValue> {
		if x.unit == Unit::None || y.unit == Unit::None || depth.unit == Unit::None {
			bail!("All arguments must have a unit");
		}

		self.gcode.drill(
			x.convert_unit(Unit::MM).into(),
			y.convert_unit(Unit::MM).into(),
			depth.convert_unit(Unit::MM).into(),
		);

		Ok(ScriptValue::Null)
	}

	#[ffi_func]
	fn builtin_circle_pocket(&mut self, cx: Number, cy: Number, diameter: Option<Number>, radius: Option<Number>, depth: Number) -> Result<ScriptValue> {
		let diameter = if let Some(diameter) = diameter {
			diameter
		} else if let Some(radius) = radius {
			radius * 2.0.into()
		} else {
			bail!("Either diameter or radius must be specified");
		};

		if cx.unit == Unit::None || cy.unit == Unit::None || diameter.unit == Unit::None || depth.unit == Unit::None {
			bail!("All arguments must have a unit");
		}

		self.gcode.circle_pocket(
			cx.convert_unit(Unit::MM).into(),
			cy.convert_unit(Unit::MM).into(),
			diameter.convert_unit(Unit::MM).into(),
			depth.convert_unit(Unit::MM).into(),
		)?;

		Ok(ScriptValue::Null)
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
	) -> Result<ScriptValue> {
		let material = Material {
			stepover: stepover.as_float().ok_or(anyhow!("stepover must be a number"))?,
			depth_per_pass: depth_per_pass.as_float().ok_or(anyhow!("depth_per_pass must be a number"))?,
			feed_rate: feed_rate.as_float().ok_or(anyhow!("feed_rate must be a number"))?,
			plunge_rate: plunge_rate.as_float().ok_or(anyhow!("plunge_rate must be a number"))?,
			rpm: rpm.as_float().ok_or(anyhow!("rpm must be a number"))?,
		};

		self.materials.insert(name, material);

		Ok(ScriptValue::Null)
	}

	#[ffi_func]
	fn builtin_groove_pocket(&mut self, x: Number, y: Number, width: Number, height: Number, depth: Number) -> Result<ScriptValue> {
		if x.unit == Unit::None || y.unit == Unit::None || width.unit == Unit::None || height.unit == Unit::None || depth.unit == Unit::None {
			bail!("All arguments must have a unit");
		}

		self.gcode.groove_pocket(
			x.convert_unit(Unit::MM).into(),
			y.convert_unit(Unit::MM).into(),
			width.convert_unit(Unit::MM).into(),
			height.convert_unit(Unit::MM).into(),
			depth.convert_unit(Unit::MM).into(),
		)?;

		Ok(ScriptValue::Null)
	}

	#[ffi_func]
	fn builtin_comment(&mut self, text: String) -> Result<ScriptValue> {
		self.gcode.write_comment(&text);

		Ok(ScriptValue::Null)
	}

	#[ffi_func]
	fn builtin_linspace(&mut self, start: Number, stop: Number, num: Number) -> Result<ScriptValue> {
		if num.unit != Unit::None {
			bail!("num must not have a unit");
		}

		if start.unit == Unit::None && stop.unit != Unit::None {
			bail!("start must have a unit if stop has a unit");
		}

		if start.unit != Unit::None && stop.unit == Unit::None {
			bail!("stop must have a unit if start has a unit");
		}

		let stop = stop.convert_unit(start.unit);
		let num: i64 = num.try_into().map_err(|_| anyhow!("num argument must be an integer"))?;
		let step = (stop - start) / (num - 1).into();
		let num: usize = num.try_into().map_err(|_| anyhow!("num argument must be a positive integer"))?;

		Ok(ScriptValue::Range { start, step, num })
	}

	#[ffi_func]
	fn builtin_scale(&mut self, x: Number, y: Number) -> Result<ScriptValue> {
		if x.unit != Unit::None || y.unit != Unit::None {
			bail!("All arguments must not have a unit");
		}

		self.gcode.transformation = Matrix3::new_nonuniform_scaling(&Vector2::new(x.into(), y.into()));

		Ok(ScriptValue::Null)
	}
}
