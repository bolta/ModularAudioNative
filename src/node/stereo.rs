use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
};

pub struct MonoToStereo {
	input: MonoNodeIndex,
}
impl MonoToStereo {
	pub fn new(input: MonoNodeIndex) -> Self { Self { input } }
}
impl Node for MonoToStereo {
	fn channels(&self) -> i32 { 2 }
	fn upstreams(&self) -> Upstreams { vec![self.input.channeled()] }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, context: &Context, env: &mut Environment) {
		output_stereo(output, inputs[0], inputs[0]);
	}
}

pub struct Split {
	input: StereoNodeIndex, 
	channel: usize,
}
impl Split {
	pub fn new(input: StereoNodeIndex, channel: i32) -> Self {
		Self { input, channel: channel as usize }
	}
}
impl Node for Split {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![self.input.channeled()] }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, context: &Context, env: &mut Environment) {
		output_mono(output, inputs[self.channel]);
	}
}

pub struct Join {
	inputs: Vec<MonoNodeIndex>,
}
impl Join {
	pub fn new(inputs: Vec<MonoNodeIndex>) -> Self { Self { inputs } }
}
impl Node for Join {
	fn channels(&self) -> i32 { self.inputs.len() as i32 }
	fn upstreams(&self) -> Upstreams { self.inputs.iter().map(|i| i.channeled()).collect() }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, context: &Context, env: &mut Environment) {
		for i in 0 .. self.inputs.len() {
			output[i] = inputs[i];
		}
	}
}

pub struct Pan {
	input: MonoNodeIndex,
	pos: MonoNodeIndex,
}
impl Pan {
	pub fn new(input: MonoNodeIndex, pos: MonoNodeIndex) -> Self { Self { input, pos } }
}
impl Node for Pan {
	fn channels(&self) -> i32 { 2 }
	fn upstreams(&self) -> Upstreams { vec![self.input.channeled(), self.pos.channeled()] }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, context: &Context, env: &mut Environment) {
		let input = inputs[0];
		let pos = inputs[1].max(-1f32).min(1f32); // 外すとどうなる？

		// http://amei.or.jp/midistandardcommittee/Recommended_Practice/e/rp36.pdf
		let arg = (pos + 1f32) / 2f32 * std::f32::consts::PI / 2f32; // [-1, 1] を [0, pi/2] に変換
		// TODO 定位が変わっていないのに毎サンプル計算するのは無駄だが
		let amp_l = arg.cos();
		let amp_r = arg.sin();

		output_stereo(output, input * amp_l, input * amp_r);
	}
}
