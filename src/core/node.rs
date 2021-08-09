extern crate portaudio;
use super::{
	common::*,
	context::*,
	event::*,
	machine::*,
};

pub type Upstreams = Vec<(NodeIndex, i32)>;

pub trait Node {
	fn channels(&self) -> i32 { 1 }
	// fn upstreams(&self) -> Vec<(NodeIndex, i32)> { vec![] }
	fn upstreams(&self) -> Upstreams { vec![] }
	fn initialize(&mut self, context: &Context, env: &mut Environment) { }
	// fn execute(&mut self, inputs: &Vec<Sample>, context: &Context, env: &mut Environment) -> Sample { NO_OUTPUT }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, context: &Context, env: &mut Environment) { }
	fn update(&mut self, _inputs: &Vec<Sample>, context: &Context, env: &mut Environment) { }
	fn finalize(&mut self, context: &Context, env: &mut Environment) { }
	fn process_event(&mut self, event: &dyn Event, context: &Context, env: &mut Environment) { }
}

pub fn output_mono(output: &mut Vec<Sample>, value: Sample) {
	output[0] = value;
}
pub fn output_stereo(output: &mut Vec<Sample>, value_l: Sample, value_r: Sample) {
	output[0] = value_l;
	output[1] = value_r;
}
