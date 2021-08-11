use super::{
	value::*,
};
use crate::{
	core::{
		common::*,
		node_host::*,
		node::*,
	},
};

// TODO 別の場所に移す
use crate::node::{
	arith::*,
	env::*,
	osc::*,
	stereo::*,
};

use std::collections::hash_map::HashMap;

type Error = String;

pub struct NodeArgSpec { pub name: String, pub channels: i32 }
fn spec(name: &str, channels: i32) -> NodeArgSpec { NodeArgSpec { name: name.to_string(), channels } }

pub type ValueArgs = HashMap<String, Value>;
pub type NodeArgs = HashMap<String, ChanneledNodeIndex>;

pub trait NodeFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec>;
	fn input_channels(&self) -> i32;
	/// piped_upstream は接続の前段となっているノード
	fn create_node(&self, value_args: &ValueArgs, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node>;
}

pub struct SineOscFactory { }
impl NodeFactory for SineOscFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, value_args: &ValueArgs, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		let freq = piped_upstream.as_mono();
		Box::new(SineOsc::new(freq))
	}
}

pub struct LimitFactory { }
impl NodeFactory for LimitFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![spec("min", 1), spec("max", 1)] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, value_args: &ValueArgs, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		let signal = piped_upstream.as_mono();
		// ここは、存在しなければ呼び出し元でエラーにするのでチェック不要、のはず
		let min = node_args.get("min").unwrap().as_mono();
		let max = node_args.get("max").unwrap().as_mono();
		Box::new(Limit::new(signal, min, max))
	}
}

pub struct Env1Factory { }
impl NodeFactory for Env1Factory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, _value_args: &ValueArgs, _node_args: &NodeArgs, _piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		Box::new(ExpEnv::new(0.125f32))
	}
}

pub struct StereoTestOscFactory { }
impl NodeFactory for StereoTestOscFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, _value_args: &ValueArgs, _node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		let freq = piped_upstream.as_mono();
		Box::new(StereoTestOsc::new(freq))
	}
}

pub struct PanFactory { }
impl NodeFactory for PanFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![spec("pos", 1)] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, _value_args: &ValueArgs, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		let input = piped_upstream.as_mono();
		let pan = node_args.get("pos").unwrap().as_mono();
		Box::new(Pan::new(input, pan))
	}
}
