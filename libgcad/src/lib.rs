mod engine;
mod gcode;
mod numbers;
mod value;

pub use engine::ScriptEngine;

pub const BUILTIN_MATERIALS: &'static str = include_str!("../materials.gcad");