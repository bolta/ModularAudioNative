extern crate portaudio;
use super::common::*;

pub trait Node {
	fn initialize(&mut self) { }
	fn upstreams(&self) -> Vec<NodeIndex>; // { vec![] }
	fn execute(&mut self, inputs: &Vec<Sample>) -> Sample; // ここで状態を変えないといけない場合があるかも？
	fn update(&mut self, _inputs: &Vec<Sample>) { }
	fn finalize(&mut self) { }
}
