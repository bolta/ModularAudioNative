extern crate portaudio;
use super::common::*;
use super::context::*;
use super::machine::*;

pub trait Node {
	fn upstreams(&self) -> Vec<NodeIndex>; // { vec![] }
	fn initialize(&mut self, context: &Context, env: &mut Environment) { }
	fn execute(&mut self, inputs: &Vec<Sample>, context: &Context, env: &mut Environment) -> Sample; // ここで状態を変えないといけない場合があるかも？
	fn update(&mut self, _inputs: &Vec<Sample>, context: &Context, env: &mut Environment) { }
	fn finalize(&mut self, context: &Context, env: &mut Environment) { }
}
