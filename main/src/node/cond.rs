use crate::{
	calc::sample_to_bool,
	core::{
		common::*,
		context::*,
		machine::*,
		node::*,
	},
};
use node_macro::node_impl;

 ////
//// Condition

pub struct Condition {
	base_: NodeBase,
	cond: MonoNodeIndex,
	then: MonoNodeIndex,
	els: MonoNodeIndex,
}
impl Condition {
	pub fn new(base: NodeBase, cond: MonoNodeIndex, then: MonoNodeIndex, els: MonoNodeIndex) -> Self {
		Self { base_: base, cond, then, els }
	}
}
#[node_impl]
impl Node for Condition {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![self.cond.channeled(), self.then.channeled(), self.els.channeled()] }
	fn activeness(&self) -> Activeness { Activeness::Passive }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut [Sample], _context: &Context, _env: &mut Environment) {
		let cond = sample_to_bool(inputs[0]);
		let then = inputs[1];
		let els = inputs[2];
		output_mono(output, if cond { then } else { els });
	}
}
// pub struct ConditionFactory { }
// impl NodeFactory for ConditionFactory {
// 	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![spec("cond", 1), spec("then", 1), spec("else", 1)] }
// 	fn input_channels(&self) -> i32 { 1 }
// 	fn create_node(&self, base: NodeBase, node_args: &NodeArgs, _piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
// 		Box::new(Condition::new(
// 			node_args.get("cond").unwrap().as_mono(),
// 			node_args.get("then").unwrap().as_mono(),
// 			node_args.get("else").unwrap().as_mono(),
// 		))
// 	}
// }

