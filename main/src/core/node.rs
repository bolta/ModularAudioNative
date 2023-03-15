extern crate portaudio;
use std::any::type_name;

use regex::Regex;

use super::{
	common::*,
	context::*,
	event::*,
	machine::*,
};
// use std::collections::hash_set::HashSet;

pub type Upstreams = Vec<ChanneledNodeIndex>;

pub struct NodeBase {
	delay_samples: u32,
}
impl NodeBase {
	pub fn new(delay_samples: u32) -> Self {
		Self { delay_samples }
	}

	pub fn delay_samples(&self) -> u32 { self.delay_samples }
}

pub trait Node: Send {
	fn type_label_default(&self) -> String {
		// foo::bar::Baz<hoge::pita::Boke> → Baz<Boke>
		let modules_re = Regex::new(r"[^<>]*::").unwrap(); // TODO できれば singleton にしたい
		modules_re.replace_all(type_name::<Self>(), "").to_string()
	}
	fn type_label(&self) -> String { self.type_label_default() }
	fn channels(&self) -> i32;
	fn upstreams(&self) -> Upstreams;
	fn activeness(&self) -> Activeness;
	fn initialize(&mut self, _context: &Context, _env: &mut Environment) { }
	// TODO inputs も output と同様にスライスでいいはず
	fn execute(&mut self, _inputs: &Vec<Sample>, _output: &mut [OutputBuffer], _context: &Context, _env: &mut Environment) { }
	fn update(&mut self, _inputs: &Vec<Sample>, _context: &Context, _env: &mut Environment) { }
	fn finalize(&mut self, _context: &Context, _env: &mut Environment) { }
	fn process_event(&mut self, _event: &dyn Event, _context: &Context, _env: &mut Environment) { }

	fn delay_samples(&self) -> u32 { self.base().delay_samples() }

	// 以下は node_impl 属性によって自動実装されるため実装不要
	fn implements_execute(&self) -> bool;
	fn implements_update(&self) -> bool;
	fn base(&self) -> &NodeBase;
}

pub fn output_mono(output: &mut [OutputBuffer], value: Sample) {
	output[0].push(value);
}
pub fn output_stereo(output: &mut [OutputBuffer], value_l: Sample, value_r: Sample) {
	output[0].push(value_l);
	output[1].push(value_r);
}

#[derive(Clone)]
pub enum Activeness {
	/// 更新は一切不要
	Static,

	/// 更新が必要かどうかは入力に依存する
	Passive,

	/// 特定のイベント発生時のみ更新が必要
	Evential/* (HashSet<EventTarget>) */,

	/// 常に更新が必要（状態を持っていて勝手に出力が変わる）
	Active,
}
