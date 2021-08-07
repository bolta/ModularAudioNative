use crate::core::{
	common::*,
	context::*,
	machine::*,
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
	fn execute(&mut self, inputs: &Vec<Sample>, context: &Context, env: &mut Environment) -> Sample {
		inputs.iter().take(self.args.len()).sum()
	}
}

pub struct Mul {
	args: Vec<NodeIndex>,
}
impl Mul {
	pub fn new(args: Vec<NodeIndex>) -> Self { Self { args } }
}
impl Node for Mul {
	fn upstreams(&self) -> Vec<NodeIndex> { self.args.clone() }
	fn execute(&mut self, inputs: &Vec<Sample>, context: &Context, env: &mut Environment) -> Sample {
		inputs.iter().take(self.args.len()).product()
	}
}

pub struct Limit {
	signal: NodeIndex,
	min: NodeIndex,
	max: NodeIndex,
}
impl Limit {
	pub fn new(signal: NodeIndex, min: NodeIndex, max: NodeIndex) -> Self {
		Self { signal, min, max }
	}
}
impl Node for Limit {
	fn upstreams(&self) -> Vec<NodeIndex> { vec![self.signal, self.min, self.max] }
	fn execute(&mut self, inputs: &Vec<Sample>, context: &Context, env: &mut Environment) -> Sample {
		let sig = inputs[0];
		let min = inputs[1];
		let max = inputs[2];

		sig.max(min).min(max)
	}
}

// TODO Mul, Sub, Div, Rem, Neg, Abs, Sgn, Sqrt, Pow, Max, Min, Limit, 
