use crate::core::{
	common::*,
	node::*,
};

pub struct Add {
	args: Vec<NodeIndex>,
}
impl Add {
	pub fn new(args: Vec<NodeIndex>) -> Self { Self { args } }
}
impl Node for Add {
	fn upstreams(&self) -> Vec<NodeIndex> { self.args.clone() }
	fn execute(&mut self, inputs: &Vec<Sample>) -> Sample {
		inputs.iter().take(self.args.len()).sum()
	}
}

