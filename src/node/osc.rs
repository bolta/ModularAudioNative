use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
};

pub struct SineOsc {
	freq: NodeIndex,

	phase: f32,
}
impl SineOsc {
	pub fn new(freq: NodeIndex) -> Self { Self { freq, phase: 0f32 } }
}
impl Node for SineOsc {
	fn initialize(&mut self, context: &Context, env: &mut Environment) { self.phase = 0f32; }
	fn upstreams(&self) -> Upstreams { vec![(self.freq, 1)] }
	fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut Vec<Sample>, context: &Context, env: &mut Environment) {
		output_mono(output, self.phase.sin());
	}
	fn update(&mut self, inputs: &Vec<Sample>, context: &Context, env: &mut Environment) {
		let freq = inputs[0];
		self.phase = (self.phase + TWO_PI * freq / context.sample_rate_f32()) % TWO_PI;
	}
}

pub struct StereoTestOsc {
	freq: NodeIndex,

	phase_l: f32,
	phase_r: f32,
}
impl StereoTestOsc {
	pub fn new(freq: NodeIndex) -> Self { Self { freq, phase_l: 0f32, phase_r: 0f32 } }
}
impl Node for StereoTestOsc {
	fn channels(&self) -> i32 { 2 }
	fn initialize(&mut self, context: &Context, env: &mut Environment) { /* self.phase = 0f32; */ }
	fn upstreams(&self) -> Upstreams { vec![(self.freq, 1)] }
	fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut Vec<Sample>, context: &Context, env: &mut Environment) {
		output_stereo(output, 0.25f32 * self.phase_l.sin(), 0.5f32 * self.phase_r.sin());
	}
	fn update(&mut self, inputs: &Vec<Sample>, context: &Context, env: &mut Environment) {
		let freq = inputs[0];
		self.phase_l = (self.phase_l + TWO_PI * freq         / context.sample_rate_f32()) % TWO_PI;
		self.phase_r = (self.phase_r + TWO_PI * freq / 2_f32 / context.sample_rate_f32()) % TWO_PI;
	}
}
