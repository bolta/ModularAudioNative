use super::{
	value::*,
};
use crate::{
	core::{
		common::*,
//		node_host::*,
node::*,
	},
};

// TODO 別の場所に移す
use crate::node::{
	arith::*,
	env::*,
	osc::*,
};

use std::collections::hash_map::HashMap;

pub type NamedArgs = HashMap<String, Value>;

// pub trait NodeFactory {
// 	fn create_node(&self, /* nodes: &mut NodeHost, tag: &str, */ args: &NamedArgs, piped_upstreams: &Vec<NodeIndex>) -> /* NodeIndex */Box<dyn Node>;
// }

// pub struct SineOscFactory { }
// impl SineOscFactory {
// 	pub fn new() -> Self { Self { } }
// }
// impl NodeFactory for SineOscFactory {
// 	fn create_node(&self, /* nodes: &mut NodeHost, tag: &str, */ _args: &NamedArgs, piped_upstreams: &Vec<NodeIndex>) -> /* NodeIndex */Box<dyn Node> {
// 		let freq = piped_upstreams[0];
// 		Box::new(SineOsc::new(freq))
// 	}
// }

type Error = String;

/// piped_upstreams は接続の前段となっているノード。
/// ModDL では常に 1 つだが、ステレオを考慮すると複数必要になるケースがあるので Vec で受け取る
pub type NodeFactory = dyn Fn (/* args: */ &NamedArgs, /* piped_upstreams: */ &Vec<NodeIndex>) -> Box<dyn Node>;

pub fn create_sine_osc(_args: &NamedArgs, piped_upstreams: &Vec<NodeIndex>) -> Box<dyn Node> {
	let freq = piped_upstreams[0];
	Box::new(SineOsc::new(freq))
}

pub fn create_limit(args: &NamedArgs, piped_upstreams: &Vec<NodeIndex>) -> Box<dyn Node> {
	// TODO エラー処理
	let signal = piped_upstreams[0];
	let min = args.get("min").unwrap().as_node().unwrap();
	let max = args.get("max").unwrap().as_node().unwrap();

	Box::new(Limit::new(signal, min, max))
}

pub fn create_env1(_args: &NamedArgs, _piped_upstreams: &Vec<NodeIndex>) -> Box<dyn Node> {
	Box::new(ExpEnv::new(0.125f32))
}

// pub fn foo() {
// 	let f: Box<NodeFactory> = Box::new(create_sine_osc);
// 	let g: Box<NodeFactory> = Box::new(create_sine_osc);
// }
