use crate::core::{
	common::*,
	context::*,
	delay_buffer::*,
	machine::*,
	node::*,
	node_factory::*,
};
use node_macro::node_impl;

  ////
 //// Delay effect

pub struct Delay {
	buffer: DelayBuffer<Sample>,
	signal: MonoNodeIndex,
	time: MonoNodeIndex, // ディレイタイムは秒単位
	feedback: MonoNodeIndex,
}
impl Delay {
	pub fn new(
		max_time: f32,
		sample_rate: i32,
		signal: MonoNodeIndex,
		time: MonoNodeIndex,
		feedback: MonoNodeIndex,
	) -> Self {
		Self {
			buffer: DelayBuffer::new((max_time * sample_rate as f32).ceil() as usize),
			signal,
			time,
			feedback,
		}
	}
}
#[node_impl]
impl Node for Delay {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![self.signal.channeled(), self.time.channeled(), self.feedback.channeled()] }
	fn activeness(&self) -> Activeness { Activeness::Active }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut [Sample], context: &Context, _env: &mut Environment) {
		let signal = inputs[0];
		let time = inputs[1];
		let feedback = inputs[2];

		let time_sample = ((time * context.sample_rate_f32()).round() as i32).min(self.buffer.len() as i32);
		let delay = self.buffer[- (time_sample - 1)];

		// 入力を含まない、バッファの値だけを出力する
		output_mono(output, delay);
		// 入力はバッファを一周してから出力されるよう、（フィードバックとともに）バッファに入れる
		self.buffer.push(signal + feedback * delay);
	}
}
pub struct DelayFactory {
	max_time: f32,
	sample_rate: i32,
}
impl DelayFactory {
	pub fn new(max_time: f32, sample_rate: i32) -> Self {
		Self { max_time, sample_rate }
	}
}
impl NodeFactory for DelayFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![
		spec_with_default("time", 1, self.max_time),
		spec_with_default("feedback", 1, 0f32),
	] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		Box::new(Delay::new(
			self.max_time,
			self.sample_rate,
			piped_upstream.as_mono(),
			node_args.get("time").unwrap().as_mono(),
			node_args.get("feedback").unwrap().as_mono(),
		))
	}
}
