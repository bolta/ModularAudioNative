use crate::core::{
	common::*,
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
	fn initialize(&mut self) { self.phase = 0f32; }
	fn upstreams(&self) -> Vec<NodeIndex> { vec![self.freq] }
	fn execute(&mut self, _inputs: &Vec<Sample>, machine: &mut Machine) -> Sample { self.phase.sin() }
	fn update(&mut self, inputs: &Vec<Sample>) {
		let freq = inputs[0];
		self.phase = (self.phase + TWO_PI * freq / SAMPLE_RATE_F32) % TWO_PI;
	}
}
