use crate::core::{
	common::*,
	node::*,
};

// TODO 全体的に要整理

use portaudio as pa;

const FRAMES: u32 = 256;
const INTERLEAVED: bool = true;

const WRITE_COUNT: usize = FRAMES as usize * CHANNELS as usize;

pub struct PortAudioOut {
	input: NodeIndex,
	stream: Option<pa::Stream<pa::Blocking<pa::stream::Buffer>, pa::Output<Sample>>>,
	buffer: Vec<Sample>,
}
impl PortAudioOut {
	pub fn new(input: NodeIndex) -> Self {
		Self {
			input,
			stream: None,
			buffer: Vec::with_capacity(WRITE_COUNT),
		}
	}
}
impl Node for PortAudioOut {
	// TODO ↓これ抽象クラス的なものに括り出したい
	fn initialize(&mut self) {
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
			pa::StreamParameters::<f32>::new(output_device, CHANNELS, INTERLEAVED, latency);

		pa.is_output_format_supported(output_params, SAMPLE_RATE as f64).expect("error");

		let output_settings = pa::OutputStreamSettings::new(output_params, SAMPLE_RATE as f64, FRAMES);

		let stream = pa.open_blocking_stream(output_settings).expect("error");
		self.stream = Some(stream);

		match &mut self.stream {
			None => { }
			Some(stream) => stream.start().expect("error")
		}
	}

	fn upstreams(&self) -> Vec<NodeIndex> { vec![self.input] }

	fn execute(&mut self, _inputs: &Vec<Sample>) -> Sample {
		if self.buffer.len() < WRITE_COUNT { return NO_OUTPUT; }

		let b = &mut self.buffer;

		match &mut self.stream {
			None => { }
			Some(stream) => {
				stream.write(FRAMES as u32, |output| {
					for (i, sample) in b.iter().enumerate() {
						output[i] = 0.5 * sample;
					};
				}).expect("error");
			}
		}
	
		NO_OUTPUT
	}

	fn update(&mut self, inputs: &Vec<Sample>) {
		if self.buffer.len() >= WRITE_COUNT { self.buffer.clear(); }
		self.buffer.push(inputs[0]);
	}

	fn finalize(&mut self) {
		match &mut self.stream {
			None => { }
			Some(stream) => {
				stream.stop();
				stream.close();
			}
		}
		self.stream = None;
	}
}
