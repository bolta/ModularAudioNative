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
	// TODO ステレオ対応
	fn upstreams(&self) -> Upstreams { vec![(self.input, 1)] }
	fn execute<'a>(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, context: &Context, env: &mut Environment) {
		println!("{}", inputs[0]);
	}
}
