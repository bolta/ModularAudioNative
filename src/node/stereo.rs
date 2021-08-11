use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
};

pub struct MonoToStereo {
	input: MonoNodeIndex,
}
impl MonoToStereo {
	pub fn new(input: MonoNodeIndex) -> Self { Self { input } }
}
impl Node for MonoToStereo {
	fn channels(&self) -> i32 { 2 }
	fn upstreams(&self) -> Upstreams { vec![self.input.channeled()] }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, context: &Context, env: &mut Environment) {
		output_stereo(output, inputs[0], inputs[0]);
	}
}

pub struct Split {
	input: StereoNodeIndex, 
	channel: usize,
}
impl Split {
	pub fn new(input: StereoNodeIndex, channel: i32) -> Self {
		Self { input, channel: channel as usize }
	}
}
impl Node for Split {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![self.input.channeled()] }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, context: &Context, env: &mut Environment) {
		output_mono(output, inputs[self.channel]);
	}
}

pub struct Join {
	inputs: Vec<MonoNodeIndex>,
}
impl Join {
	pub fn new(inputs: Vec<MonoNodeIndex>) -> Self { Self { inputs } }
}
impl Node for Join {
	fn channels(&self) -> i32 { self.inputs.len() as i32 }
	fn upstreams(&self) -> Upstreams { self.inputs.iter().map(|i| i.channeled()).collect() }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, context: &Context, env: &mut Environment) {
		for i in 0 .. self.inputs.len() {
			output[i] = inputs[i];
		}
	}
}
