use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
	node_factory::*,
};
use node_macro::node_impl;

 ////
//// Quant Rate Crusher

pub struct QuantCrush {
	signal: MonoNodeIndex,
	resolution: MonoNodeIndex,
	min: MonoNodeIndex,
	max: MonoNodeIndex,
}
impl QuantCrush {
	pub fn new(signal: MonoNodeIndex, resolution: MonoNodeIndex, min: MonoNodeIndex, max: MonoNodeIndex) -> Self {
		Self { signal, resolution, min, max }
	}
}
#[node_impl]
impl Node for QuantCrush {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![
		self.signal.channeled(),
		self.resolution.channeled(),
		self.min.channeled(),
		self.max.channeled(),
	] }
	fn activeness(&self) -> Activeness { Activeness::Passive }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut [Sample], _context: &Context, _env: &mut Environment) {
		let signal = inputs[0];
		let resolution = inputs[1];
		let min = inputs[2];
		let max = inputs[3];
		let result = if min == max {
			0f32
		} else if signal < min {
			min
		} else if signal > max {
			max
		} else {
			(resolution * (signal - min) / (max - min)).floor() / resolution * (max - min) + min
		};

		output_mono(output, result);
	}
}

pub struct QuantCrushFactory { }
impl NodeFactory for QuantCrushFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![
		spec("resolution", 1),
		spec_with_default("min", 1, -1f32),
		spec_with_default("max", 1, 1f32),
	] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		let signal = piped_upstream.as_mono();
		let resolution = node_args.get("resolution").unwrap().as_mono(); 
		let min = node_args.get("min").unwrap().as_mono(); 
		let max = node_args.get("max").unwrap().as_mono(); 
		Box::new(QuantCrush::new(signal, resolution, min, max))
	}
}

 ////
//// Sample Rate Crusher

pub struct SampleCrush {
	signal: MonoNodeIndex,
	sample_rate: MonoNodeIndex,
	accum: f32,
	out_value: f32,
}
impl SampleCrush {
	pub fn new(signal: MonoNodeIndex, sample_rate: MonoNodeIndex) -> Self {
		Self { signal, sample_rate, accum: 0f32, out_value: 0f32 }
	}
}
#[node_impl]
impl Node for SampleCrush {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![
		self.signal.channeled(),
		self.sample_rate.channeled(),
	] }
	fn activeness(&self) -> Activeness { Activeness::Passive }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut [Sample], context: &Context, _env: &mut Environment) {
		let signal = inputs[0];
		let sample_rate = inputs[1];

		if self.accum >= context.sample_rate_f32() {
			self.accum %= context.sample_rate_f32();
			self.out_value = signal;
		}
		output_mono(output, self.out_value);
		self.accum += sample_rate;
	}
}

pub struct SampleCrushFactory {
	default_sample_rate: i32,
}
impl SampleCrushFactory {
	pub fn new(default_sample_rate: i32) -> Self {
		Self { default_sample_rate }
	} 
}
impl NodeFactory for SampleCrushFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![
		spec_with_default("sampleRate", 1, self.default_sample_rate as f32),
	] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		let signal = piped_upstream.as_mono();
		let sample_rate = node_args.get("sampleRate").unwrap().as_mono(); 
		Box::new(SampleCrush::new(signal, sample_rate))
	}
}
