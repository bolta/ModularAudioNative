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

// TODO Mul, Sub, Div, Rem, Neg, Abs, Sgn, Sqrt, Pow, Max, Min, Limit, 
