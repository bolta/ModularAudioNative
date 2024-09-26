use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
};
use node_macro::node_impl;

use std::sync::mpsc;

pub struct Sender {
	signal: ChanneledNodeIndex,
	sender: mpsc::SyncSender<Vec<Sample>>,
	buffer: Vec<Sample>,
	// capacity は多めに確保されることがあるので別途持っておく
	buffer_size: usize,
}
impl Sender {
	pub fn new(
		signal: ChanneledNodeIndex,
		sender: mpsc::SyncSender<Vec<Sample>>,
		buffer_size: usize,
	) -> Self {
		let channels = signal.channels();
		Self {
			signal,
			sender,
			buffer: Vec::with_capacity(buffer_size * (channels as usize)),
			buffer_size,
		}
	}
}
// TODO ステレオ対応
#[node_impl]
impl Node for Sender {
	fn channels(&self) -> i32 { 0 }
	fn upstreams(&self) -> Upstreams { vec![
		self.signal,
	] }
	fn activeness(&self) -> Activeness { Activeness::Active }
	fn execute(&mut self, inputs: &Vec<Sample>, _output: &mut [Sample], _context: &Context, _env: &mut Environment) {
		match self.signal {
			ChanneledNodeIndex::NoOutput(_) => { },
			ChanneledNodeIndex::Mono(_) => {
				self.buffer.push(inputs[0]);
			},
			ChanneledNodeIndex::Stereo(_) => {
				self.buffer.push(inputs[0]);
				self.buffer.push(inputs[1]);
			},
		}
		// let signal = inputs[0];
		// self.buffer.push(signal);
		if self.buffer.len() >= self.buffer_size {
			// TODO エラー処理
			let _ = self.sender.send(self.buffer.clone());
			self.buffer.clear();
		}
	}
}

pub struct Receiver {
	channels: i32,
	receiver: mpsc::Receiver<Vec<Sample>>,
	buffer: Option<(Vec<Sample>, usize)>,
	error_count: i32,
}
impl Receiver {
	pub fn new(channels: i32, receiver: mpsc::Receiver<Vec<Sample>>) -> Self {
		Self {
			channels,
			receiver,
			buffer: None,
			error_count: 0,
		}
	}
}
// TODO ステレオ対応
#[node_impl]
impl Node for Receiver {
	fn channels(&self) -> i32 { self.channels }
	fn upstreams(&self) -> Upstreams { vec![] }
	fn activeness(&self) -> Activeness { Activeness::Active }
	fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut [Sample], _context: &Context, _env: &mut Environment) {
		match &mut self.buffer {
			Some((buffer, index)) => {
				if *index >= buffer.len() {
					// TODO ちゃんとエラー処理
					let new_buffer = self.receiver.recv().unwrap();
					let mut new_index = 0usize;
					output_buffer_values(output, self.channels, &new_buffer, &mut new_index);
					self.buffer = Some((new_buffer, new_index));
					// value

				} else {
					output_buffer_values(output, self.channels, buffer, index)
				}
			}
			None => {
				let try_result = self.receiver.recv();
				match try_result {
					Err(_) => {
						self.error_count += 1;
						output_zeros(output, self.channels);
					},
					Ok(new_buffer) => {
						println!("error_count: {}", self.error_count);
						let mut new_index = 0usize;
						output_buffer_values(output, self.channels, &new_buffer, &mut new_index);
						self.buffer = Some((new_buffer, new_index));
					}
				}
			}
		};
	}
}
fn output_buffer_values(output: &mut [Sample], channels: i32, buffer: &Vec<Sample>, index: &mut usize) /* -> Sample */ {
	for c in 0 .. channels as usize {
		output[c] = buffer[*index];
		*index += 1;
	}
}
fn output_zeros(output: &mut [Sample], channels: i32) {
	for c in 0 .. channels as usize {
		output[c] = 0f32;
	}
}
