use crate::{
	// calc::sample_to_bool,
	common::*,
	core::{
		common::*,
		context::*,
		delay_buffer::*,
		machine::*,
		node::*,
		node_factory::*,
		util::*,
	},
};
use node_macro::node_impl;

  ////
 //// Delay effect

pub struct Delay {
	base_: NodeBase,
	buffer: DelayBuffer<Sample>,
	signal: MonoNodeIndex,
	time: MonoNodeIndex, // ディレイタイムは秒単位
	feedback: MonoNodeIndex,
	wet: MonoNodeIndex,
}
impl Delay {
	pub fn new(
		base: NodeBase,
		max_time: f32,
		sample_rate: i32,
		signal: MonoNodeIndex,
		time: MonoNodeIndex,
		feedback: MonoNodeIndex,
		wet: MonoNodeIndex,
	) -> Self {
		Self {
			base_: base,
			buffer: DelayBuffer::new((max_time * sample_rate as f32).ceil() as usize),
			signal,
			time,
			feedback,
			wet,
		}
	}
}
#[node_impl]
impl Node for Delay {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![self.signal.channeled(), self.time.channeled(), self.feedback.channeled(), self.wet.channeled()] }
	fn activeness(&self) -> Activeness { Activeness::Active }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut [OutputBuffer], context: &Context, _env: &mut Environment) {
		let signal = inputs[0];
		let time = inputs[1];
		let feedback = inputs[2];
		let wet = inputs[3];

		let time_sample = ((time * context.sample_rate_f32()).round() as i32).min(self.buffer.len() as i32);
		output_mono(output, signal + self.buffer[- (time_sample - 1)] * wet);
		self.buffer.push(signal + feedback * self.buffer[- (time_sample - 1)]);
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
		spec_with_default("wet", 1, 1f32),
	] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, base: NodeBase, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		Box::new(Delay::new(
			base,
			self.max_time,
			self.sample_rate,
			piped_upstream.as_mono(),
			node_args.get("time").unwrap().as_mono(),
			node_args.get("feedback").unwrap().as_mono(),
			node_args.get("wet").unwrap().as_mono(),
		))
	}
}
