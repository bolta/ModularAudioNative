use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
};
use node_macro::node_impl;

pub struct Constant {
	base_: NodeBase,
	value: Sample,
}
impl Constant {
	pub fn new(/* base: NodeBase, */ value: Sample) -> Self { Self { base_: NodeBase::new(0),  value } }
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
