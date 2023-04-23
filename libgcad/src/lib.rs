mod engine;
mod gcode;
mod numbers;
mod value;

pub use engine::ScriptEngine;

pub const BUILTIN_MATERIALS: &str = include_str!("../materials.gcad");
