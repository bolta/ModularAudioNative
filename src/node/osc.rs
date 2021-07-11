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
	fn upstreams(&self) -> Vec<NodeIndex> { vec![self.freq] }
	fn execute(&mut self, _inputs: &Vec<Sample>, context: &Context, env: &mut Environment) -> Sample { self.phase.sin() }
	fn update(&mut self, inputs: &Vec<Sample>, context: &Context, env: &mut Environment) {
		let freq = inputs[0];
		self.phase = (self.phase + TWO_PI * freq / context.sample_rate_f32()) % TWO_PI;
	}
}
