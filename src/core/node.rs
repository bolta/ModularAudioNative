extern crate portaudio;
use super::{
	common::*,
	context::*,
	event::*,
	machine::*,
};

pub type Upstreams = Vec<ChanneledNodeIndex>;

pub trait Node {
	fn channels(&self) -> i32;
	fn upstreams(&self) -> Upstreams { vec![] }
	fn initialize(&mut self, _context: &Context, _env: &mut Environment) { }
	fn execute(&mut self, _inputs: &Vec<Sample>, _output: &mut Vec<Sample>, _context: &Context, _env: &mut Environment) { }
	fn update(&mut self, _inputs: &Vec<Sample>, _context: &Context, _env: &mut Environment) { }
	fn finalize(&mut self, _context: &Context, _env: &mut Environment) { }
	fn process_event(&mut self, _event: &dyn Event, _context: &Context, _env: &mut Environment) { }
}

pub fn output_mono(output: &mut Vec<Sample>, value: Sample) {
	output[0] = value;
}
pub fn output_stereo(output: &mut Vec<Sample>, value_l: Sample, value_r: Sample) {
	output[0] = value_l;
	output[1] = value_r;
}
