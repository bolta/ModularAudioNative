use crate::{common::util::ignore_errors, node::event_scheduler::EventScheduler};

use super::{
	common::*,
	context::*,
	event::*,
	node::*,
	node_host::*,
	util::*,
};

use crate::{
	wave::waveform_host::*,
};

use std::{
	collections::hash_map::HashMap,
	collections::hash_set::HashSet,
	sync::{
		Arc,
		mpsc::Receiver
	},
	ops::DerefMut,
};

use itertools::Itertools; // for into_group_map_by

use ringbuf::{
	Consumer,
	Producer,
	RingBuffer,
};

const EVENT_QUEUE_CAPACITY: usize = 1000;
const BROADCAST_POLLING_INTERVAL: i32 = 1;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ValueIndex(pub usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct OutputIndex(pub usize);

static mut EXECUTE_COUNT: i32 = 0;
static mut UPDATE_COUNT: i32 = 0;

pub struct MachineSpec {
	pub name: String,
	pub nodes: NodeHost,
}

pub struct Machine {
	/// マルチマシン構成のデバッグ用
	name: String,
}
impl Machine {
	pub fn new(name: String) -> Self {
		Self { name }
	}

	// TODO Node から状態を切り離すことができれば mut は不要になるのだが
	pub fn play(
		&mut self,
		context: &mut Context,
		nodes: &mut NodeHost,
		waveforms: &Arc<WaveformHost>,
		broadcaster: Broadcaster,
		broadcast_receiver: Receiver<GlobalEvent>,
		skip_mode_events: Option<Box<dyn Fn () -> Vec<Box<dyn Event>>>>,
	) {
		// ここで追加したノードは Graphviz では出力されない（Graphviz 出力の方が先だから）
		// TODO Player 側で追加した方がいいかも
		let scheduler_idx = nodes.add(Box::new(EventScheduler::new(NodeBase::new(0)))).unchanneled();

		let upstreams: Vec<Vec<ChanneledNodeIndex>> = nodes.nodes().iter()
				.map(|node| node.upstreams())
				.collect();
		//  println!("{}: {:?}: {} nodes ({:?})", self.name, std::thread::current().id(), nodes.count(), &upstreams);
		// TODO max_channels 不要？
		let (value_offsets, value_offsets_reversed, value_count, _max_channels) = {
			let mut value_offsets = HashMap::<NodeIndex, ValueIndex>::new();
			// TODO もうちょっときれいにいけないか
			let mut value_offsets_reversed = HashMap::<ValueIndex, NodeIndex>::new();
			let mut next_val = ValueIndex(0_usize);
			let mut max_channels = 0_usize;
			for (i, node) in nodes.nodes().iter().enumerate() {
				let chs = node.channels();
				max_channels = max_channels.max(chs as usize);
				if chs <= 0 { continue; }
				value_offsets.insert(NodeIndex(i), next_val);
				value_offsets_reversed.insert(next_val, NodeIndex(i));
				next_val.0 += chs as usize;
			}
			(value_offsets, value_offsets_reversed, next_val, max_channels)
		};
		// for i in 0usize .. nodes.count() {
		// 	println!("{}[{}]: delay = {}", &self.name, i, nodes[NodeIndex(i)].delay_samples());
		// }

		let activenesses = compute_activenesses(nodes);
// dbg!(&activenesses);
		let mut update_flags = make_update_flags(&activenesses);
// dbg!(&update_flags);
		// let mut values = vec_with_length(value_count.0);

		// NodeIndex -> Delay
		// 各ノードが最大何サンプル遅れて参照されるか
		let max_downstream_delays = {
			let mut result = vec_with_length_and_init_value(nodes.count(), |_| 0u32);
			for (d, us) in upstreams.iter().enumerate() {
				let delay_down = nodes[NodeIndex(d)].delay_samples();
				for u in us {
					let up_idx = u.unchanneled();
					let delay_up = nodes[u.unchanneled()].delay_samples();
					result[up_idx.0] = result[up_idx.0].max(delay_down - delay_up);
				}
			}
			result
		};

		// ValueIndex -> OutputBuffer
		// 各ノードの出力値。複数個所から参照される可能性がある。また遅れて参照される可能性がある
		let mut values = {
			let mut values = Vec::with_capacity(value_count.0);
			for value_idx in 0 .. value_count.0 {
				let buffer_size = max_downstream_delays[value_offsets_reversed[& ValueIndex(value_idx)].0] as usize + 1;
				values.push(OutputBuffer::new(buffer_size));
			}
			values
		};
		// 各ノードの入力値
		let mut inputs =  sample_vec_with_length(upstreams.iter().map(|us| {
			us.iter().map(|u| u.channels()).sum::<i32>()
		}).max().unwrap() as usize);

		// TODO 結果的に active, evential になるノードと、evential なノードがどの変数に依存するかを求める
		// 

		let instructions = self.compile(nodes, &upstreams, &value_offsets);
		let events = RingBuffer::<Box<dyn Event>>::new(EVENT_QUEUE_CAPACITY);
		let (mut events_prod, mut events_cons) = events.split();

		let mut env = Environment::new(&mut events_prod, &broadcaster, waveforms);

		let start = std::time::Instant::now();

		println!("initializing...");
		for node in nodes.nodes_mut().iter_mut() { node.initialize(context, &mut env); }

		println!("playing...");
		let mut skip = false;
		'play: loop {
			if context.elapsed_samples() % BROADCAST_POLLING_INTERVAL == 0 {
				loop {
					match broadcast_receiver.try_recv() {
						Err(_) => { break; },
						Ok(e) => {
							// NodeHost に入れた EventScheduler を復元する
							// https://stackoverflow.com/questions/42418964/porting-a-c-program-to-rust-of-reinterpret-cast-structs-and-bluetooth
							// TODO 安全ではあるが、もうちょっとやりようはないのだろうか…
							let scheduler_ptr = nodes[scheduler_idx].deref_mut() as *mut dyn Node as *mut EventScheduler;
							let scheduler: &mut EventScheduler = unsafe { &mut *scheduler_ptr };
							scheduler.add_event(e.elapsed_samples(), e.event());
						}
					}
				}
			}
			'do_events: loop {
				match events_cons.pop() {
					None => { break 'do_events; }
					Some(event) => {
						let machine_event = self.consume_event(event, nodes, context, &mut env, &mut update_flags);
						if let Some(typ) = machine_event {
							match typ.as_str() {
								EVENT_TYPE_TERMINATE => break 'play,
								EVENT_TYPE_ENTER_SKIP_MODE => skip = true,
								EVENT_TYPE_EXIT_SKIP_MODE => skip = false,
								EVENT_TYPE_DEBUG_PRINT => println!("debug: event received on machine {} at sample {}", &self.name, context.elapsed_samples()),
								_ => println!("unknown machine event: {}", &typ),
							}
						}
					}
				}
			}

			// スキップ中の場合はサンプルごとのノードの処理を飛ばす。
			// ただし状態を先に進めるのに必要なイベントも発行されなくなるので、別途供給を受ける
			// （必要なイベントとは TickEvent を想定している。Tick が止まると Sequencer が止まり、
			// スキップから抜けられなくなる）
			if skip {
				match skip_mode_events {
					Some(ref generate_events) => {
						let events = generate_events();
						for event in events {
							env.post_event(event);
						}
					},
					None => panic!("skip mode used but skip mode events not provided"),
				}
				continue;
			}

			for instrc in &instructions {
				self.do_instruction(nodes, &instrc, &mut values, &mut inputs, context, &mut env, &update_flags);
			}

			// TODO 削除
			// if context.elapsed_samples() % 44100 == 0 && self.name == "a" {
			// 	let at = context.elapsed_samples() + 1;
			// 	println!("******************** {}", at);
			// 	env.broadcast_event(at, Box::new(DebugPrintEvent { }));
			// }

			update_flags.init();
			context.sample_elapsed();
		}

		println!("finalizing...");
		for node in nodes.nodes_mut().iter_mut().rev() { node.finalize(context, &mut env); }

		let end = std::time::Instant::now();
		println!("{:?}", end.duration_since(start));
		unsafe {
			println!("execute: {}", EXECUTE_COUNT);
			println!("update: {}", UPDATE_COUNT);
		}
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
				let delay_down = nodes[NodeIndex(i)].delay_samples();
				let delay_up = nodes[upstream_idx.unchanneled()].delay_samples();
				let delay_idx = delay_up as i32 - delay_down as i32;
				if delay_idx > 0 { panic!("delay_idx must be non_positive") };

				let instrcs = (0 .. count).map(move |j| Instruction::Load {
					to: InputIndex(input_idx.0 + j),
					from: ValueIndex(from.0 + j),
					delay_idx,
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

	/// マシン対象のイベントの場合、その名前を返す
	fn consume_event(&mut self, event: Box<dyn Event>, nodes: &mut NodeHost, context: &Context, env: &mut Environment, update_flags: &mut UpdateFlags) -> Option<String> {
		match event.target() {
			EventTarget::Machine => {
				Some(event.event_type().to_string())
			}
			EventTarget::Tag(tag) => {
				let idxs = nodes.resolve_tag(&tag);
				for idx in idxs {
					nodes[idx].process_event(&*event, context, env);
					if event.event_type() == "Var::Set" {
						update_flags.apply_event(&tag);
					}
				}

				None
			}
		}
	}

	fn do_instruction(&mut self, nodes: &mut NodeHost, instrc: &Instruction, values: &mut Vec<OutputBuffer>, inputs: &mut Vec<Sample>, context: &Context, env: &mut Environment, update_flags: &UpdateFlags) {
		match instrc {
			Instruction::Load { to, from, delay_idx } => {
				// XXX 遅延数の差を補償しないといけないはずだが、補償するとかえってずれてしまい、
				// しない方がむしろ正しくなる。原因不明
				// inputs[to.0] = values[from.0][*delay_idx];
				inputs[to.0] = values[from.0][0];
			}
			Instruction::Execute{ node_idx, output } => {
				// TODO #4 対応で UpdateFlags が正しく動作しなくなった。とりあえず無効にしておく
// 				if context.elapsed_samples() > 0 && ! update_flags.at(*node_idx) {
// // println!("{:?}: skipping Execute", node_idx);
// 					return;
// 				}
// // println!("{:?}: executing Execute", node_idx);

				let node = &mut nodes[*node_idx];
				let output_slice = match output {
					// TODO 終端の指定はデバッグ目的でしか意味がないのでリリースビルドでは外したい
					Some(o) => &mut values[o.0 .. o.0 + node.channels() as usize],
					None => &mut values[0 .. 0], // 出力なし
				};
				node.execute(&inputs, output_slice, context, env);
				unsafe { EXECUTE_COUNT += 1; }
			}
			Instruction::Update(node_idx) => {
				// TODO #4 対応で UpdateFlags が正しく動作しなくなった。とりあえず無効にしておく
// 				if context.elapsed_samples() > 0 && ! update_flags.at(*node_idx) {
// // println!("{:?}: skipping Update", node_idx);
// 					return;
// 				}
// // println!("{:?}: executing Update", node_idx);
					
				nodes[*node_idx].update(&inputs, context, env);
				unsafe { UPDATE_COUNT += 1; }
			}
		}
	}
}

#[derive(Debug)]
enum ComputedActiveness {
	ComputedEvential(HashSet<String>),
	ComputedActive,
}

fn compute_activenesses(nodes: &NodeHost) -> Vec<ComputedActiveness> {
	// 各ノードから任意個のタグへの対応関係（NodeHost::nodes() の逆引き）を作る
	let node_to_tags: HashMap<NodeIndex, HashSet<String>> = nodes.tags()
			.iter().flat_map(|(tag, nodes)| nodes.iter().map(move |node| (*node, tag.clone())))
			.into_group_map_by(|(node, _)| *node) // ここまでで HashMap<NodeIndex, Vec<(&NodeIndex, &String)>>
			.iter().map(|(node, nodes_and_tags)| {
				(*node, nodes_and_tags.iter().map(|(_, tag)| tag.clone()).collect()) // HashMap の値（Vec）を掃除しつつ HashSet に変換
			}).collect();

	let mut result =  vec![];
	// Passive の処理では他のノードの計算結果を使う必要があるので、for_each で回して result に詰めていく形をとる
	nodes.nodes().iter().enumerate().for_each(|(i, node)| {
		debug_assert!(result.len() == i);
		let computed = match node.activeness() {
			Activeness::Static => ComputedActiveness::ComputedEvential(HashSet::new()),
			Activeness::Passive => {
				node.upstreams().iter().fold(ComputedActiveness::ComputedEvential(HashSet::new()), |acc, up| {
					match acc {
						// 1 つでも ComputedActive があれば、それが勝つ
						ComputedActiveness::ComputedActive => ComputedActiveness::ComputedActive,
						ComputedActiveness::ComputedEvential(acc_tags) => match & result[up.unchanneled().0] {
							// 1 つでも ComputedActive があれば、それが勝つ
							ComputedActiveness::ComputedActive => ComputedActiveness::ComputedActive,
							// 上流の ComputedEvential を全て合併
							ComputedActiveness::ComputedEvential(up_tags) => ComputedActiveness::ComputedEvential({
								acc_tags.union(&up_tags).map(|tag| tag.clone()).collect()
							}),
						},
					}
				})
			},
			Activeness::Evential => ComputedActiveness::ComputedEvential(node_to_tags.get(&NodeIndex(i))
					.map(|labels| labels.clone()).unwrap_or_else(|| HashSet::new())),
			Activeness::Active => ComputedActiveness::ComputedActive,
		};
		result.push(computed);
	});

	result
}

pub type EventProducer = Producer<Box<dyn Event>>;
pub type EventConsumer = Consumer<Box<dyn Event>>;
pub struct Environment<'a> {
	events: &'a mut EventProducer,
	broadcaster: &'a Broadcaster,
	waveforms: &'a Arc<WaveformHost>,
}
impl <'a> Environment<'a> {
	fn new(events: &'a mut EventProducer, broadcaster: &'a Broadcaster, waveforms: &'a Arc<WaveformHost>) -> Self {
		Self { events, broadcaster, waveforms }
	}
	pub fn events_mut(&mut self) -> &mut EventProducer { self.events }
	pub fn post_event(&mut self, event: Box<dyn Event>) {
		ignore_errors(self.events_mut().push(event));
	}
	pub fn broadcast_event(&self, elapsed_samples: SampleCount, event: Box<dyn Event>) {
		// TODO 時刻を外から与えないでよくしたい。Environment と Context ってまとめられないものか
		self.broadcaster.broadcast(GlobalEvent::new(elapsed_samples, event));
	}
	pub fn waveforms(&self) -> &Arc<WaveformHost> { self.waveforms }
}


#[derive(Clone, Copy, Debug)]
struct InputIndex(pub usize);

#[derive(Debug)]
enum Instruction {
	/// 計算済みの値を次の計算のための入力値にコピーする
	/// delay_idx は DelayBuffer にアクセスする際の添字（常に非正）
	Load { to: InputIndex, from: ValueIndex, delay_idx: i32 },
	Execute { node_idx: NodeIndex, output: Option<ValueIndex> },
	Update(NodeIndex),
}

// TODO ちゃんと名前空間を規定する
const EVENT_TYPE_TERMINATE: &str = "Machine::Terminate";
#[derive(Clone)]
pub struct TerminateEvent { }
impl Event for TerminateEvent {
	fn event_type(&self) -> &str { EVENT_TYPE_TERMINATE }
	fn target(&self) -> &EventTarget { &EventTarget::Machine }
	fn clone_event(&self) -> Box<dyn Event> { clone_event(self) }
}
const EVENT_TYPE_ENTER_SKIP_MODE: &str = "Machine::EnterSkipMode";
#[derive(Clone)]
pub struct EnterSkipModeEvent { }
impl Event for EnterSkipModeEvent {
	fn event_type(&self) -> &str { EVENT_TYPE_ENTER_SKIP_MODE }
	fn target(&self) -> &EventTarget { &EventTarget::Machine }
	fn clone_event(&self) -> Box<dyn Event> { clone_event(self) }
}
const EVENT_TYPE_EXIT_SKIP_MODE: &str = "Machine::ExitSkipMode";
#[derive(Clone)]
pub struct ExitSkipModeEvent { }
impl Event for ExitSkipModeEvent {
	fn event_type(&self) -> &str { EVENT_TYPE_EXIT_SKIP_MODE }
	fn target(&self) -> &EventTarget { &EventTarget::Machine }
	fn clone_event(&self) -> Box<dyn Event> { clone_event(self) }
}
const EVENT_TYPE_DEBUG_PRINT: &str = "Machine::DebugPrint";
#[derive(Clone)]
pub struct DebugPrintEvent { }
impl Event for DebugPrintEvent {
	fn event_type(&self) -> &str { EVENT_TYPE_DEBUG_PRINT }
	fn target(&self) -> &EventTarget { &EventTarget::Machine }
	fn clone_event(&self) -> Box<dyn Event> { clone_event(self) }
}

#[derive(Debug)]
struct UpdateFlags {
	current: Vec<u128>,
	init: Vec<u128>,
	event_patterns: HashMap<String, Vec<u128>>,
	event_occurred: bool,
}
impl UpdateFlags {
	fn new(init: Vec<u128>, event_patterns: HashMap<String, Vec<u128>>) -> Self {
		// TODO init と event_patterns の全ての値の長さが同じであることを assert
		Self {
			current: init.clone(),
			init,//: vec![0,0,0,0],
			event_patterns,
			event_occurred: false,
		}
	}
	fn init(&mut self) {
		if ! self.event_occurred { return; }

		self.current.clone_from_slice(& self.init[0 ..]);
		self.event_occurred = false;
	}
	fn apply_event(&mut self, tag: &String) {
		// 存在しない場合は考えない
		let pattern = & self.event_patterns[tag];
		self.current.iter_mut().enumerate().for_each(|(i, chunk)| {
			*chunk |= pattern[i];
		});
		self.event_occurred = true;
	}
	fn at(&self, idx: NodeIndex) -> bool {
		let result = (self.current[idx.0 / 128] >> (idx.0 % 128)) % 2 == 1;

// dbg!(idx.0, result);
		result
	}
}

fn make_update_flags(activenesses: &Vec<ComputedActiveness>) -> UpdateFlags {
	let size = (activenesses.len() + 127) / 128;

	let mut init = vec![0; size];
	let mut event_patterns = HashMap::new();
	activenesses.iter().enumerate().for_each(|(i, a)| {
		let (idx, bit) = (i / 128, i % 128);
		let set_bit = |vec: &mut Vec<u128>| { vec[idx] |= 1 << bit; };

		match a {
			ComputedActiveness::ComputedActive => {
				set_bit(&mut init);
			},
			ComputedActiveness::ComputedEvential(tags) => {
				tags.iter().for_each(|tag| {
					if ! event_patterns.contains_key(tag) {
						event_patterns.insert(tag.clone(), vec![0; size]);
					}
					set_bit(event_patterns.get_mut(tag).unwrap());
				})
			}
		}
	});

	UpdateFlags::new(init, event_patterns)
}


#[cfg(test)]
mod tests {
	use crate::core::{
		machine::*,
	};
	
	use std::{
		collections::hash_map::HashMap,
		collections::hash_set::HashSet,
	};

	#[test]
	fn test_update_flags() {
		let mut flags = UpdateFlags::new(
			vec![0b0001],
			HashMap::from([
				("a".to_string(), vec![0b0110]),
				("b".to_string(), vec![0b1100]),
			]),
		);
		assert_eq!(true, flags.at(NodeIndex(0)));
		assert_eq!(false, flags.at(NodeIndex(1)));
		assert_eq!(false, flags.at(NodeIndex(2)));
		assert_eq!(false, flags.at(NodeIndex(3)));

		flags.apply_event(& "a".to_string());
		assert_eq!(true, flags.at(NodeIndex(0)));
		assert_eq!(true, flags.at(NodeIndex(1)));
		assert_eq!(true, flags.at(NodeIndex(2)));
		assert_eq!(false, flags.at(NodeIndex(3)));

		flags.apply_event(& "b".to_string());
		assert_eq!(true, flags.at(NodeIndex(0)));
		assert_eq!(true, flags.at(NodeIndex(1)));
		assert_eq!(true, flags.at(NodeIndex(2)));
		assert_eq!(true, flags.at(NodeIndex(3)));

		flags.init();
		assert_eq!(true, flags.at(NodeIndex(0)));
		assert_eq!(false, flags.at(NodeIndex(1)));
		assert_eq!(false, flags.at(NodeIndex(2)));
		assert_eq!(false, flags.at(NodeIndex(3)));
	}

	#[cfg(test)]
	#[test]
	fn test_make_update_flags() {
		let activenesses = vec![
			ComputedActiveness::ComputedActive,
			ComputedActiveness::ComputedEvential(HashSet::from([
				"a".to_string(),
			])),
			ComputedActiveness::ComputedEvential(HashSet::from([
				"b".to_string(),
			])),
			ComputedActiveness::ComputedEvential(HashSet::from([
				"a".to_string(),
				"b".to_string(),
			])),
		]; // Vec<ComputedActiveness>
		let flags = make_update_flags(&activenesses);
		assert_eq!(flags.init, vec![0b0001]);
		assert_eq!(flags.event_patterns[&"a".to_string()], vec![0b1010]);
		assert_eq!(flags.event_patterns[&"b".to_string()], vec![0b1100]);
	}

	use crate::{
		calc::*,
		node::{
			arith::*,
			noise::*,
			prim::*,
			var::*,
		},
	};

	#[test]
	fn test_compute_activeness() {
		let assert_active = |activeness: &ComputedActiveness| assert!(matches!(activeness, ComputedActiveness::ComputedActive));
		let assert_evential = |activeness: &ComputedActiveness, expected_content: &HashSet<String>| assert!(match activeness {
			ComputedActiveness::ComputedEvential(tags) => { *tags == *expected_content },
			_ => false,
		});

		{
			let mut nodes = NodeHost::new();
			// 定数は static なので、タグをつけてても ComputedEvential として収集されない
			let a = nodes.add_with_tag("a".to_string(), Box::new(Constant::new(42f32))).as_mono();
			let b = nodes.add_with_tag("b".to_string(), Box::new(Constant::new(42f32))).as_mono();
			nodes.add(Box::new(MonoCalc::<AddCalc>::new(vec![a, b])));

			let activenesses = compute_activenesses(&nodes);
			assert_evential(&activenesses[0], &HashSet::from([]));
			assert_evential(&activenesses[1], &HashSet::from([]));
			assert_evential(&activenesses[2], &HashSet::from([]));
		}
		{
			let mut nodes = NodeHost::new();
			let a = nodes.add_with_tag("a".to_string(), Box::new(Var::new(42f32))).as_mono();
			let b = nodes.add_with_tag("b".to_string(), Box::new(Var::new(42f32))).as_mono();
			nodes.add(Box::new(MonoCalc::<AddCalc>::new(vec![a, b])));

			let activenesses = compute_activenesses(&nodes);
			assert_evential(&activenesses[0], &HashSet::from(["a".to_string()]));
			assert_evential(&activenesses[1], &HashSet::from(["b".to_string()]));
			assert_evential(&activenesses[2], &HashSet::from(["a".to_string(), "b".to_string()]));
		}
		{
			let mut nodes = NodeHost::new();
			let a = nodes.add_with_tag("a".to_string(), Box::new(UniformNoise::new())).as_mono(); // Active
			let b = nodes.add_with_tag("b".to_string(), Box::new(Constant::new(42f32))).as_mono(); // Static
			nodes.add(Box::new(MonoCalc::<AddCalc>::new(vec![a, b])));

			let activenesses = compute_activenesses(&nodes);
			assert_active(&activenesses[0]);
			assert_evential(&activenesses[1], &HashSet::from([]));
			assert_active(&activenesses[2]);
		}
		{
			let mut nodes = NodeHost::new();
			let a = nodes.add_with_tag("a".to_string(), Box::new(UniformNoise::new())).as_mono(); // Active
			let b = nodes.add_with_tag("b".to_string(), Box::new(Var::new(42f32))).as_mono(); // Evential
			nodes.add(Box::new(MonoCalc::<AddCalc>::new(vec![a, b])));

			let activenesses = compute_activenesses(&nodes);
			assert_active(&activenesses[0]);
			assert_evential(&activenesses[1], &HashSet::from(["b".to_string()]));
			assert_active(&activenesses[2]);
		}
	}
}
