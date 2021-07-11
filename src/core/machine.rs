use super::{
	common::*,
	context::*,
	event::*,
	node::*,
};

use std::collections::VecDeque;
use ringbuf::{
	Consumer,
	Producer,
	RingBuffer,
};

const EVENT_QUEUE_CAPACITY: usize = 1000;

pub struct Machine {
}
impl Machine {
	pub fn new() -> Self {
		Self {
		}
	}

	// TODO Node から状態を切り離すことができれば mut は不要になるのだが
	pub fn play(&mut self, context: &mut Context, nodes: &mut NodeHost) {
		let num_nodes = nodes.len();
		let upstreams: Vec<Vec<NodeIndex>> = nodes.iter()
				.map(|node| node.upstreams())
				.collect();
		let instructions = self.compile(nodes, &upstreams);

		let mut state = State {
			values: vec_with_length(num_nodes),
			inputs: vec_with_length(upstreams.iter().map(|u| u.len()).max().unwrap()),
			output: 0f32,
		};

		let events = RingBuffer::<Box<dyn Event>>::new(EVENT_QUEUE_CAPACITY);
		let (mut events_prod, mut events_cons) = events.split();

		let mut env = Environment::new(&mut events_prod);
	
		println!("initializing...");
		for node in nodes.iter_mut() { node.initialize(context, &mut env); }

		println!("playing...");
		'play: loop {
			'do_events: loop {
				match events_cons.pop() {
					None => { break 'do_events; }
					Some(event) => {
						let terminate = self.consume_event(event);
						if terminate { break 'play; }
					}
				}
			}

			for instrc in &instructions {
				self.do_instruction(nodes, &instrc, &mut state, context, &mut env);
			}

			context.sample_elapsed();
		}

		println!("finalizing...");
		for node in nodes.iter_mut().rev() { node.finalize(context, &mut env); }
	}

	fn compile(&self, nodes: &NodeHost, upstreams: &Vec<Vec<NodeIndex>>) -> Vec<Instruction> {
		// nodes が topologically sorted であることを期待している。
		// 普通に構築すればそうなるはず…
		(0usize .. nodes.len()).flat_map(|i| {
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

	fn do_instruction(&mut self, nodes: &mut NodeHost, instrc: &Instruction, state: &mut State, context: &Context, env: &mut Environment) {
		match instrc {
			Instruction::Load { to, from } => {
				state.inputs[to.0] = state.values[from.0];
			}
			Instruction::Store { to } => {
				state.values[to.0] = state.output;
			}
			Instruction::Execute(node_idx) => {
				let node = &mut nodes[node_idx.0];
				state.output = node.execute(&state.inputs, context, env);
			}
			Instruction::Update(node_idx) => {
				nodes[node_idx.0].update(&state.inputs, context, env);
			}
		}
	}
}

pub type NodeHost = Vec<Box<dyn Node>>;
pub trait NodeHostExt {
	fn add(&mut self, node: Box<dyn Node>) -> NodeIndex;
}
impl NodeHostExt for NodeHost {
	fn add(&mut self, node: Box<dyn Node>) -> NodeIndex {
		self.push(node);
		NodeIndex(self.len() - 1)
	}
}

pub type EventProducer = Producer<Box<dyn Event>>;
pub type EventConsumer = Consumer<Box<dyn Event>>;
pub struct Environment<'a> {
	events: &'a mut EventProducer,
}
impl <'a> Environment<'a> {
	fn new(events: &'a mut EventProducer) -> Self { Self { events } }
	pub fn events_mut(&mut self) -> &mut EventProducer { self.events }
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
	// elapsed_samples: SampleCount,
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
