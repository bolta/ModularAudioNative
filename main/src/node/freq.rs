use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
	node_factory::*,
};
use node_macro::node_impl;

 ////
//// NES Freq Simulator

pub struct NesFreq {
	freq: MonoNodeIndex,

	/// 三角波チャンネルとして計算するか。
	/// 三角波チャンネルでは 1 オクターブ高いものとして計算する（結果、より音痴になる）
	/// これも MonoNodeIndex でいいのだろうか…？
	triangle: bool,
}
impl NesFreq {
	pub fn new(freq: MonoNodeIndex, triangle: bool) -> Self {
		Self { freq, triangle }
	}
}

#[node_impl]
impl Node for NesFreq {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![
		self.freq.channeled(),
	] }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, _context: &Context, _env: &mut Environment) {
		let freq = inputs[0];
		// 周波数を一旦 2A03 のレジスタの値に変換し、また周波数に戻すことで、周波数分解能を 2A03 相当にする
		// https://wikiwiki.jp/mck/周波数とレジスタの関係
		let ratio = if self.triangle { 2f32 } else { 1f32 };
		// var r = Math.Min(Math.Max(0, Math.Round(1789772.7272f / f / 16)), 2047);
		let register = 0f32.max((1789772.7272f32 / (freq * ratio) / 16f32).round()).min(2047f32);
		let result = 1789772.7272f32 / register / 16f32 / ratio;

		output_mono(output, result);
	}
}

pub struct NesFreqFactory {
	triangle: bool,
}
impl NesFreqFactory {
	pub fn new(triangle: bool) -> Self { Self { triangle } }
}
impl NodeFactory for NesFreqFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, _node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		let freq = piped_upstream.as_mono();
		Box::new(NesFreq::new(freq, self.triangle))
	}
}
