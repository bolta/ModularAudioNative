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
