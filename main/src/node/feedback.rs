use crate::{core::{
	common::*,
	context::*,
	machine::*,
	node::*, node_factory::{NodeFactory, NodeArgSpec, NodeArgs},
}, moddl::{io::Io, error::ModdlResult, value::{Value, ValueBody}}};
use node_macro::node_impl;
use parser::common::Location;

use std::{sync::mpsc, rc::Rc};

// TODO ステレオ対応

pub struct FeedbackIn {
	base_: NodeBase,
	signal: ChanneledNodeIndex,
	id: FeedbackId,
}
impl FeedbackIn {
	pub fn new(
		base: NodeBase, 
		signal: ChanneledNodeIndex,
		id: FeedbackId,
	) -> Self {
		Self {
			base_: base, 
			signal,
			id,
		}
	}
}
#[node_impl]
impl Node for FeedbackIn {
	fn channels(&self) -> i32 { self.signal.channels() }
	fn upstreams(&self) -> Upstreams { vec![
		self.signal,
	] }
	fn activeness(&self) -> Activeness { Activeness::Active } // TODO でいいか？
	fn features(&self) -> Vec<Feature> { vec![Feature::FeedbackIn { id: self.id }] }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut [OutputBuffer], _context: &Context, _env: &mut Environment) {
		match self.signal {
			ChanneledNodeIndex::NoOutput(_) => { },
			ChanneledNodeIndex::Mono(_) => {
				output_mono(output, inputs[0]);
			},
			ChanneledNodeIndex::Stereo(_) => {
				output_stereo(output, inputs[0], inputs[1]);
			},
		}
	}
}

pub struct FeedbackInFactory {
	id: FeedbackId,
}
impl FeedbackInFactory {
	pub fn new(id: FeedbackId) -> Self{
		Self { id }
	}
}
impl NodeFactory for FeedbackInFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, base: NodeBase, _node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		let signal = piped_upstream.as_mono();
		Box::new(FeedbackIn::new(base, signal.channeled(), self.id))
	}

}

pub struct FeedbackOut {
	base_: NodeBase,
	channels: i32,
	id: FeedbackId,
}
impl FeedbackOut {
	pub fn new(base: NodeBase, channels: i32, id: FeedbackId) -> Self {
		Self {
			base_: base, 
			channels,
			id,
		}
	}
}
#[node_impl]
impl Node for FeedbackOut {
	fn channels(&self) -> i32 { self.channels }
	fn upstreams(&self) -> Upstreams { vec![] }
	fn activeness(&self) -> Activeness { Activeness::Active } // TODO でいいか？
	fn features(&self) -> Vec<Feature> { vec![Feature::FeedbackOut { id: self.id }] }
	// 単なるプレースホルダなので、することはない
//	fn execute(&mut self, _inputs: &Vec<Sample>, _output: &mut [OutputBuffer], _context: &Context, _env: &mut Environment) { }
}

pub struct FeedbackOutFactory {
	id: FeedbackId,
}
impl FeedbackOutFactory {
	pub fn new(id: FeedbackId) -> Self{
		Self { id }
	}
}
impl NodeFactory for FeedbackOutFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, base: NodeBase, _node_args: &NodeArgs, _piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		Box::new(FeedbackOut::new(base, 1 /* 仮 */, self.id))
	}

}

pub struct FeedbackIo {
	id: FeedbackId,
}
impl FeedbackIo {
	pub fn new() -> Self {
		Self { id: FeedbackId(0usize) }
	}
}
impl Io for FeedbackIo {
	fn perform(&mut self, loc: &Location) -> ModdlResult<Value> {
		let id = self.id;
		self.id.0 += 1;

		Ok((ValueBody::Array(vec![
			(ValueBody::NodeFactory(Rc::new(FeedbackInFactory::new(id))), loc.clone()),
			(ValueBody::NodeFactory(Rc::new(FeedbackOutFactory::new(id))), loc.clone()),
		]), loc.clone()))
	}
}
