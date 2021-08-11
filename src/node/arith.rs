use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
};

pub struct Add {
	lhs: MonoNodeIndex,
	rhs: MonoNodeIndex,
}
impl Add {
	pub fn new(lhs: MonoNodeIndex, rhs: MonoNodeIndex) -> Self { Self { lhs, rhs } }
}
impl Node for Add {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![self.lhs.channeled(), self.rhs.channeled()] }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, context: &Context, env: &mut Environment) {
		output_mono(output, inputs[0] + inputs[1]);
	}
}

pub struct StereoAdd {
	// とりあえず 2 項以外の場合は使っていないので 2 項で（速度的にも有利なはず）
	lhs: StereoNodeIndex,
	rhs: StereoNodeIndex,
}
impl StereoAdd {
	pub fn new(lhs: StereoNodeIndex, rhs: StereoNodeIndex) -> Self { Self { lhs, rhs } }
}
impl Node for StereoAdd {
	fn channels(&self) -> i32 { 2 }
	fn upstreams(&self) -> Upstreams { vec![self.lhs.channeled(), self.rhs.channeled()] }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, context: &Context, env: &mut Environment) {
		output_stereo(output, inputs[0] + inputs[2], inputs[1] + inputs[3]);
	}
}

pub struct Mul {
	lhs: MonoNodeIndex,
	rhs: MonoNodeIndex,
}
impl Mul {
	pub fn new(lhs: MonoNodeIndex, rhs: MonoNodeIndex) -> Self { Self { lhs, rhs } }
}
impl Node for Mul {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![self.lhs.channeled(), self.rhs.channeled()] }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, context: &Context, env: &mut Environment) {
		output_mono(output, inputs[0] * inputs[1]);
	}
}

// TODO ちゃんと共通化
pub struct StereoMul {
	// とりあえず 2 項以外の場合は使っていないので 2 項で（速度的にも有利なはず）
	lhs: StereoNodeIndex,
	rhs: StereoNodeIndex,
}
impl StereoMul {
	pub fn new(lhs: StereoNodeIndex, rhs: StereoNodeIndex) -> Self { Self { lhs, rhs } }
}
impl Node for StereoMul {
	fn channels(&self) -> i32 { 2 }
	fn upstreams(&self) -> Upstreams { vec![self.lhs.channeled(), self.rhs.channeled()] }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, context: &Context, env: &mut Environment) {
		output_stereo(output, inputs[0] * inputs[2], inputs[1] * inputs[3]);
	}
}

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

// TODO Mul, Sub, Div, Rem, Neg, Abs, Sgn, Sqrt, Pow, Max, Min, Limit, 
