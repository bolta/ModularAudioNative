use std::collections::HashMap;

use crate::core::{
	common::*,
	context::*,
	event::*,
	machine::*,
	node::*,
	node_factory::*,
};
use node_macro::node_impl;

pub struct Var {
	base_: NodeBase,
	value: Sample,
}

impl Var {
	pub fn new(base: NodeBase, value: Sample) -> Self { Self { base_: base, value } }
}
#[node_impl]
impl Node for Var {
	fn type_label(&self) -> String {
		format!("{}: {}", self.type_label_default(), self.value)
	}
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![] }
	fn activeness(&self) -> Activeness { Activeness::Evential }
	fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut [OutputBuffer], _context: &Context, _env: &mut Environment) {
		output_mono(output, self.value);
	}

	fn process_event(&mut self, event: &dyn Event, _context: &Context, _env: &mut Environment) {
		if event.event_type() != EVENT_TYPE_SET { return; }

		let event = event.downcast_ref::<SetEvent>().unwrap();
		if event.key() == "value" {
			self.value = event.value();
		}
	}
}

// TODO ModDL からの Var 生成はこれを通すようにする
pub struct VarFactory {
	value: Sample,
}
impl VarFactory {
	pub fn new(value: Sample) -> Self {
		Self { value }
	}
}
impl NodeFactory for VarFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![] }
	fn input_channels(&self) -> i32 { 1 }
	fn default_prop_key(&self) -> Option<String> { Some("value".to_string()) }
	fn initial_values(&self) -> HashMap<String, Sample> {
		vec![
			("value".to_string(), self.value)
		].into_iter().collect()
	}
	fn create_node(&self, base: NodeBase, _node_args: &NodeArgs, _piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		Box::new(Var::new(base, self.value))
	}
}

#[derive(Clone)]
pub struct SetEvent {
	// base: TargetedEventBase,
	target: EventTarget,
	key: String,
	value: Sample,
}
impl SetEvent {
	pub fn new(target: EventTarget, key: String, value: Sample) -> Self {
		Self {
			target,
			key,
			value
		}
	}
	pub fn key(&self) -> &str { self.key.as_str() }
	pub fn value(&self) -> Sample { self.value }
}
impl Event for SetEvent {
	fn target(&self) -> &EventTarget { &self.target }
	fn event_type(&self) -> &str { EVENT_TYPE_SET }
	fn clone_event(&self) -> Box<dyn Event> { clone_event(self) }
}

// const EVENT_TYPE_SET: &str = "Var::Set";
pub const EVENT_TYPE_SET: &str = "Set::Number";
