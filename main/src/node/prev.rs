use crate::{core::{
	common::*,
	context::*,
	machine::*,
	node::*, node_factory::{NodeArgSpec, NodeArgs, NodeFactory},
}, moddl::{error::ModdlResult, import::ImportCache, io::Io, value::{Value, ValueBody}}};
use node_macro::node_impl;
use parser::common::Location;

use std::rc::Rc;

// TODO ステレオ対応

pub struct PrevIn {
	signal: ChanneledNodeIndex,
	id: PrevId,
}
impl PrevIn {
	pub fn new(
		signal: ChanneledNodeIndex,
		id: PrevId,
	) -> Self {
		Self {
			signal,
			id,
		}
	}
}
#[node_impl]
impl Node for PrevIn {
	fn channels(&self) -> i32 { self.signal.channels() }
	fn upstreams(&self) -> Upstreams { vec![
		self.signal,
	] }
	fn activeness(&self) -> Activeness { Activeness::Active } // TODO でいいか？
	fn features(&self) -> Vec<Feature> { vec![Feature::PrevIn { id: self.id }] }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut [Sample], _context: &Context, _env: &mut Environment) {
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

pub struct PrevInFactory {
	id: PrevId,
}
impl PrevInFactory {
	pub fn new(id: PrevId) -> Self{
		Self { id }
	}
}
impl NodeFactory for PrevInFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, _node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		let signal = piped_upstream.as_mono();
		Box::new(PrevIn::new(signal.channeled(), self.id))
	}

}

pub struct PrevOut {
	channels: i32,
	id: PrevId,
}
impl PrevOut {
	pub fn new(channels: i32, id: PrevId) -> Self {
		Self {
			channels,
			id,
		}
	}
}
#[node_impl]
impl Node for PrevOut {
	fn channels(&self) -> i32 { self.channels }
	fn upstreams(&self) -> Upstreams { vec![] }
	fn activeness(&self) -> Activeness { Activeness::Active } // TODO でいいか？
	fn features(&self) -> Vec<Feature> { vec![Feature::PrevOut { id: self.id }] }
	// 単なるプレースホルダなので、することはない
//	fn execute(&mut self, _inputs: &Vec<Sample>, _output: &mut [Sample], _context: &Context, _env: &mut Environment) { }
}

pub struct PrevOutFactory {
	id: PrevId,
}
impl PrevOutFactory {
	pub fn new(id: PrevId) -> Self{
		Self { id }
	}
}
impl NodeFactory for PrevOutFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, _node_args: &NodeArgs, _piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		Box::new(PrevOut::new(1 /* 仮 */, self.id))
	}

}

pub struct PrevIo {
	id: PrevId,
}
impl PrevIo {
	pub fn new() -> Self {
		Self { id: PrevId(0usize) }
	}
}
impl Io for PrevIo {
	fn perform(&mut self, loc: &Location, _imports: &mut ImportCache) -> ModdlResult<Value> {
		let id = self.id;
		self.id.0 += 1;

		Ok((ValueBody::Assoc(vec![
			("in".to_string(), (ValueBody::NodeFactory(Rc::new(PrevInFactory::new(id))), loc.clone())),
			("out".to_string(), (ValueBody::NodeFactory(Rc::new(PrevOutFactory::new(id))), loc.clone())),
		].into_iter().collect()), loc.clone()))
	}
}
