use std::{
	collections::HashMap,
	fs::File,
	io::{BufWriter, Write},
	path::Path,
};

use anyhow::{bail, Context, Result};
use nalgebra::{Matrix3, Point2};

const RETRACT: f64 = 0.25;

pub struct GcodeState {
	pub stepover: f64,
	pub depth_per_pass: f64,
	pub feed_rate: f64,
	pub plunge_rate: f64,
	pub cutter_diameter: f64,

	pub transformation: Matrix3<f64>,

	program: Vec<GCode>,
}

impl GcodeState {
	pub fn new() -> GcodeState {
		GcodeState {
			stepover: 0.0,
			depth_per_pass: 0.0,
			feed_rate: 0.0,
			plunge_rate: 0.0,
			cutter_diameter: 0.0,

			transformation: Matrix3::identity(),

			program: Vec::new(),
		}
	}

	pub fn write_header(&mut self) {
		self.program.push(GCode::AbsoluteDistanceMode);
		self.program.push(GCode::MetricUnits);
		self.program.push(GCode::Comment("Move to safe Z".to_string()));
		self.program.push(GCode::MoveInAbsoluteCoordinates(Box::new(GCode::RapidMove {
			x: None,
			y: None,
			z: Some(-5.0),
		})));
		self.program.push(GCode::SpindleStop);
	}

	pub fn set_rpm(&mut self, rpm: f64) {
		self.program.push(GCode::SpindleOnCW { rpm });
	}

	pub fn write_comment(&mut self, comment: &str) {
		self.program.push(GCode::Comment(comment.to_string()));
	}

	pub fn cutting_move(&mut self, x: f64, y: f64, z: Option<f64>) {
		let xy = Point2::new(x, y);
		let xy = self.transformation.transform_point(&xy);

		self.program.push(GCode::LinearMove {
			x: Some(xy.x),
			y: Some(xy.y),
			z: z,
			feed: self.feed_rate,
		});
	}

	pub fn plunge(&mut self, z: f64) {
		self.program.push(GCode::LinearMove {
			x: None,
			y: None,
			z: Some(z),
			feed: self.plunge_rate,
		});
	}

	pub fn rapid_move(&mut self, x: f64, y: f64, z: Option<f64>) {
		let xy = Point2::new(x, y);
		let xy = self.transformation.transform_point(&xy);

		self.program.push(GCode::RapidMove {
			x: Some(xy.x),
			y: Some(xy.y),
			z: z,
		});
	}

	pub fn rapid_move_xy(&mut self, x: f64, y: f64) {
		self.rapid_move(x, y, None)
	}

	pub fn arc_cut(&mut self, x: f64, y: f64, cx: f64, cy: f64) {
		let xy = self.transformation.transform_point(&Point2::new(x, y));
		let cxy = self.transformation.transform_point(&Point2::new(cx, cy));

		self.program.push(GCode::CounterClockwiseArc {
			x: xy.x,
			y: xy.y,
			cx: cxy.x,
			cy: cxy.y,
			feed: self.feed_rate,
		});
	}

	pub fn drill(&mut self, x: f64, y: f64, depth: f64) {
		self.rapid_move_xy(x, y);
		self.rapid_move(x, y, Some(0.25));
		self.plunge(-depth);
		self.rapid_move(x, y, Some(5.0));
	}

	pub fn contour_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, depth: f64) {
		let n_passes = (depth / self.depth_per_pass).ceil() as i64;

		for layer in 1..=n_passes {
			let z = -(depth * layer as f64 / n_passes as f64);
			self.rapid_move_xy(x1, y1);
			self.plunge(z);
			self.cutting_move(x2, y2, None);
			self.rapid_move(x2, y2, Some(5.0));
		}
	}

	pub fn circle_pocket(&mut self, cx: f64, cy: f64, diameter: f64, depth: f64) -> Result<()> {
		if diameter <= self.cutter_diameter {
			bail!("Diameter must be greater than cutter diameter");
		}

		let n_circles = (diameter / self.cutter_diameter).floor() as i64;
		let n_passes = (depth / self.depth_per_pass).ceil() as i64;
		let x_offset = (diameter / 2.0) - (self.cutter_diameter * n_circles as f64 / 2.0);

		self.rapid_move_xy(cx + x_offset, cy);
		self.plunge(2.5);

		for i in 1..=n_passes {
			self.plunge(-(depth * i as f64 / n_passes as f64));

			for j in 1..=n_circles {
				self.arc_cut(cx - x_offset - self.cutter_diameter * (j - 1) as f64 / 2.0, cy, cx, cy);

				if j == n_circles {
					self.arc_cut(cx + x_offset + self.cutter_diameter * (j - 1) as f64 / 2.0, cy, cx, cy);
				} else {
					self.arc_cut(cx + x_offset + self.cutter_diameter * j as f64 / 2.0, cy, cx + self.cutter_diameter / 4.0, cy);
				}
			}

			if i < n_passes {
				self.cutting_move(cx + x_offset, cy, None);
			}
		}

		self.rapid_move(cx + x_offset + self.cutter_diameter * (n_circles - 1) as f64 / 2.0, cy, Some(5.0));

		Ok(())
	}

	pub fn finish<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
		self.program.push(GCode::ProgramEnd);
		self.write_program(path)
	}

	fn write_program<P: AsRef<Path>>(&self, path: P) -> Result<()> {
		let mut file = File::create(path.as_ref()).with_context(|| format!("Failed to create file {}", path.as_ref().display()))?;
		let mut writer = BufWriter::new(&mut file);
		let mut last_command = None;
		let mut state = HashMap::new();

		for line in &self.program {
			if let GCode::Comment(comment) = &line {
				writer.write_all(format!("({})\n", comment).as_bytes())?;
				continue;
			}
			let words = line.to_words(state.get(&'X').cloned(), state.get(&'Y').cloned())?;
			let mut pieces = Vec::new();
			let mut g53 = false;

			for word in &words {
				match word {
					GcodeWord::G(g) => {
						if *g == 53 {
							g53 = true;
							last_command = None;
						}

						if last_command != Some(*word) {
							pieces.push(*word);
						}
					},
					GcodeWord::M(_) => {
						if last_command != Some(*word) {
							pieces.push(*word);
						}
					},
					GcodeWord::X(v) | GcodeWord::Y(v) | GcodeWord::Z(v) | GcodeWord::I(v) | GcodeWord::J(v) | GcodeWord::F(v) | GcodeWord::S(v) => {
						if g53 {
							pieces.push(*word);
						} else if state.get(&word.to_char()) != Some(&v) {
							pieces.push(word.clone());
						}
					},
				}
			}

			// If the command is completely empty or the line does nothing, skip it
			if pieces.is_empty() || line.is_empty(&pieces) {
				continue;
			}

			writer.write_all(pieces.iter().map(|w| w.to_string()).collect::<Vec<String>>().join(" ").as_bytes())?;
			writer.write_all(b"\n")?;

			// Update state based on the command as written
			for word in pieces {
				match word {
					GcodeWord::G(_) | GcodeWord::M(_) => {
						if !g53 {
							last_command = Some(word)
						}
					},
					GcodeWord::X(v) | GcodeWord::Y(v) | GcodeWord::Z(v) | GcodeWord::I(v) | GcodeWord::J(v) | GcodeWord::F(v) | GcodeWord::S(v) => {
						if !g53 {
							state.insert(word.to_char(), v);
						} else {
							// Since we don't know the machine coordinate system, we have to nuke the state of any modified positions
							state.remove(&word.to_char());
						}
					},
				}
			}
		}

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
				self.rapid_move_xy(x, y);
				self.rapid_move(x, y, Some(5.0));
				self.plunge(z);
			} else {
				self.rapid_move_xy(x, y);
				self.plunge(z);
			}

			for (x, y) in pattern.iter().skip(1) {
				self.cutting_move(*x, *y, None);
			}

			if layer == n_passes {
				self.rapid_move(x, y, Some(5.0));
			} else {
				self.rapid_move(x, y, Some(z + RETRACT));
			}
		}

		Ok(())
	}
}


fn format_number(f: f64) -> String {
	let mut s = format!("{:.3}", f);
	let t = s.trim_end_matches('0').trim_end_matches('.').len();
	s.truncate(t);
	s
}


#[derive(PartialEq, Clone, Debug)]
enum GCode {
	Comment(String),
	RapidMove {
		x: Option<f64>,
		y: Option<f64>,
		z: Option<f64>,
	}, // G0
	LinearMove {
		x: Option<f64>,
		y: Option<f64>,
		z: Option<f64>,
		feed: f64,
	}, // G1
	CounterClockwiseArc {
		x: f64,
		y: f64,
		cx: f64,
		cy: f64,
		feed: f64,
	}, // G3
	MetricUnits,                          // G21
	MoveInAbsoluteCoordinates(Box<Self>), // G53
	AbsoluteDistanceMode,                 // G90

	ProgramEnd, // M02
	SpindleOnCW {
		rpm: f64,
	}, // M03
	SpindleStop, // M05
}

#[derive(PartialEq, Clone, Debug, Copy)]
enum GcodeWord {
	G(u8),
	M(u8),
	F(f64),
	I(f64),
	J(f64),
	S(f64),
	X(f64),
	Y(f64),
	Z(f64),
}

impl GCode {
	fn to_words(&self, current_x: Option<f64>, current_y: Option<f64>) -> Result<Vec<GcodeWord>> {
		Ok(match self {
			GCode::RapidMove { x, y, z } => vec![
				Some(GcodeWord::G(0)),
				x.map(|x| GcodeWord::X(x)),
				y.map(|y| GcodeWord::Y(y)),
				z.map(|z| GcodeWord::Z(z)),
			]
			.into_iter()
			.filter_map(|x| x)
			.collect(),
			GCode::LinearMove { x, y, z, feed } => vec![
				Some(GcodeWord::G(1)),
				x.map(|x| GcodeWord::X(x)),
				y.map(|y| GcodeWord::Y(y)),
				z.map(|z| GcodeWord::Z(z)),
				Some(GcodeWord::F(*feed)),
			]
			.into_iter()
			.filter_map(|x| x)
			.collect(),
			GCode::CounterClockwiseArc { x, y, cx, cy, feed } => {
				if let (Some(current_x), Some(current_y)) = (current_x, current_y) {
					vec![
						Some(GcodeWord::G(3)),
						Some(GcodeWord::X(*x)),
						Some(GcodeWord::Y(*y)),
						Some(GcodeWord::I(*cx - current_x)),
						Some(GcodeWord::J(*cy - current_y)),
						Some(GcodeWord::F(*feed)),
					]
					.into_iter()
					.filter_map(|x| x)
					.collect()
				} else {
					bail!("Cannot generate G3 arc without current position");
				}
			},
			GCode::MetricUnits => vec![GcodeWord::G(21)],
			GCode::MoveInAbsoluteCoordinates(gcode) => {
				let mut words = gcode.to_words(current_x, current_y)?;
				words.insert(0, GcodeWord::G(53));
				words
			},
			GCode::AbsoluteDistanceMode => vec![GcodeWord::G(90)],
			GCode::ProgramEnd => vec![GcodeWord::M(2)],
			GCode::SpindleOnCW { rpm } => vec![GcodeWord::M(3), GcodeWord::S(*rpm)],
			GCode::SpindleStop => vec![GcodeWord::M(5)],
			GCode::Comment(_) => unreachable!(),
		})
	}

	fn is_empty(&self, words: &[GcodeWord]) -> bool {
		let pos_present = words.iter().any(|w| match w {
			GcodeWord::X(_) | GcodeWord::Y(_) | GcodeWord::Z(_) => true,
			_ => false,
		});

		let s_present = words.iter().any(|w| match w {
			GcodeWord::S(_) => true,
			_ => false,
		});

		match self {
			GCode::Comment(_) => unreachable!(),
			GCode::RapidMove { x: _, y: _, z: _ } => !pos_present,
			GCode::LinearMove { x: _, y: _, z: _, feed: _ } => !pos_present,
			GCode::CounterClockwiseArc {
				x: _,
				y: _,
				cx: _,
				cy: _,
				feed: _,
			} => !pos_present,
			GCode::MetricUnits | GCode::AbsoluteDistanceMode | GCode::ProgramEnd | GCode::SpindleStop | GCode::MoveInAbsoluteCoordinates(_) => false,
			GCode::SpindleOnCW { rpm: _ } => !s_present,
		}
	}
}

impl ToString for GcodeWord {
	fn to_string(&self) -> String {
		match self {
			GcodeWord::G(n) => format!("G{}", n),
			GcodeWord::M(n) => format!("M{:02}", n),
			GcodeWord::F(n) => format!("F{}", format_number(*n)),
			GcodeWord::I(n) => format!("I{}", format_number(*n)),
			GcodeWord::J(n) => format!("J{}", format_number(*n)),
			GcodeWord::S(n) => format!("S{}", format_number(*n)),
			GcodeWord::X(n) => format!("X{}", format_number(*n)),
			GcodeWord::Y(n) => format!("Y{}", format_number(*n)),
			GcodeWord::Z(n) => format!("Z{}", format_number(*n)),
		}
	}
}

impl GcodeWord {
	fn to_char(&self) -> char {
		match self {
			GcodeWord::G(_) => 'G',
			GcodeWord::M(_) => 'M',
			GcodeWord::F(_) => 'F',
			GcodeWord::I(_) => 'I',
			GcodeWord::J(_) => 'J',
			GcodeWord::S(_) => 'S',
			GcodeWord::X(_) => 'X',
			GcodeWord::Y(_) => 'Y',
			GcodeWord::Z(_) => 'Z',
		}
	}
}
