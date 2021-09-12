use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
};
use node_macro::node_impl;

pub struct SineOsc {
	freq: MonoNodeIndex,

	phase: f32,
}
impl SineOsc {
	pub fn new(freq: MonoNodeIndex) -> Self { Self { freq, phase: 0f32 } }
}

#[node_impl]
impl Node for SineOsc {
	fn channels(&self) -> i32 { 1 }
	fn initialize(&mut self, _context: &Context, _env: &mut Environment) { self.phase = 0f32; }
	fn upstreams(&self) -> Upstreams { vec![self.freq.channeled()] }
	fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut Vec<Sample>, _context: &Context, _env: &mut Environment) {
		output_mono(output, self.phase.sin());
	}
	fn update(&mut self, inputs: &Vec<Sample>, context: &Context, _env: &mut Environment) {
		let freq = inputs[0];
		self.phase = (self.phase + TWO_PI * freq / context.sample_rate_f32()) % TWO_PI;
	}
}

pub struct PulseOsc {
	freq: MonoNodeIndex,
	duty: MonoNodeIndex,

	phase: f32,
}
impl PulseOsc {
	pub fn new(freq: MonoNodeIndex, duty: MonoNodeIndex) -> Self { Self { freq, duty, phase: 0f32 } }
}

#[node_impl]
impl Node for PulseOsc {
	fn channels(&self) -> i32 { 1 }
	fn initialize(&mut self, _context: &Context, _env: &mut Environment) { self.phase = 0f32; }
	fn upstreams(&self) -> Upstreams { vec![self.freq.channeled(), self.duty.channeled()] }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, _context: &Context, _env: &mut Environment) {
		let duty = inputs[1];
		output_mono(output, if self.phase % TWO_PI < TWO_PI * duty {
			1f32 
		} else {
			-1f32
		});
	}
	fn update(&mut self, inputs: &Vec<Sample>, context: &Context, _env: &mut Environment) {
		let freq = inputs[0];
		self.phase = (self.phase + TWO_PI * freq / context.sample_rate_f32()) % TWO_PI;
	}
}

pub struct StereoTestOsc {
	freq: MonoNodeIndex,

	phase_l: f32,
	phase_r: f32,
}
impl StereoTestOsc {
	pub fn new(freq: MonoNodeIndex) -> Self { Self { freq, phase_l: 0f32, phase_r: 0f32 } }
}
#[node_impl]
impl Node for StereoTestOsc {
	fn channels(&self) -> i32 { 2 }
	fn initialize(&mut self, _context: &Context, _env: &mut Environment) { }
	fn upstreams(&self) -> Upstreams { vec![self.freq.channeled()] }
	fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut Vec<Sample>, _context: &Context, _env: &mut Environment) {
		output_stereo(output, self.phase_l.sin(), self.phase_r.sin());
	}
	fn update(&mut self, inputs: &Vec<Sample>, context: &Context, _env: &mut Environment) {
		let freq = inputs[0];
		self.phase_l = (self.phase_l + TWO_PI * freq         / context.sample_rate_f32()) % TWO_PI;
		self.phase_r = (self.phase_r + TWO_PI * freq / 2_f32 / context.sample_rate_f32()) % TWO_PI;
	}
}
