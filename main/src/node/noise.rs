use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
	node_factory::*,
};
use node_macro::node_impl;

use rand::prelude::*;

 ////
//// Uniform Noise

/**
 * 一様乱数によるノイズジェネレータ
 */
pub struct UniformNoise {
	gen: ThreadRng,
}
impl UniformNoise {
	pub fn new() -> Self {
		Self { gen: rand::thread_rng() }
	}
}
#[node_impl]
impl Node for UniformNoise {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![
	] }
	fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut [Sample], _context: &Context, _env: &mut Environment) {
		output_mono(output, 2f32 * self.gen.gen::<f32>() - 1f32);
	}
}

pub struct UniformNoiseFactory { }
impl NodeFactory for UniformNoiseFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, _node_args: &NodeArgs, _piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		Box::new(UniformNoise::new())
	}
}
