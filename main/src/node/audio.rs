use crate::common::util::ignore_errors;

use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
};
use node_macro::node_impl;

// TODO 全体的に要整理

use portaudio as pa;

const FRAMES: u32 = 1000;
const INTERLEAVED: bool = true;

// #4 マルチマシン対応
// PortAudio の Stream は生ポインタを持っている都合で Send にならないので、
// ムリヤリ Send にするためのラッパーをかます。
// https://users.rust-lang.org/t/workaround-missing-send-trait-for-the-ffi/30828/7
// ノード生成時には初期化含めて一切触らず、別スレッドでの演奏開始時に初めて初期化するので問題はないはず
struct SendWrapper(pa::Stream<pa::Blocking<pa::stream::Buffer>, pa::Output<Sample>>);
unsafe impl Send for SendWrapper { }

pub struct PortAudioOut {
	base_: NodeBase,
	input: ChanneledNodeIndex,
	stream: Option<SendWrapper>,
	buffer: Vec<Sample>,
	buffer_size: usize,
}
impl PortAudioOut {
	pub fn new(base: NodeBase, input: ChanneledNodeIndex) -> Self {
		let channels = input.channels();
		let buffer_size = FRAMES as usize * channels as usize;

		Self {
			base_: base,
			input,
			stream: None,
			buffer: Vec::with_capacity(buffer_size),
			buffer_size,
		}
	}
}
#[node_impl]
impl Node for PortAudioOut {
	// ノードグラフ上で出力するチャンネル数は 0
	fn channels(&self) -> i32 { 0 }
	fn activeness(&self) -> Activeness { Activeness::Active } // TODO でいいのかな
	// TODO ↓これ抽象クラス的なものに括り出したい
	fn initialize(&mut self, context: &Context, _env: &mut Environment) {
		let pa = pa::PortAudio::new().expect("error");

		// let default_host = pa.default_host_api().expect("error");
		// println!("default host: {:#?}", pa.host_api_info(default_host));

		let output_device = pa.default_output_device().expect("error");

		let output_info = pa.device_info(output_device).expect("error");
		// println!("Use output device info: {:#?}", &output_info);

		// 出力の設定
		let latency = output_info.default_low_output_latency;
		// float32形式で再生
		let output_params =
			pa::StreamParameters::<f32>::new(output_device, self.input.channels(), INTERLEAVED, latency);

		let sample_rate = context.sample_rate() as f64;
		pa.is_output_format_supported(output_params, sample_rate).expect("error");

		let output_settings = pa::OutputStreamSettings::new(output_params, sample_rate as f64, FRAMES);

		let stream = pa.open_blocking_stream(output_settings).expect("error");
		self.stream = Some(SendWrapper(stream));

		match &mut self.stream {
			None => { }
			Some(stream) => stream.0.start().expect("error")
		}
	}

	fn upstreams(&self) -> Upstreams { vec![self.input] }

	fn execute(&mut self, _inputs: &Vec<Sample>, _output: &mut [OutputBuffer], _context: &Context, _env: &mut Environment) {
		if self.buffer.len() < self.buffer_size { return; }

		let b = &mut self.buffer;

		match &mut self.stream {
			None => { }
			Some(stream) => {
				stream.0.write(FRAMES as u32, |output| {
					for (i, sample) in b.iter().enumerate() {
						output[i] = 0.5 * sample;
					};
				}).expect("error");
			}
		}
	}

	fn update(&mut self, inputs: &Vec<Sample>, _context: &Context, _env: &mut Environment) {
		if self.buffer.len() >= self.buffer_size { self.buffer.clear(); }
		for ch in 0 .. self.input.channels() {
			self.buffer.push(inputs[ch as usize]);
		}
	}

	fn finalize(&mut self, _context: &Context, _env: &mut Environment) {
		match &mut self.stream {
			None => { }
			Some(stream) => {
				ignore_errors(stream.0.stop());
				ignore_errors(stream.0.close());
			}
		}
		self.stream = None;
	}
}
