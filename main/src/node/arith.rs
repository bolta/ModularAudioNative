use crate::{
	core::{
		common::*,
		context::*,
		machine::*,
		node::*,
		node_factory::*,
	},
	operator::*,
};
use node_macro::node_impl;

use std::{
	marker::PhantomData,
};

pub struct MonoBinary<Op: BinaryOp> {
	_op: PhantomData<fn () -> Op>,
	lhs: MonoNodeIndex,
	rhs: MonoNodeIndex,
}
impl <Op: BinaryOp> MonoBinary<Op> {
	pub fn new(lhs: MonoNodeIndex, rhs: MonoNodeIndex) -> Self {
		Self { _op: PhantomData, lhs, rhs }
	}
}
#[node_impl]
impl <Op: BinaryOp> Node for MonoBinary<Op> {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![self.lhs.channeled(), self.rhs.channeled()] }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, _context: &Context, _env: &mut Environment) {
		output_mono(output, Op::oper(inputs[0], inputs[1]));
	}
}

pub struct StereoBinary<Op: BinaryOp> {
	_op: PhantomData<fn () -> Op>,
	lhs: StereoNodeIndex,
	rhs: StereoNodeIndex,
}
impl <Op: BinaryOp> StereoBinary<Op> {
	pub fn new(lhs: StereoNodeIndex, rhs: StereoNodeIndex) -> Self {
		Self { _op: PhantomData, lhs, rhs }
	}
}
#[node_impl]
impl <Op: BinaryOp> Node for StereoBinary<Op> {
	fn channels(&self) -> i32 { 2 }
	fn upstreams(&self) -> Upstreams { vec![self.lhs.channeled(), self.rhs.channeled()] }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, _context: &Context, _env: &mut Environment) {
		output_stereo(output, Op::oper(inputs[0], inputs[2]), Op::oper(inputs[1], inputs[3]));
	}
}

pub type Add = MonoBinary<AddOp>;
pub type StereoAdd = StereoBinary<AddOp>;
pub type Sub = MonoBinary<SubOp>;
pub type StereoSub = StereoBinary<SubOp>;
pub type Mul = MonoBinary<MulOp>;
pub type StereoMul = StereoBinary<MulOp>;
pub type Div = MonoBinary<DivOp>;
pub type StereoDiv = StereoBinary<DivOp>;
pub type Rem = MonoBinary<RemOp>;
pub type StereoRem = StereoBinary<RemOp>;
pub type Pow = MonoBinary<PowOp>;
pub type StereoPow = StereoBinary<PowOp>;

pub struct Limit {
	signal: MonoNodeIndex,
	min: MonoNodeIndex,
	max: MonoNodeIndex,
}
impl Limit {
	pub fn new(signal: MonoNodeIndex, min: MonoNodeIndex, max: MonoNodeIndex) -> Self {
		Self { signal, min, max }
	}
}
#[node_impl]
impl Node for Limit {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![self.signal.channeled(), self.min.channeled(), self.max.channeled()] }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, _context: &Context, _env: &mut Environment) {
		let sig = inputs[0];
		let min = inputs[1];
		let max = inputs[2];

		output_mono(output, sig.max(min).min(max));
	}
}

pub struct LimitFactory { }
impl NodeFactory for LimitFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![spec("min", 1), spec("max", 1)] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		let signal = piped_upstream.as_mono();
		// ここは、存在しなければ呼び出し元でエラーにするのでチェック不要、のはず
		let min = node_args.get("min").unwrap().as_mono();
		let max = node_args.get("max").unwrap().as_mono();
		Box::new(Limit::new(signal, min, max))
	}
}

// TODO Neg, Abs, Sgn, Sqrt, Max, Min,
