use super::node::*;
use super::node_handle::*;
use std::rc::Rc;
use std::rc::Weak;
use std::cell::RefCell;

pub struct NodeHost {
	nodes: Vec<Node>,
}

impl NodeHost {
		pub fn new() -> Self {
		Self { nodes: vec![], }
	}

	pub fn add_node(&mut self, node: Node) -> NodeHandle {

		self.nodes.push(node);

		NodeHandle {
			host: self as *mut Self,
			id: self.nodes.len() - 1,
		}
	}

	pub fn node(&self, id: usize) -> &Node { & self.nodes[id] }
	pub fn node_mut(&mut self, id: usize) -> &mut Node { &mut self.nodes[id] }

	pub fn update(&mut self) {
		for n in &mut self.nodes {
			n.update();
		}
	}

	pub fn play(&mut self) {
		let play_sample = |value: f32| {
			println!("{}", value);
		};

		// let last = { (self /* as &mut Self */).nodes.last_mut() };
		let node_exists = self.nodes.len() > 0;
		if ! node_exists {
			panic!("no nodes to play");
		} else {
			loop {
				//_ ここ（self.update() より前）に書くとエラーになる。
				// 厳しすぎないか？　毎回正直に last().unwrap() すると
				// パフォーマンスに悪そうなんだけど…
				// let master = self.nodes.last().unwrap();
				self.update();
				(play_sample)(self.nodes.last().unwrap().current());
			}
		}
	}
}
