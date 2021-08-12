use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
};

pub struct Constant {
	value: Sample,
}
impl Constant {
	pub fn new(value: Sample) -> Self { Self { value } }
}
impl Node for Constant {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![] }
	fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut Vec<Sample>, _context: &Context, _env: &mut Environment) {
		output_mono(output, self.value);
	}
}
