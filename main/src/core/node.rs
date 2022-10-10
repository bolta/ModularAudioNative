extern crate portaudio;
use super::{
	common::*,
	context::*,
	event::*,
	machine::*,
};
// use std::collections::hash_set::HashSet;

pub type Upstreams = Vec<ChanneledNodeIndex>;

pub trait Node {
	fn channels(&self) -> i32;
	fn upstreams(&self) -> Upstreams;
	fn activeness(&self) -> Activeness;
	fn initialize(&mut self, _context: &Context, _env: &mut Environment) { }
	// TODO inputs も output と同様にスライスでいいはず
	fn execute(&mut self, _inputs: &Vec<Sample>, _output: &mut [Sample], _context: &Context, _env: &mut Environment) { }
	fn update(&mut self, _inputs: &Vec<Sample>, _context: &Context, _env: &mut Environment) { }
	fn finalize(&mut self, _context: &Context, _env: &mut Environment) { }
	fn process_event(&mut self, _event: &dyn Event, _context: &Context, _env: &mut Environment) { }

	// 以下は node_impl 属性によって自動実装されるため実装不要
	fn implements_execute(&self) -> bool;
	fn implements_update(&self) -> bool;
}

pub fn output_mono(output: &mut [Sample], value: Sample) {
	output[0] = value;
}
pub fn output_stereo(output: &mut [Sample], value_l: Sample, value_r: Sample) {
	output[0] = value_l;
	output[1] = value_r;
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
