use std::{
	fs::File,
	io::{BufWriter, Write},
	path::Path,
};

use anyhow::{bail, Context, Result, anyhow};
use nalgebra::{Matrix3, Point2};

const RETRACT: f64 = 0.25;

pub struct GcodeState {
	file: BufWriter<File>,

	pub stepover: f64,
	pub depth_per_pass: f64,
	pub feed_rate: f64,
	pub plunge_rate: f64,
	pub cutter_diameter: f64,

	last_feed: Option<f64>,
	last_command: Option<String>,

	position_xy: Option<Point2<f64>>,
	position_z: Option<f64>,

	pub transformation: Matrix3<f64>,
}

impl GcodeState {
	pub fn new<P: AsRef<Path>>(output_path: P) -> Result<GcodeState> {
		let file = File::create(output_path.as_ref()).with_context(|| format!("Failed to create file: {}", output_path.as_ref().display()))?;

		Ok(GcodeState {
			file: BufWriter::new(file),

			stepover: 0.0,
			depth_per_pass: 0.0,
			feed_rate: 0.0,
			plunge_rate: 0.0,
			cutter_diameter: 0.0,

			last_feed: None,
			last_command: None,

			position_xy: None,
			position_z: None,

			transformation: Matrix3::identity(),
		})
	}

	pub fn write_header(&mut self) -> Result<()> {
		self.file
			.write_all("G90\nG21\n(Move to safe Z to avoid workholding)\nG53G0Z-5.000\nM05\n".as_bytes())?;
		Ok(())
	}

	pub fn set_rpm(&mut self, rpm: f64) -> Result<()> {
		self.file.write_all(format!("M03S{}\n", format_number(rpm)).as_bytes())?;
		Ok(())
	}

	pub fn write_comment(&mut self, comment: &str) -> Result<()> {
		self.file.write_all(format!("( {} )\n", comment).as_bytes())?;
		Ok(())
	}

	pub fn cutting_move(&mut self, x: Option<f64>, y: Option<f64>, z: Option<f64>) -> Result<()> {
		let xy = match (x, y) {
			(None, None) => None,
			(x, y) => Some(Point2::new(
				x.map(|x| Ok(x)).unwrap_or_else(|| self.position_xy.ok_or(anyhow!("No position set")).map(|p| p.x))?,
				y.map(|y| Ok(y)).unwrap_or_else(|| self.position_xy.ok_or(anyhow!("No position set")).map(|p| p.y))?,
			)),
		};

		let xy = xy.map(|xy| self.transformation.transform_point(&xy));

		if xy.map(|xy| Some(xy) == self.position_xy).unwrap_or(true) && z.map(|z| Some(z) == self.position_z).unwrap_or(true) {
			return Ok(());
		}

		let mut command = Vec::new();

		if self.last_command != Some("G1".to_string()) {
			command.push("G1".to_string());

			self.last_command = Some("G1".to_string());
		}

		if let Some(xy) = xy {
			if Some(xy.x) != self.position_xy.map(|p| p.x) {
				command.push(format!("X{}", format_number(xy.x)));
			}

			if Some(xy.y) != self.position_xy.map(|p| p.y) {
				command.push(format!("Y{}", format_number(xy.y)));
			}

			self.position_xy = Some(xy);
		}

		if let Some(z) = z {
			if Some(z) != self.position_z {
				command.push(format!("Z{}", format_number(z)));
			}

			self.position_z = Some(z);
		}

		if self.last_feed != Some(self.feed_rate) {
			command.push(format!("F{}", format_number(self.feed_rate)));

			self.last_feed = Some(self.feed_rate);
		}

		self.file.write_all(format!("{}\n", command.join(" ")).as_bytes())?;

		Ok(())
	}

	pub fn plunge(&mut self, z: f64) -> Result<()> {
		if self.position_z == Some(z) {
			return Ok(());
		}

		let mut command = Vec::new();

		if self.last_command != Some("G1".to_string()) {
			command.push("G1".to_string());
		}

		command.push(format!("Z{}", format_number(z)));

		if self.last_feed != Some(self.plunge_rate) {
			command.push(format!("F{}", format_number(self.plunge_rate)));
		}

		self.file.write_all(format!("{}\n", command.join(" ")).as_bytes())?;

		self.last_feed = Some(self.plunge_rate);
		self.last_command = Some("G1".to_string());
		self.position_z = Some(z);

		Ok(())
	}

	pub fn rapid_move(&mut self, x: Option<f64>, y: Option<f64>, z: Option<f64>) -> Result<()> {
		let xy = match (x, y) {
			(None, None) => None,
			(x, y) => Some(Point2::new(
				x.map(|x| Ok(x)).unwrap_or_else(|| self.position_xy.ok_or(anyhow!("No position set")).map(|p| p.x))?,
				y.map(|y| Ok(y)).unwrap_or_else(|| self.position_xy.ok_or(anyhow!("No position set")).map(|p| p.y))?,
			)),
		};

		let xy = xy.map(|xy| self.transformation.transform_point(&xy));

		if xy.map(|xy| Some(xy) == self.position_xy).unwrap_or(true) && z.map(|z| Some(z) == self.position_z).unwrap_or(true) {
			return Ok(());
		}

		let mut command = Vec::new();

		if self.last_command != Some("G0".to_string()) {
			command.push("G0".to_string());

			self.last_command = Some("G0".to_string());
		}

		if let Some(xy) = xy {
			if Some(xy.x) != self.position_xy.map(|p| p.x) {
				command.push(format!("X{}", format_number(xy.x)));
			}

			if Some(xy.y) != self.position_xy.map(|p| p.y) {
				command.push(format!("Y{}", format_number(xy.y)));
			}

			self.position_xy = Some(xy);
		}

		if let Some(z) = z {
			if self.position_z != Some(z) {
				command.push(format!("Z{}", format_number(z)));
			}

			self.position_z = Some(z);
		}

		self.file.write_all(format!("{}\n", command.join(" ")).as_bytes())?;

		Ok(())
	}

	pub fn rapid_move_xy(&mut self, x: f64, y: f64) -> Result<()> {
		self.rapid_move(Some(x), Some(y), None)
	}

	pub fn rapid_move_xyz(&mut self, x: f64, y: f64, z: f64) -> Result<()> {
		self.rapid_move(Some(x), Some(y), Some(z))
	}

	pub fn rapid_move_z(&mut self, z: f64) -> Result<()> {
		self.rapid_move(None, None, Some(z))
	}

	pub fn arc_cut(&mut self, x: f64, y: f64, cx: f64, cy: f64) -> Result<()> {
		let xy = self.transformation.transform_point(&Point2::new(x, y));
		let cxy = self.transformation.transform_point(&Point2::new(cx, cy));
		let current_xy = self.position_xy.ok_or(anyhow!("No current XY position"))?;

		if xy == current_xy {
			return Ok(());
		}

		let mut command = Vec::new();

		if self.last_command != Some("G3".to_string()) {
			command.push("G3".to_string());

			self.last_command = Some("G3".to_string());
		}

		if xy.x != current_xy.x {
			command.push(format!("X{}", format_number(xy.x)));
		}

		if xy.y != current_xy.y {
			command.push(format!("Y{}", format_number(xy.y)));
		}

		command.push(format!("I{}", format_number(cxy.x - current_xy.x)));
		command.push(format!("J{}", format_number(cxy.y - current_xy.y)));

		if self.last_feed != Some(self.feed_rate) {
			command.push(format!("F{}", format_number(self.feed_rate)));

			self.last_feed = Some(self.feed_rate);
		}

		self.file.write_all(format!("{}\n", command.join(" ")).as_bytes())?;

		self.position_xy = Some(xy);

		Ok(())
	}

	pub fn drill(&mut self, x: f64, y: f64, depth: f64) -> Result<()> {
		self.rapid_move_xy(x, y)?;
		self.rapid_move_z(0.25)?;
		self.plunge(-depth)?;
		self.rapid_move_z(5.0)?;

		Ok(())
	}

	pub fn contour_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, depth: f64) -> Result<()> {
		let n_passes = (depth / self.depth_per_pass).ceil() as i64;

		for layer in 1..=n_passes {
			let z = -(depth * layer as f64 / n_passes as f64);
			self.rapid_move_xy(x1, y1)?;
			self.plunge(z)?;
			self.cutting_move(Some(x2), Some(y2), None)?;
			self.rapid_move_z(5.0)?;
		}

		Ok(())
	}

	pub fn circle_pocket(&mut self, cx: f64, cy: f64, diameter: f64, depth: f64) -> Result<()> {
		if diameter <= self.cutter_diameter {
			bail!("Diameter must be greater than cutter diameter");
		}

		let n_circles = (diameter / self.cutter_diameter).floor() as i64;
		let n_passes = (depth / self.depth_per_pass).ceil() as i64;
		let x_offset = (diameter / 2.0) - (self.cutter_diameter * n_circles as f64 / 2.0);

		self.rapid_move_xy(cx + x_offset, cy)?;
		self.plunge(2.5)?;

		for i in 1..=n_passes {
			self.plunge(-(depth * i as f64 / n_passes as f64))?;

			for j in 1..=n_circles {
				self.arc_cut(2.0 * cx - self.position_xy.unwrap().x, cy, cx, cy)?;

				if j == n_circles {
					self.arc_cut(2.0 * cx - self.position_xy.unwrap().x, cy, cx, cy)?;
				} else {
					self.arc_cut(2.0 * cx - self.position_xy.unwrap().x + self.cutter_diameter / 2.0, cy, cx + self.cutter_diameter / 4.0, cy)?;
				}
			}

			if i < n_passes {
				self.cutting_move(Some(cx + x_offset), Some(cy), None)?;
			}
		}

		self.rapid_move_z(5.0)?;

		Ok(())
	}

	pub fn finish(&mut self) -> Result<()> {
		self.file.write(b"M02\n")?;
		Ok(())
	}

	/// Cuts a rectangular pocket with the given dimensions, and x y specifying the lower left corner.
	/// Note that this only handles narrow rectangles right now, hence the name groove.
	pub fn groove_pocket(&mut self, x: f64, y: f64, width: f64, height: f64, depth: f64) -> Result<()> {
		// Build the cutting pattern backwards
		let mut pattern = Vec::new();

		let mut c_x = x + self.cutter_diameter / 2.0;
		let mut c_y = y + self.cutter_diameter / 2.0;
		let mut c_width = width - self.cutter_diameter;
		let mut c_height = height - self.cutter_diameter;
		let n_passes = (depth / self.depth_per_pass).ceil() as i64;
		let n_loops = 1 + (((width / 2.0) - self.cutter_diameter) / self.stepover).ceil() as i64;

		for _ in 0..n_loops {
			pattern.push((c_x, c_y));
			c_x += c_width;
			pattern.push((c_x, c_y));
			c_y += c_height;
			pattern.push((c_x, c_y));
			c_x -= c_width;
			pattern.push((c_x, c_y));
			c_y -= c_height;
			pattern.push((c_x, c_y));
			c_x += self.stepover;
			c_y += self.stepover;
			c_width -= 2.0 * self.stepover;
			c_height -= 2.0 * self.stepover;
		}

		pattern.reverse();

		for layer in 1..=n_passes {
			let z = -(depth * layer as f64 / n_passes as f64);
			let (x, y) = pattern[0];

			if layer == 1 {
				self.rapid_move_xy(x, y)?;
				self.rapid_move_z(5.0)?;
				self.plunge(z)?;
			} else {
				self.rapid_move_xy(x, y)?;
				self.plunge(z)?;
			}

			for (x, y) in pattern.iter().skip(1) {
				self.cutting_move(Some(*x), Some(*y), None)?;
			}

			self.rapid_move_z(z + RETRACT)?;
		}

		self.rapid_move_z(5.0)?;

		Ok(())
	}
}


fn format_number(f: f64) -> String {
	let mut s = format!("{:.3}", f);
	let t = s.trim_end_matches('0').trim_end_matches('.').len();
	s.truncate(t);
	s
}
