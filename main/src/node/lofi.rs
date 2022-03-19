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
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, _context: &Context, _env: &mut Environment) {
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
