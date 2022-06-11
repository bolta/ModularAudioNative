use crate::common::util::ignore_errors;

use super::{
	common::*,
	context::*,
	event::*,
	node_host::*,
};

use crate::{
	wave::waveform_host::*,
};

use std::{
	collections::hash_map::HashMap,
};

use ringbuf::{
	Consumer,
	Producer,
	RingBuffer,
};

const EVENT_QUEUE_CAPACITY: usize = 1000;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ValueIndex(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct OutputIndex(pub usize);

pub struct Machine {
}
impl Machine {
	pub fn new() -> Self {
		Self {
		}
	}

	// TODO Node から状態を切り離すことができれば mut は不要になるのだが
	pub fn play(&mut self, context: &mut Context, nodes: &mut NodeHost, waveforms: &mut WaveformHost) {
		let upstreams: Vec<Vec<ChanneledNodeIndex>> = nodes.nodes().iter()
				.map(|node| node.upstreams())
				.collect();

		let (value_offsets, value_count, max_channels) = {
			let mut value_offsets = HashMap::<NodeIndex, ValueIndex>::new();
			let mut next_val = ValueIndex(0_usize);
			let mut max_channels = 0_usize;
			for (i, node) in nodes.nodes().iter().enumerate() {
				let chs = node.channels();
				max_channels = max_channels.max(chs as usize);
				if chs <= 0 { continue; }
				value_offsets.insert(NodeIndex(i), next_val);
				next_val.0 += chs as usize;
			}
			(value_offsets, next_val, max_channels)
		};

		// 各ノードの出力値。複数個所から参照される可能性があるのでキャッシュする
		let mut values = vec_with_length(value_count.0);
		// 各ノードの入力値
		let mut inputs =  vec_with_length(upstreams.iter().map(|us| {
			us.iter().map(|u| u.channels()).sum::<i32>()
		}).max().unwrap() as usize);

		let instructions = self.compile(nodes, &upstreams, &value_offsets);
		let events = RingBuffer::<Box<dyn Event>>::new(EVENT_QUEUE_CAPACITY);
		let (mut events_prod, mut events_cons) = events.split();

		let mut env = Environment::new(&mut events_prod, waveforms);
	
		println!("initializing...");
		for node in nodes.nodes_mut().iter_mut() { node.initialize(context, &mut env); }

		println!("playing...");
		'play: loop {
			'do_events: loop {
				match events_cons.pop() {
					None => { break 'do_events; }
					Some(event) => {
						let terminate = self.consume_event(event, nodes, context, &mut env);
						if terminate { break 'play; }
					}
				}
			}

			for instrc in &instructions {
				self.do_instruction(nodes, &instrc, &mut values, &mut inputs, context, &mut env);
			}

			context.sample_elapsed();
		}

		println!("finalizing...");
		for node in nodes.nodes_mut().iter_mut().rev() { node.finalize(context, &mut env); }
	}

	fn compile(&self, nodes: &NodeHost, upstreams: &Vec<Vec<ChanneledNodeIndex>>,
			value_offsets: &HashMap<NodeIndex, ValueIndex>) -> Vec<Instruction> {
		// nodes が topologically sorted であることを期待している。
		// 普通に構築すればそうなるはず…
		(0usize .. nodes.count()).flat_map(|i| {
			let mut input_idx = InputIndex(0_usize);
			let loads = upstreams[i].iter().flat_map(move |upstream_idx| {
				// TODO エラー処理？　出力を持たないノードを upstream に指定している
				let from = * value_offsets.get(& upstream_idx.unchanneled()).unwrap();
				let count = nodes[upstream_idx.unchanneled()].channels() as usize;

				// let to_0 = input_idx;
				let instrcs = (0 .. count).map(move |j| Instruction::Load {
					to: InputIndex(input_idx.0 + j),
					from: ValueIndex(from.0 + j),
				});
				input_idx.0 += count;
				instrcs
			});

			let node_idx = NodeIndex(i);
			let node = & nodes[node_idx];
			loads
					.chain(if node.implements_execute() {
						vec![Instruction::Execute { node_idx, output: value_offsets.get(&node_idx).map(|o| *o) }]
					} else {
						vec![]
					})
					.chain(if node.implements_update() {
						vec![Instruction::Update(node_idx)]
					} else {
						vec![]
					})
		}).collect()
	}

	/// terminate する場合 true
	fn consume_event(&mut self, event: Box<dyn Event>, nodes: &mut NodeHost, context: &Context, env: &mut Environment) -> bool {
		match event.target() {
			EventTarget::Machine => {
				let typ = event.event_type();
				match typ {
					// TODO 各種イベントの処理
					EVENT_TYPE_TERMINATE => true,
		
					_ => {
						println!("unknown event type: {}", typ);
						false
					}
				}
			}
			EventTarget::Tag(tag) => {
				let idxs = nodes.resolve_tag(&tag);
				for idx in idxs {
					nodes[idx].process_event(&*event, context, env);
						// None => {
						// 	println!("unknown node id: {}", &id);
						// }
				}

				false
			}
		}
	}

	fn do_instruction(&mut self, nodes: &mut NodeHost, instrc: &Instruction, values: &mut Vec<Sample>, inputs: &mut Vec<Sample>, context: &Context, env: &mut Environment) {
		match instrc {
			Instruction::Load { to, from } => {
				inputs[to.0] = values[from.0];
				// let to_slice = &mut state.inputs[to.0 .. to.0 + count];
				// let from_slice = &state.values[from.0 .. from.0 + count];
				// to_slice.copy_from_slice(from_slice);
			}
			Instruction::Execute{ node_idx, output } => {
				let node = &mut nodes[*node_idx];
				let output_slice = match output {
					// TODO 終端の指定はデバッグ目的でしか意味がないのでリリースビルドでは外したい
					Some(o) => &mut values[o.0 .. o.0 + node.channels() as usize],
					None => &mut values[0 .. 0], // 出力なし
				};
				node.execute(&inputs, output_slice, context, env);
			}
			Instruction::Update(node_idx) => {
				nodes[*node_idx].update(&inputs, context, env);
			}
		}
	}
}

pub type EventProducer = Producer<Box<dyn Event>>;
pub type EventConsumer = Consumer<Box<dyn Event>>;
pub struct Environment<'a> {
	events: &'a mut EventProducer,
	waveforms: &'a mut WaveformHost,
}
impl <'a> Environment<'a> {
	fn new(events: &'a mut EventProducer, waveforms: &'a mut WaveformHost) -> Self {
		Self { events, waveforms }
	}
	pub fn events_mut(&mut self) -> &mut EventProducer { self.events }
	pub fn post_event(&mut self, event: Box<dyn Event>) {
		ignore_errors(self.events_mut().push(event));
	}
	pub fn waveforms(&self) -> &WaveformHost { self.waveforms }
	pub fn waveforms_mut(&mut self) -> &mut WaveformHost { self.waveforms }
}


#[derive(Clone, Copy, Debug)]
struct InputIndex(pub usize);

#[derive(Debug)]
enum Instruction {
	/// 計算済みの値を次の計算のための入力値にコピー
	Load { to: InputIndex, from: ValueIndex },
	Execute { node_idx: NodeIndex, output: Option<ValueIndex> },
	Update(NodeIndex),
}

// TODO ちゃんと名前空間を規定する
const EVENT_TYPE_TERMINATE: &str = "Machine::Terminate";
pub struct TerminateEvent { }
impl Event for TerminateEvent {
	fn event_type(&self) -> &str { EVENT_TYPE_TERMINATE }
	fn target(&self) -> &EventTarget { &EventTarget::Machine }
}

fn vec_with_length(len: usize) -> Vec<Sample> {
	let mut result = Vec::with_capacity(len);
	for _ in 0 .. len { result.push(0f32); }
	result
}
