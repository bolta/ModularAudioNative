use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
};

pub struct Print {
	input: NodeIndex,
}
impl Print {
	pub fn new(input: NodeIndex) -> Self { Self { input } }
}
impl Node for Print {
	// TODO ↓これ抽象クラス的なものに括り出したい
	fn upstreams(&self) -> Vec<NodeIndex> { vec![self.input] }
	fn execute<'a>(&mut self, inputs: &Vec<Sample>, context: &Context, env: &mut Environment) -> Sample {
		println!("{}", inputs[0]);
		NO_OUTPUT
	}
}
