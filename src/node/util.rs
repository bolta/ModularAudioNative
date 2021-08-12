use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
};

pub struct Print {
	input: ChanneledNodeIndex,
}
impl Print {
	pub fn new(input: ChanneledNodeIndex) -> Self { Self { input } }
}
impl Node for Print {
	fn channels(&self) -> i32 { 1 }
	// TODO ↓これ抽象クラス的なものに括り出したい
	// TODO ステレオ対応
	fn upstreams(&self) -> Upstreams { vec![self.input] }
	fn execute<'a>(&mut self, inputs: &Vec<Sample>, _output: &mut Vec<Sample>, _context: &Context, _env: &mut Environment) {
		// TODO ステレオ対応
		println!("{}", inputs[0]);
	}
}
