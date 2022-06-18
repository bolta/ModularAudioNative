use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
};
use node_macro::node_impl;

pub struct Constant {
	value: Sample,
}
impl Constant {
	pub fn new(value: Sample) -> Self { Self { value } }
}
#[node_impl]
impl Node for Constant {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![] }
	fn activeness(&self) -> Activeness { Activeness::Static }
	fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut [Sample], _context: &Context, _env: &mut Environment) {
		output_mono(output, self.value);
	}
}
