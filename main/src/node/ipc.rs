use std::ptr::null_mut;

use crate::{core::{
	common::*,
	context::*,
	event::{self, *},
	machine::*,
	node::*,
}, node::var::SetEvent};
use ipc::{Response, Server, Set};
use node_macro::node_impl;

pub struct MessageReceiver {
	channel_name: String,
	// Server は Send でないため、Send である Node が普通に持つことはできない。
	// そこで Server の初期化を Machine の稼働開始後（initialize）まで遅延することで
	// Server がスレッドを跨がないようにする
	c2p_server: *mut Server,
}

// Server のスレッド安全性が考慮されているので、Send だと言い張る
unsafe impl Send for MessageReceiver { }

impl MessageReceiver {
	pub fn new(channel_name: &str) -> Self { Self {
		channel_name: channel_name.to_string(),
		c2p_server: null_mut(),
	} }
}

#[node_impl]
impl Node for MessageReceiver {
	fn channels(&self) -> i32 { 0 }
	fn upstreams(&self) -> Upstreams { vec![] }
	fn activeness(&self) -> Activeness { Activeness::Active }
	fn initialize(&mut self, _context: &Context, _env: &mut Environment) {
		// Server を遅延初期化する
		// TODO エラー処理
		let server = Server::new(self.channel_name.as_str()).unwrap();
		self.c2p_server = Box::into_raw(Box::new(server));
	}
	fn finalize(&mut self, _context: &Context, _env: &mut Environment) {
		if self.c2p_server.is_null() { return; }

		unsafe { let _ = Box::from_raw(self.c2p_server); }
	}
	fn execute(&mut self, _inputs: &Vec<Sample>, _output: &mut [Sample], context: &Context, env: &mut Environment) {
		if self.c2p_server.is_null() {
			// 先に initialize() されているので null ではない
			assert!(false); 
			return;
		}

		let c2p_server = unsafe { &*self.c2p_server };

		c2p_server.poll_for_request(|request_bytes| {
			match bson::Document::from_reader(request_bytes) {
				Ok(doc) => {
					println!("player received {}", &doc);
					match doc.get_str("type") {
						Ok(tipe) => {
							match tipe {
								"ping" => {
									let response = Response {
										status: "ok".into(),
										text: "pong".into(),
									};
									Ok(bson::serialize_to_vec(&response).unwrap())
								},
								"set" => {
									let msg: Set = bson::deserialize_from_document(doc).unwrap();
									// print!("set: "); dbg!(&msg);
									let event = SetEvent::new(EventTarget::Tag(msg.target), msg.key, msg.value);
									println!("?????????? {}", context.elapsed_samples());
									// main マシンでの elapsed_samples の流れ方は実時間と同じではない（速い）。
									// 一方各トラックのマシンではほぼ実時間に近い（AudioOut 律速のため）。
									// このため elapsed_samples を正直に渡すと大きく遅れて処理されてしまうので、
									// 最速で処理されるよう 0 固定とする
									// TODO elapsed_samples が不要なら Node としない設計もありかもしれない
									env.broadcast_event(/* context.elapsed_samples() */ 0, Box::new(event));

									let response = Response {
										status: "ok".into(),
										text: "".into(),
									};
									Ok(bson::serialize_to_vec(&response).unwrap())
								},
								_ => {
									// TODO エラーにすべきかもしれない。ちゃんと処理
									print!("_");
									let response = Response {
										status: "ok".into(),
										text: "hello".into(),
									};
									Ok(bson::serialize_to_vec(&response).unwrap())
								},
							}
						}
						Err(e) => todo!()
					}
				}
				Err(e) => todo!()
			}
		}).unwrap_or_else(|e| println!("{}", e));
	}
}
