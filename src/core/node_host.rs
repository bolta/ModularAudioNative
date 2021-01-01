use super::node::*;
use super::node_handle::*;
use std::cell::RefCell;
use std::rc::Rc;

// exper
extern crate portaudio;
use portaudio as pa;

pub struct NodeHost {
	nodes: Vec<Node>,
}

impl NodeHost {
	pub fn new() -> Self {
		Self { nodes: vec![] }
	}

	pub fn add_node(&mut self, node: Node) -> NodeHandle {
		self.nodes.push(node);

		NodeHandle {
			host: self as *mut Self,
			id: self.nodes.len() - 1,
		}
	}

	pub fn node(&self, id: usize) -> &Node {
		&self.nodes[id]
	}
	pub fn node_mut(&mut self, id: usize) -> &mut Node {
		&mut self.nodes[id]
	}

	pub fn update(&mut self) {
		for n in &mut self.nodes {
			n.update();
		}
	}

	// TODO オーディオと密結合させない
	pub fn play(this: Rc<RefCell<Self>>) {
		let node_exists = this.borrow().nodes.len() > 0;
		if ! node_exists {
			panic!("no nodes to play");
		}

		// コールバックの中から Self のメソッドを呼ぶ関係で、
		// 
		let pa = pa::PortAudio::new().expect("failed to initialise PortAudio");

		let play_sample = |value: f32| {
			println!("{}", value);
		};

		const SMP_RATE: i32 = 44100;
		const QUANT_RATE: i32 = 16;
		const CHANNELS: i32 = 1;
		const FRAMES_PER_BUFFER: u32 = 64;

		let mut settings =
				pa.default_output_stream_settings(CHANNELS, SMP_RATE as f64, FRAMES_PER_BUFFER)
				.expect("failed to initialise settings");
		// we won't output out of range samples so don't bother clipping them.
		settings.flags = pa::stream_flags::CLIP_OFF;

		// This routine will be called by the PortAudio engine when audio is needed. It may called at
		// interrupt level on some machines so don't do anything that could mess up the system like
		// dynamic resource allocation or IO.
		let callback = move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
			let mut idx = 0;
			for _ in 0..frames {
				//_ ここ（self.update() より前）に書くとエラーになる。
				// 厳しすぎないか？　毎回正直に last().unwrap() すると
				// パフォーマンスに悪そうなんだけど…
				// let master = self.nodes.last().unwrap();
				this.borrow_mut().update();
				let value = this.borrow().nodes.last().unwrap().current();
				buffer[idx] = value;
				idx += 1;
			}
			pa::Continue
		};

		let mut stream = pa.open_non_blocking_stream(settings, callback)
				.expect("failed to open stream");

		// TODO エラー処理
		stream.start();

		const NUM_SECONDS: i32 = 2;
		println!("Play for {} seconds.", NUM_SECONDS);
		pa.sleep(NUM_SECONDS * 1_000);

		stream.stop();
		stream.close();
	}
}
