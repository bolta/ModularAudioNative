use super::common::*;
use super::event::*;
use super::node::*;

use std::collections::VecDeque;

pub struct Machine {
	nodes: Vec<Box<dyn Node>>,
	events: VecDeque<Box<dyn Event>>,
}
impl Machine {
	pub fn new() -> Self {
		Self {
			nodes: vec![],
			events: VecDeque::new(),
		}
	}

	pub fn add_node(&mut self, node: Box<dyn Node>) -> NodeIndex {
		self.nodes.push(node);
		NodeIndex(self.nodes.len() - 1)
	}

	// TODO Node から状態を切り離すことができれば mut は不要になるのだが
	pub fn play(&mut self) {
		let num_nodes = self.nodes.len();
		let upstreams: Vec<Vec<NodeIndex>> = self.nodes.iter()
				.map(|node| node.upstreams())
				.collect();
		let instructions = self.compile(&upstreams);

		let mut state = State {
			values: vec_with_length(num_nodes),
			inputs: vec_with_length(upstreams.iter().map(|u| u.len()).max().unwrap()),
			output: 0f32,
			elapsed_samples: 0u32,
		};
	
		println!("initializing...");
		for node in &mut self.nodes { node.initialize(); }

		println!("playing...");
		'play: loop {
			while ! self.events.is_empty() {
				let event = self.events.pop_back().unwrap();
				let terminate = self.consume_event(event);
				if terminate { break 'play; }
			}

			for instrc in &instructions {
				self.do_instruction(&instrc, &mut state);
			}

			// TODO 仮…本当は Sequencer とか EventScheduler が投げる
			if state.elapsed_samples == 44100u32 {
				self.post_event(Box::new(TerminateEvent { }));
			}

			state.elapsed_samples += 1;
		}

		println!("finalizing...");
		for node in self.nodes.iter_mut().rev() { node.finalize(); }
	}

	pub fn post_event(&mut self, event: Box<dyn Event>) {
		self.events.push_back(event);
	}

	fn compile(&self, upstreams: &Vec<Vec<NodeIndex>>) -> Vec<Instruction> {
		// nodes が topologically sorted であることを期待している。
		// 普通に構築すればそうなるはず…
		(0usize .. self.nodes.len()).flat_map(|i| {
			let loads = upstreams[i].iter().enumerate().map(|(input_idx, upstream_idx)| {
				Instruction::Load { to: InputIndex(input_idx), from: *upstream_idx }
			});

			let node_idx = NodeIndex(i);
			loads.chain(vec![
				Instruction::Execute(node_idx),
				Instruction::Store { to: node_idx },
				Instruction::Update(node_idx),
			])
		}).collect()
	}

	/// terminate する場合 true
	fn consume_event(&mut self, event: Box<dyn Event>) -> bool {
		let typ = event.event_type();
		if typ == EVENT_TYPE_TERMINATE { return true; }

		match typ {
			// TODO 各種イベントの処理

			_ => { println!("unknown event type: {}", typ); }
		}

		false
	}

	fn do_instruction(&mut self, instrc: &Instruction, state: &mut State) {
		match instrc {
			Instruction::Load { to, from } => {
				state.inputs[to.0] = state.values[from.0];
			}
			Instruction::Store { to } => {
				state.values[to.0] = state.output;
			}
			Instruction::Execute(node_idx) => {
				state.output = self.nodes[node_idx.0].execute(&state.inputs);
			}
			Instruction::Update(node_idx) => {
				self.nodes[node_idx.0].update(&state.inputs);
			}
		}
	}
}

#[derive(Clone, Copy)]
struct InputIndex(pub usize);

enum Instruction {
	Load { to: InputIndex, from: NodeIndex },
	Store { to: NodeIndex },
	Execute(NodeIndex),
	Update(NodeIndex),
}

struct State {
	values: Vec<Sample>,
	inputs: Vec<Sample>,
	output: Sample,
	elapsed_samples: u32,
}

// TODO ちゃんと名前空間を規定する
const EVENT_TYPE_TERMINATE: &str = "Machine::Terminate";
pub struct TerminateEvent { }
impl Event for TerminateEvent {
	fn event_type(&self) -> &str { EVENT_TYPE_TERMINATE }
}

fn vec_with_length(len: usize) -> Vec<Sample> {
	let mut result = Vec::with_capacity(len);
	for _ in 0 .. len { result.push(0f32); }
	result
}
