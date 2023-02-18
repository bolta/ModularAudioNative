use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
};
use node_macro::node_impl;

pub struct Print {
	base_: NodeBase,
	input: ChanneledNodeIndex,
}
impl Print {
	pub fn new(base: NodeBase, input: ChanneledNodeIndex) -> Self { Self { base_: base,  input } }
}
#[node_impl]
impl Node for Print {
	fn channels(&self) -> i32 { 0 }
	// TODO ↓これ抽象クラス的なものに括り出したい
	// TODO ステレオ対応
	fn upstreams(&self) -> Upstreams { vec![self.input] }
	fn activeness(&self) -> Activeness { Activeness::Active }
	fn execute<'a>(&mut self, inputs: &Vec<Sample>, _output: &mut [OutputBuffer], _context: &Context, _env: &mut Environment) {
		// TODO ステレオ対応
		println!("{}", inputs[0]);
	}
}

pub struct NullOut {
	base_: NodeBase,
	input: ChanneledNodeIndex,
}
impl NullOut {
	pub fn new(base: NodeBase, input: ChanneledNodeIndex) -> Self { Self { base_: base,  input } }
}
#[node_impl]
impl Node for NullOut {
	fn channels(&self) -> i32 { 0 }
	fn upstreams(&self) -> Upstreams { vec![self.input] }
	fn activeness(&self) -> Activeness { Activeness::Static }
	fn execute<'a>(&mut self, _inputs: &Vec<Sample>, _output: &mut [OutputBuffer], _context: &Context, _env: &mut Environment) {
		// do nothing
	}
}

pub struct MemoryOut<'a> {
	base_: NodeBase,
	input: ChanneledNodeIndex,
	output: &'a mut Vec<Sample>,
}
impl <'a> MemoryOut<'a> {
	pub fn new(base: NodeBase, input: ChanneledNodeIndex, output: &'a mut Vec<Sample>) -> Self {
		Self { base_: base, input, output }
	}
}
#[node_impl]
impl <'a> Node for MemoryOut<'a> {
	fn channels(&self) -> i32 { 0 }
	fn upstreams(&self) -> Upstreams { vec![self.input] }
	fn activeness(&self) -> Activeness { Activeness::Active }
	fn execute(&mut self, inputs: &Vec<Sample>, _output: &mut [OutputBuffer], _context: &Context, _env: &mut Environment) {
		self.output.push(inputs[0]);
	}
}
