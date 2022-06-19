use crate::{
	core::{
		common::*,
		context::*,
		machine::*,
		node::*,
		node_factory::*,
	},
};
use node_macro::node_impl;

use std::{
	fs::File,
	path::Path,
};
use wav::{
	bit_depth::BitDepth,
	header::Header,
	header::WAV_FORMAT_PCM,
};

pub struct WavFileOut {
	input: ChanneledNodeIndex,
	path: String,

	buffer: Vec<Sample>,
	// buffer: BitDepth,

	/// 絶対値で最も大きかったサンプル（の絶対値）
	max_abs: Sample,
	/// max_abs を記録したのがいつだったか（(1 / sample_rate) 秒を 1 と数える。ステレオでも 2 倍にはならない）
	max_at_sample: i32,
}
impl WavFileOut {
	pub fn new(input: ChanneledNodeIndex, path: String) -> Self {
		Self {
			input,
			path,
			buffer: vec![],
			max_abs: 0f32,

			max_at_sample: 0,
			// buffer: BitDepth::ThirtyTwoFloat(vec![]),
		}
	}
}
#[node_impl]
impl Node for WavFileOut {
	fn channels(&self) -> i32 { 0 }
	fn upstreams(&self) -> Upstreams { vec![self.input] }
	fn activeness(&self) -> Activeness { Activeness::Active }
	fn update(&mut self, inputs: &Vec<Sample>, _context: &Context, _env: &mut Environment) {
		for i in 0usize .. self.input.channels() as usize {
			let smp = inputs[i];
			self.buffer.push(smp);

			let smp_abs = smp.abs();
			if smp_abs > self.max_abs {
				self.max_abs = smp_abs;
				self.max_at_sample = _context.elapsed_samples();
			}
		}
	}

	fn finalize(&mut self, context: &Context, _env: &mut Environment) {
		// wav ファイルの仕様上、float のまま書き込むこともできるが（WAV_FORMAT_IEEE_FLOAT）、
		// Windows の普通のプレーヤーでは開けなかったりするので、16 ビットの整数にする
		let header = Header::new(
			WAV_FORMAT_PCM,
			self.input.channels() as u16,
			context.sample_rate() as u32,
			16u16, // TODO 可変にする
		);

		println!("max amplitude (abs): {} at sample {}", self.max_abs, self.max_at_sample);
		let norm_ratio = if self.max_abs != 0f32 { 1f32 / self.max_abs } else { 0f32 };

		// TODO ビットレートは可変にしたい
		let buf_16bit = self.buffer.iter().map(|smp| (smp * norm_ratio * 32767f32).round() as i16 ).collect();
		let write_buf = BitDepth::Sixteen(buf_16bit);

		// TODO エラー処理
		let mut out_file = File::create(Path::new(&self.path)).unwrap();
		wav::write(header, &write_buf, &mut out_file).unwrap();
	}
}

pub struct WavFileOutFactory {
	channels: i32,
	path: String,
}
impl WavFileOutFactory {
	pub fn new(channels: i32, path: String) -> Self {
		Self { channels, path }
	}
}
impl NodeFactory for WavFileOutFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![] }
	fn input_channels(&self) -> i32 { self.channels }
	fn create_node(&self, _node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		Box::new(WavFileOut::new(piped_upstream, self.path.clone()))
	}
}
