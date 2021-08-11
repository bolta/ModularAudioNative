use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
};

use std::{
	marker::PhantomData,
};

mod op {
	use crate::core::common::Sample;
	pub trait BinaryOp {
		fn oper(lhs: Sample, rhs: Sample) -> Sample;
	}

	// TODO マクロでやろうとしたがうまくいかない
	// macro_rules! op {
	// 	($name: ident, $def: block) => {
	// 		pub struct $name { }
	// 		impl BinaryOp for $name { $def }
	// 	}
	// }
	// op!(AddOp, { fn oper(lhs: Sample, rhs: Sample) -> Sample { lhs + rhs } });
	pub struct AddOp { }
	impl BinaryOp for AddOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { lhs + rhs } }
	pub struct SubOp { }
	impl BinaryOp for SubOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { lhs - rhs } }
	pub struct MulOp { }
	impl BinaryOp for MulOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { lhs * rhs } }
	pub struct DivOp { }
	impl BinaryOp for DivOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { lhs / rhs } }
	pub struct RemOp { }
	impl BinaryOp for RemOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { lhs % rhs } }
	pub struct PowOp { }
	impl BinaryOp for PowOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { lhs.powf(rhs) } }
}
use self::op::*;

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
impl <Op: BinaryOp> Node for MonoBinary<Op> {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![self.lhs.channeled(), self.rhs.channeled()] }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, context: &Context, env: &mut Environment) {
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
impl <Op: BinaryOp> Node for StereoBinary<Op> {
	fn channels(&self) -> i32 { 2 }
	fn upstreams(&self) -> Upstreams { vec![self.lhs.channeled(), self.rhs.channeled()] }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, context: &Context, env: &mut Environment) {
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
impl Node for Limit {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![self.signal.channeled(), self.min.channeled(), self.max.channeled()] }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, context: &Context, env: &mut Environment) {
		let sig = inputs[0];
		let min = inputs[1];
		let max = inputs[2];

		output_mono(output, sig.max(min).min(max));
	}
}

// TODO Neg, Abs, Sgn, Sqrt, Max, Min, Limit, 
