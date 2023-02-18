use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
	node_factory::*,
};
use node_macro::node_impl;

 ////////
//////// 値の遷移に関するノードの置き場 

 ////
//// Glide

pub struct Glide {
	base_: NodeBase,
	signal: MonoNodeIndex,
	/**
	 * 半減期：目標値が与えられ続けたとき、現在値と目標値の中間まで達するのにかかる時間（秒）
	 */
	halflife: MonoNodeIndex,
	actual: Option<f32>,
}
impl Glide {
	pub fn new(base: NodeBase, signal: MonoNodeIndex, halflife: MonoNodeIndex) -> Self {
		Self { base_: base, signal, halflife, actual: None }
	}
}
#[node_impl]
impl Node for Glide {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![
		self.signal.channeled(),
		self.halflife.channeled(),
	] }
	fn activeness(&self) -> Activeness { Activeness::Active }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut [OutputBuffer], context: &Context, _env: &mut Environment) {
		let signal = inputs[0];
		let halflife = inputs[1];
		// 			var ratioPerSample = halflife_sec.AsFloat().Select(h => (float) (1 - Math.Pow(2, -1 / (ModuleSpace.SampleRate * h))));

		let ratio_per_smp = 1f32 - 2f32.powf(-1f32 / (context.sample_rate_f32() * halflife));

		let result = match self.actual {
			None => signal,
			Some(actual) => (1f32 - ratio_per_smp) * actual + ratio_per_smp * signal,
		};
		self.actual = Some(result);

		output_mono(output, result);
	}
}

pub struct GlideFactory { }
impl NodeFactory for GlideFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![
		spec("halflife", 1),
	] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, base: NodeBase, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		let signal = piped_upstream.as_mono();
		let halflife = node_args.get("halflife").unwrap().as_mono(); 
		Box::new(Glide::new(base, signal, halflife))
	}
}
