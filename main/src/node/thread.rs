use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
};
use node_macro::node_impl;

use std::sync::mpsc;

pub struct Sender {
	base_: NodeBase,
	signal: MonoNodeIndex,
	sender: mpsc::SyncSender<Vec<Sample>>,
	buffer: Vec<Sample>,
	// capacity は多めに確保されることがあるので別途持っておく
	buffer_size: usize,
}
impl Sender {
	pub fn new(
		base: NodeBase, 
		signal: MonoNodeIndex,
		sender: mpsc::SyncSender<Vec<Sample>>,
		buffer_size: usize,
	) -> Self {
		Self {
			base_: base, 
			signal,
			sender,
			buffer: Vec::with_capacity(buffer_size),
			buffer_size,
		}
	}
}
// TODO ステレオ対応
#[node_impl]
impl Node for Sender {
	fn channels(&self) -> i32 { 0 }
	fn upstreams(&self) -> Upstreams { vec![
		self.signal.channeled(),
	] }
	fn activeness(&self) -> Activeness { Activeness::Active }
	fn execute(&mut self, inputs: &Vec<Sample>, _output: &mut [Sample], _context: &Context, _env: &mut Environment) {
		// TODO ステレオ対応
		let signal = inputs[0];
		self.buffer.push(signal);
		if self.buffer.len() >= self.buffer_size {
			self.sender.send(self.buffer.clone());
			self.buffer.clear();
		}
	}
}

pub struct Receiver {
	base_: NodeBase,
	receiver: mpsc::Receiver<Vec<Sample>>,
	buffer: Option<(Vec<Sample>, usize)>,
	error_count: i32,
}
impl Receiver {
	pub fn new(base: NodeBase, receiver: mpsc::Receiver<Vec<Sample>>) -> Self {
		Self {
			base_: base, 
			receiver,
			buffer: None,
			error_count: 0,
		}
	}
}
// TODO ステレオ対応
#[node_impl]
impl Node for Receiver {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![] }
	fn activeness(&self) -> Activeness { Activeness::Active }
	fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut [Sample], _context: &Context, _env: &mut Environment) {
		let value = match &mut self.buffer {
			Some((buffer, index)) => {
				if *index >= buffer.len() {
					// TODO ちゃんとエラー処理
					let new_buffer = self.receiver.recv().unwrap();
					let mut new_index = 0usize;
					let value = get_buffer_value(&new_buffer, &mut new_index);
					self.buffer = Some((new_buffer, new_index));
					value

				} else {
					get_buffer_value(buffer, index)
				}
			}
			None => {
				let try_result = self.receiver.try_recv();
				match try_result {
					Err(_) => {
						self.error_count += 1;
						0f32
					},
					Ok(new_buffer) => {
						println!("error_count: {}", self.error_count);
						let mut new_index = 0usize;
						let value = get_buffer_value(&new_buffer, &mut new_index);
						self.buffer = Some((new_buffer, new_index));
						value
					}
				}
			}
		};
		output_mono(output, value);
	}
}
fn get_buffer_value(buffer: &Vec<Sample>, index: &mut usize) -> Sample {
	let value = buffer[*index];
	*index += 1;
	value
}
