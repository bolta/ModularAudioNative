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
};

use std::collections::hash_map::HashMap;

type Error = String;

pub type ValueArgs = HashMap<String, Value>;
pub type NodeArgs = HashMap<String, NodeIndex>;

pub trait NodeFactory {
	fn node_args(&self) -> Vec<String>;
	/// piped_upstreams は接続の前段となっているノード。
	/// ModDL では常に 1 つだが、ステレオを考慮すると複数必要になるケースがあるので Vec で受け取る
	fn create_node(&self, value_args: &ValueArgs, node_args: &NodeArgs, piped_upstreams: &Vec<NodeIndex>) -> Box<dyn Node>;
}

pub struct SineOscFactory { }
impl NodeFactory for SineOscFactory {
	fn node_args(&self) -> Vec<String> { vec![] }
	fn create_node(&self, value_args: &ValueArgs, node_args: &NodeArgs, piped_upstreams: &Vec<NodeIndex>) -> Box<dyn Node> {
		// TODO エラー処理
		let freq = piped_upstreams[0];
		Box::new(SineOsc::new(freq))
	}
}

pub struct LimitFactory { }
impl NodeFactory for LimitFactory {
	fn node_args(&self) -> Vec<String> { vec!["min".to_string(), "max".to_string()] }
	fn create_node(&self, value_args: &ValueArgs, node_args: &NodeArgs, piped_upstreams: &Vec<NodeIndex>) -> Box<dyn Node> {
		// TODO エラー処理
		let signal = piped_upstreams[0];
		// ここは、存在しなければ呼び出し元でエラーにするのでチェック不要、のはず
		let min = node_args.get("min").unwrap();
		let max = node_args.get("max").unwrap();
		Box::new(Limit::new(signal, *min, *max))
	}
}

pub struct Env1Factory { }
impl NodeFactory for Env1Factory {
	fn node_args(&self) -> Vec<String> { vec![] }
	fn create_node(&self, value_args: &ValueArgs, node_args: &NodeArgs, piped_upstreams: &Vec<NodeIndex>) -> Box<dyn Node> {
		Box::new(ExpEnv::new(0.125f32))
	}
}
