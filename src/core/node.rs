extern crate portaudio;
use super::{
	common::*,
	context::*,
	event::*,
	machine::*,
};

pub trait Node {
	fn upstreams(&self) -> Vec<NodeIndex>; // { vec![] }
	fn initialize(&mut self, context: &Context, env: &mut Environment) { }
	fn execute(&mut self, inputs: &Vec<Sample>, context: &Context, env: &mut Environment) -> Sample; // ここで状態を変えないといけない場合があるかも？
	fn update(&mut self, _inputs: &Vec<Sample>, context: &Context, env: &mut Environment) { }
	fn finalize(&mut self, context: &Context, env: &mut Environment) { }
	fn process_event(&mut self, event: &dyn Event) { }
}
