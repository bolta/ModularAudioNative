use super::node::*;
use super::node_handle::*;
use super::sample_user::*;
use std::cell::RefCell;
use std::rc::Rc;

// exper
extern crate portaudio;
use portaudio as pa;

pub struct NodeHost {
	sample_rate: i32,
	quant_rate: i32,
	channels: i32,

	/// Node は常に参照で見られているため、Vec の引っ越しでアドレスが変わらないよう Box に入れる
	/// （コンパイラのチェックを回避しているので、参照が存在する状態で要素が追加されることもある）
	nodes: Vec<Box<Node>>,

	sample_users: Vec<&'a mut dyn SampleUser>, //Vec<Rc<RefCell<dyn SampleUser>>>,
}

impl NodeHost {
	pub fn new(sample_rate: i32, quant_rate: i32, channels: i32) -> Self {
		Self {
			sample_rate,
			quant_rate,
			channels,
			nodes: vec![],
			sample_users: vec![],
		}
	}

	pub fn sample_rate(&self) -> i32 { self.sample_rate }
	pub fn quant_rate(&self) -> i32 { self.quant_rate }
	pub fn channels(&self) -> i32 { self.channels }

	pub fn add_node(&mut self, node: Node) -> NodeHandle {
		self.nodes.push(Box::new(node));

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

	pub fn add_sample_user(&mut self, user: &'a mut dyn SampleUser/* Rc<RefCell<dyn SampleUser>> */) {
		self.sample_users.push(user);
	}

	pub fn update(&mut self) {
		for s in &mut self.sample_users {
	//            s.borrow_mut().sample();
			s.sample();
		}
		for n in &mut self.nodes {
			n.update();
		}
	}

	// TODO オーディオと密結合させない
	pub fn play(this: Rc<RefCell<Self>>) {
		// コールバックの中から Self のメソッドを呼ぶ関係で、
		// インスタンスメソッドにはせず、外部で寿命管理されたインスタンスを引数でもらう
		let node_exists = this.borrow().nodes.len() > 0;
		if ! node_exists {
			panic!("no nodes to play");
		}

		let pa = pa::PortAudio::new().expect("failed to initialise PortAudio");

		let play_sample = |value: f32| {
			println!("{}", value);
		};

		const FRAMES_PER_BUFFER: u32 = 64;

		let mut settings = {
			let host = this.borrow();
			pa.default_output_stream_settings(host.channels(), host.sample_rate() as f64, FRAMES_PER_BUFFER)
					.expect("failed to initialise settings")
		};
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

		// loop {
		// 	//_ ここ（self.update() より前）に書くとエラーになる。
		// 	// 厳しすぎないか？　毎回正直に last().unwrap() すると
		// 	// パフォーマンスに悪そうなんだけど…
		// 	// let master = self.nodes.last().unwrap();
		// 	this.borrow_mut().update();
		// 	let value = this.borrow().nodes.last().unwrap().current();
		// 	println!("------- out: {}", value);
		// }
	}
}
