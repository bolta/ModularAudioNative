use crate::{
	core::{
		common::*,
		node::*,
	},
};

// TODO 別の場所に移す
use crate::{
	node::{
		arith::*,
		env::*,
		osc::*,
		stereo::*,
	},
	wave::{
		waveform_host::*,
	}
};


use std::collections::hash_map::HashMap;

type Error = String;

pub struct NodeArgSpec {
	pub name: String,
	pub channels: i32,
	pub default: Option<Sample>,
}
pub fn spec(name: &str, channels: i32) -> NodeArgSpec {
	NodeArgSpec {
		name: name.to_string(),
		channels,
		default: None,
	}
}
pub fn spec_with_default(name: &str, channels: i32, default: Sample) -> NodeArgSpec {
	NodeArgSpec {
		name: name.to_string(),
		channels,
		default: Some(default),
	}
}

pub type NodeArgs = HashMap<String, ChanneledNodeIndex>;

pub trait NodeFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec>;
	fn input_channels(&self) -> i32;
	/// piped_upstream は接続の前段となっているノード
	fn create_node(&self, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node>;
}

pub struct Env1Factory { }
impl NodeFactory for Env1Factory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, _node_args: &NodeArgs, _piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		Box::new(ExpEnv::new(0.125f32))
	}
}

pub struct PanFactory { }
impl NodeFactory for PanFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![spec("pos", 1)] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		let input = piped_upstream.as_mono();
		let pan = node_args.get("pos").unwrap().as_mono();
		Box::new(Pan::new(input, pan))
	}
}
