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
pub type NodeArgs = HashMap<String, ChanneledNodeIndex>;

pub trait NodeFactory {
	fn node_args(&self) -> Vec<String>;
	/// piped_upstream は接続の前段となっているノード
	fn create_node(&self, value_args: &ValueArgs, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node>;
}

pub struct SineOscFactory { }
impl NodeFactory for SineOscFactory {
	fn node_args(&self) -> Vec<String> { vec![] }
	fn create_node(&self, value_args: &ValueArgs, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		// TODO エラー処理
		let freq = piped_upstream.as_mono();
		Box::new(SineOsc::new(freq))
	}
}

pub struct LimitFactory { }
impl NodeFactory for LimitFactory {
	fn node_args(&self) -> Vec<String> { vec!["min".to_string(), "max".to_string()] }
	fn create_node(&self, value_args: &ValueArgs, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		// TODO エラー処理
		let signal = piped_upstream.as_mono();
		// ここは、存在しなければ呼び出し元でエラーにするのでチェック不要、のはず
		// TODO ↑だが、モノラルであることのチェックは必要
		let min = node_args.get("min").unwrap().as_mono();
		let max = node_args.get("max").unwrap().as_mono();
		Box::new(Limit::new(signal, min, max))
	}
}

pub struct Env1Factory { }
impl NodeFactory for Env1Factory {
	fn node_args(&self) -> Vec<String> { vec![] }
	fn create_node(&self, value_args: &ValueArgs, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		Box::new(ExpEnv::new(0.125f32))
	}
}

pub struct StereoTestOscFactory { }
impl NodeFactory for StereoTestOscFactory {
	fn node_args(&self) -> Vec<String> { vec![] }
	fn create_node(&self, value_args: &ValueArgs, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		// TODO エラー処理
		let freq = piped_upstream.as_mono();
		Box::new(StereoTestOsc::new(freq))
	}
}
