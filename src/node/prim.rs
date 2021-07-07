use crate::core::{
	common::*,
	node::*,
};

pub struct Constant {
	value: Sample,
}
impl Constant {
	pub fn new(value: Sample) -> Self { Self { value } }
}
impl Node for Constant {
	fn upstreams(&self) -> Vec<NodeIndex> { vec![] }
	fn execute(&mut self, _inputs: &Vec<Sample>) -> Sample { self.value }
}
