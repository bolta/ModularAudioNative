use super::node::*;
// use typed_arena::Arena;
use std::rc::Rc;
use std::rc::Weak;
use std::cell::RefCell;

pub struct NodeHost {
	// arena: Arena<Node>,
	// nodes: Vec<&'a Node>,
	// master: Option<&'a mut Node>,
	// nodes: Vec<Node>,
//	nodes: Arena<Node>,

	nodes: Vec<Rc<RefCell<Node>>>,
}

impl NodeHost {
	// pub fn new() -> Self {
	// 	Self {
	// 		arena: Arena::new(),
	// 		// nodes: vec![],
	// 	}
	// }
	// pub fn new(arena: Arena<Node>) -> Self {
	// 	Self {
	// 		nodes: arena.into_vec(),
	// 	}
	// }
	pub fn new() -> Self {
		// Self { nodes: Arena::new(), }
		Self { nodes: vec![], }
	}

	pub fn add_node(&mut self, node: Node) -> Weak<RefCell<Node>> {
		self.nodes.push(Rc::new(RefCell::new(node)));
		Rc::downgrade(self.nodes.last().unwrap())
	}

	// pub fn nodes(&self) -> &Arena<Node> { & self.nodes }

	// pub fn new(nodes: Vec<Node>) -> Self {
	// 	Self {
	// 		nodes,
	// 	}
	// }

	// pub fn add_node<'a>(&'a /*mut*/ self, node: Node) -> &'a mut Node {
	// 	// self.nodes.push(node);
	// 	// TODO node 側にも self を登録したり
	// 	// self.nodes.last().unwrap()

	// 	self.arena.alloc(node)
	// }

	//_ この mut を外さないと play() で E0502 になり、
	// いろいろ試したが回避できなかったので、やむなく mut を外した。
	// RefCell を挟んでいるので外せてしまったのだが、
	// 実際は self の管理下にある node を変更しているのだから、
	// mut を外すのは詐欺ではないのだろうか…
	pub fn update(&/* mut */ self) {
		for n in & self.nodes {
			n.as_ref().borrow_mut().update();
		}
	}

	// この mut はなくても通るのだが、本当は変更しているのでつけておく
	pub fn play(&mut self) {
		let play_sample = |value: f32| {
			println!("{}", value);
		};

		match self.nodes.last() {
			None => panic!("no node to play"), //std::iter::repeat_with(|| { 0f32 }),

			Some(master) => {
				loop {
					self.update();
					//_ この m を、毎回同じ値だからと loop の外で確保すると、
					// already borrowed で panic する
					// （self.update の中で可変参照を取るために違反になると思われる）。
					// この位置である必要がある
					let m = master.borrow();
					(play_sample)(m.current());
				}
			},
		};

	}
}
