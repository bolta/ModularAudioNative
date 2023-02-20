use super::{
	builtin::*,
	error::*,
	evaluator::*,
	path::*,
	player_option::*,
	scope::*,
	value::*,
};
use crate::{
	calc::*,
	common::stack::*,
	core::{
		common::*,
		context::*,
		event::*,
		machine::*,
		node::*,
		node_factory::*,
		node_host::*,
	},
	mml::default::{
		feature::*,
		sequence_generator::*,
	},
	node::{
		audio::*,
		cond::*,
		prim::*,
		stereo::*,
		system::*,
		util::*,
		var::*,
	},
	seq::{
		sequencer::*,
		tick::*,
	},
	vis::{
		visualizer::*,
	},
	wave::{
		waveform_host::*,
		wav_reader::*,
	}
};
extern crate parser;
use parser::{
	mml::default_mml_parser,
	moddl::ast::*,
	moddl::parser::compilation_unit,
};

use std::{
	borrow::Borrow,
	cell::RefCell,
	collections::btree_map::BTreeMap,
	collections::hash_map::HashMap,
	collections::hash_set::HashSet,
	fs::File,
	io::Read,
	path::Path,
	rc::Rc,
	sync::Arc,
	thread,
};

// TODO エラー処理を全体的にちゃんとする

const TAG_SEQUENCER: &str = "seq";

// struct Track<'a> {
// 	instrument: &'a Expr,
// 	mml: String,
// };

#[derive(PartialEq)]
enum MuteSolo { Mute, Solo }

enum TrackSpec {
	Instrument(NodeStructure),
	Effect(HashSet<String>, NodeStructure),
	Groove(NodeStructure),
}

struct PlayerContext {
	moddl_path: String,
	sample_rate: i32,
	tempo: f32,
	ticks_per_bar: i32,
	// トラックごとの instrument/effect
	// （書かれた順序を保持するため Vec で持つ）
	track_specs: Vec<(String, TrackSpec)>,
	// effect に接続されていない、「末端」であるトラック。master でミックスする対象
	terminal_tracks: HashSet<String>,
	grooves: HashMap<String, String>, // トラックに対する Tick のタグ名
	groove_cycle: i32,
	// トラックごとの MML を蓄積
	mmls: BTreeMap<String, String>,
	waveforms: WaveformHost,
	mute_solo: MuteSolo,
	mute_solo_tracks: HashSet<String>,
	vars: Rc<RefCell<Scope>>,
	seq_tags: HashSet<String>,
}
impl PlayerContext {
	fn init(moddl_path: &str, sample_rate: i32) -> Self {
		// ルートに直に書き込むと import したときにビルトインのエントリが衝突するので、1 階層切っておく
		// TODO ルートは singleton にできるはず…
		let root_vars = Scope::root(builtin_vars(sample_rate));
		let vars = Scope::child_of(root_vars);

		Self {
			moddl_path: moddl_path.to_string(),
			sample_rate,
			tempo: 120f32,
			ticks_per_bar: 384,
			track_specs: vec![],
			terminal_tracks: HashSet::new(),
			grooves: HashMap::new(),
			groove_cycle: 384,
			mmls: BTreeMap::new(),
			waveforms: WaveformHost::new(),
			mute_solo: MuteSolo::Mute,
			mute_solo_tracks: HashSet::new(),
			vars,
			seq_tags: HashSet::new(),
		}
	}

	fn get_track_spec(&self, track: &String) -> Option<&TrackSpec> {
		self.track_specs.iter().find(|&elem| elem.0 == *track)
				.map(|elem| &elem.1)
	}
	fn add_track_spec(&mut self, track: &String, spec: TrackSpec) -> ModdlResult<()> {
		match self.get_track_spec(track) {
			None => {
				self.track_specs.push((track.clone(), spec));
				Ok(())
			}
			Some(_) => {
				Err(Error::DirectiveDuplicate { msg: track.clone() })
			}
		}
	}
}

fn process_statements(moddl: &str, sample_rate: i32, moddl_path: &str) -> ModdlResult<PlayerContext> {
	let mut pctx = PlayerContext::init(moddl_path, sample_rate);

	// TODO パーズエラーをちゃんと処理
	let (_, CompilationUnit { statements }) = compilation_unit()(moddl) ?;

	for stmt in &statements {
		process_statement(&stmt, &mut pctx) ?;
	}

	Ok(pctx)
}

fn read_file(path: &str) -> ModdlResult<String> {
	let mut file = File::open(path) ?;
	let mut moddl = String::new();
	file.read_to_string(&mut moddl) ?;

	Ok(moddl)
}

pub fn play(options: &PlayerOptions) -> ModdlResult<()> {
	let moddl_path = options.moddl_path.as_str();
	let moddl = read_file(moddl_path) ?;
	let sample_rate = 44100; // TODO 値を外から渡せるように
	let mut pctx = process_statements(moddl.as_str(), sample_rate, moddl_path) ?;
	
	let mut nodes = AllNodes::new();

	// TODO タグ名を sequence_generator と共通化
	let tempo = nodes.add_node_with_tag(MACHINE_MAIN, "#tempo".to_string(), Box::new(Var::new(NodeBase::new(0), pctx.tempo)));
	let timer = nodes.add_node(MACHINE_MAIN, Box::new(TickTimer::new(
			NodeBase::new(nodes.calc_delay(vec![tempo], false)),
			tempo.node(MACHINE_MAIN).as_mono(), pctx.ticks_per_bar, pctx.groove_cycle)))/* .as_mono() */;

	// TODO even groove を誰も使わない場合は省略
	// TODO マルチマシン化に伴い削除
	nodes.add_node(MACHINE_MAIN, Box::new(Tick::new(
			NodeBase::new(nodes.calc_delay(vec![timer], false)),
			timer.node(MACHINE_MAIN).as_mono(), pctx.groove_cycle, make_seq_tag(None, &mut pctx.seq_tags))));

	let mut output_nodes = HashMap::<String, NodeId>::new();

	// for (track, mml) in &pctx.mmls {
	for (track, spec) in &pctx.track_specs {
		let submachine_idx = nodes.add_submachine(track.clone());
		let mml = &pctx.mmls.get(track).map(|mml| mml.as_str()).unwrap_or("");
		let output_node = {
			// @mute で指定されているか、@solo で指定されていなければ、ミュート対象
			if pctx.mute_solo_tracks.contains(track) == (pctx.mute_solo == MuteSolo::Mute) {
				Some(nodes.add_node(submachine_idx, Box::new(Constant::new(0f32))))
			} else {
				// let seq_tag = pctx.grooves.get(track).map(|t| t.clone()).unwrap_or(TAG_SEQUENCER.to_string());
				let seq_tag = match pctx.grooves.get(track) {
					Some(g) => g.clone(),
					None => make_seq_tag(None, &mut pctx.seq_tags),
				};
				match spec {
					TrackSpec::Instrument(structure) => {
						Some(build_nodes_by_mml(track.as_str(), structure, mml, pctx.ticks_per_bar, &seq_tag, &mut nodes, submachine_idx,
								&mut PlaceholderStack::init(HashMap::new()), None, timer, pctx.groove_cycle) ?)
					}
					TrackSpec::Effect(source_tracks, structure) => {
						let mut placeholders = PlaceholderStack::init(HashMap::new());
						source_tracks.iter().for_each(|track| {
							placeholders.top_mut().insert(track.clone(), output_nodes[track]);
						});
						Some(build_nodes_by_mml(track.as_str(), structure, mml, pctx.ticks_per_bar, &seq_tag, &mut nodes, submachine_idx,
								&mut placeholders, None, timer, pctx.groove_cycle) ?)
					}
					TrackSpec::Groove(structure) => {
						// TODO しくみから再考が必要
						// let groovy_timer = build_nodes_by_mml(track.as_str(), structure, mml, pctx.ticks_per_bar, &seq_tag, &mut nodes, submachine_idx, &mut PlaceholderStack::init(HashMap::new()), Some(timer))?.as_mono();
						// nodes.add_node(submachine_idx, Box::new(Tick::new(groovy_timer, pctx.groove_cycle, seq_tag.clone())));

						None
					}
				}
			}
		};
		match output_node {
			Some(node) => { output_nodes.insert(track.clone(), node); },
			None => { },
		};
	}

	let mut terminal_tracks: Vec<&String> = pctx.terminal_tracks.iter().collect();
	terminal_tracks.sort_unstable();
	let terminal_nodes: Vec<NodeId> = terminal_tracks.iter().map(|t| output_nodes[*t]).collect();

	let machine_mix = nodes.add_submachine("mix".to_string());
	let mix = {
		if terminal_nodes.is_empty() {
			nodes.add_node(machine_mix, Box::new(Constant::new(0f32)))
		} else {
			// FIXME Result が絡むときの fold をきれいに書く方法
			let head = *terminal_nodes.first().unwrap();
			let tail = &terminal_nodes[1..];
			let mut sum = head;
			for t in tail {
				sum = add(None, &mut nodes, machine_mix, sum, *t) ?;
			}
			sum
		}
	};
	let master_vol = nodes.add_node(machine_mix, Box::new(Constant::new(0.5f32))); // TODO 値を外から渡せるように
	let master = multiply(None, &mut nodes, machine_mix, mix, master_vol) ?;
	let (master_node, master_delay) = ensure_on_machine(&mut nodes, master, MACHINE_MAIN);

	match &options.output {
		PlayerOutput::Audio => {
			nodes.add_node(MACHINE_MAIN,
					Box::new(PortAudioOut::new(NodeBase::new(master_delay), master_node)));
		},
		PlayerOutput::Wav { path } => {
			// wav ファイルに出力
			nodes.add_node(MACHINE_MAIN,
					Box::new(crate::node::file::WavFileOut::new(NodeBase::new(master_delay), master_node, path.clone())));
		},
		PlayerOutput::Stdout => {
			// stdout に出力
			nodes.add_node(MACHINE_MAIN,
					Box::new(Print::new(NodeBase::new(master_delay), master_node)));
		},
		PlayerOutput::Null => {
			// 出力しない（パフォーマンス計測用）
			nodes.add_node(MACHINE_MAIN,
					Box::new(NullOut::new(NodeBase::new(master_delay), master_node)));
		},
	}

	// TODO タグ名共通化
	nodes.add_node_with_tag(MACHINE_MAIN, "terminator".to_string(),
			Box::new(Terminator::new(NodeBase::new(master_delay), master_node)));

	// 一定時間で終了
	// TODO コマンドオプションで指定できるように
	// let mut sched = crate::node::event_scheduler::EventScheduler::new();
	// sched.add_event(60 * 44100, Box::new(TerminateEvent { }));
	// nodes.add(Box::new(sched));

	let seq_tags = pctx.seq_tags.clone(); // TODO 本来 clone 不要のはず
	// skip 時にメインループの代わりに tick を提供する関数
	let skip_mode_events: Box<dyn Fn () -> Vec<Box<dyn Event>>> = Box::new(move || {
		// 型がうまく合わないのでやむを得ずループで書く
		//  seq_tags.iter().map(|tag| Box::<dyn Event>::new(TickEvent::new(EventTarget::Tag(tag.clone())))).collect::<Vec<Box<dyn Event>>>()
		let mut events: Vec<Box<dyn Event>> = vec![];
		for tag in &seq_tags {
			let target = EventTarget::Tag(tag.clone());
			events.push(Box::new(TickEvent::new(target)));
		}
		events
	});

	let sends_to_receives = nodes.sends_to_receives().clone();
	let nodes_result = nodes.result();
	// TODO コマンドオプションで指定されたときだけ出力する
	output_structure(&nodes_result, &sends_to_receives);

	let waveforms = Arc::new(pctx.waveforms);
	let joins: Vec<_> = nodes_result.into_iter().map(|mut machine_spec| {
		let waveforms = Arc::clone(&waveforms);
		thread::spawn(move || {
			// TODO skip_mode_events が供給できていない
			Machine::new(machine_spec.name).play(&mut Context::new(sample_rate), &mut machine_spec.nodes, &waveforms, None);
		})
	}).collect();
	for j in joins {
		j.join();
	}

	Ok(())
}

fn output_structure(all: &Vec<MachineSpec>, sends_to_receives: &HashMap<NodeId, NodeId>) {
	output_graph(make_graph(all, sends_to_receives));
}

pub fn import(moddl_path: &str, base_moddl_path: &str, sample_rate: i32) -> ModdlResult<HashMap<String, Value>> {
	let resolved_path = resolve_path(moddl_path, base_moddl_path);
	// TODO resolved_path が valid unicode でない場合のエラー処理
	let resolved_path_str = resolved_path.to_str().unwrap();
	let moddl = read_file(resolved_path_str) ?;
	let pctx = process_statements(moddl.as_str(), sample_rate, resolved_path_str) ?;

	// pctx.vars.borrow() が通らない。こう書かないといけない
	// https://github.com/rust-lang/rust/issues/41906#issuecomment-301279688
	let vars = RefCell::<Scope>::borrow(&*pctx.vars);
	Ok(vars.entries().clone())
}

fn process_statement<'a>(stmt: &'a Statement, pctx: &mut PlayerContext) -> ModdlResult<()> {
	match stmt {
		Statement::Directive { name, args } => {
			match name.as_str() {
				"tempo" => {
					(*pctx).tempo = evaluate_arg(&args, 0, &mut pctx.vars)?.as_float()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
				},
				"instrument" => {
					let tracks = evaluate_arg(&args, 0, &mut pctx.vars)?.as_track_set()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					// let instrm = & args[1];
					for track in tracks {
						let instrm = evaluate_arg(&args, 1, &mut pctx.vars)?.as_node_structure()
								.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
						pctx.add_track_spec(&track, TrackSpec::Instrument(instrm)) ?;
						pctx.terminal_tracks.insert(track);
					}
				}
				"effect" => {
					let tracks = evaluate_arg(&args, 0, &mut pctx.vars)?.as_track_set()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					let source_tracks = evaluate_arg(&args, 1, &mut pctx.vars)?.as_track_set()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					// TODO source_tracks の各々が未定義ならエラーにする（循環が生じないように）

					// 定義を評価する際、source_tracks の各々を placeholder として定義しておく。
					let vars = Scope::child_of(pctx.vars.clone());
					
					for source_track in &source_tracks {
						pctx.vars.borrow_mut().set(source_track,
								Value::NodeStructure(NodeStructure::Placeholder { name: source_track.clone() })) ?;
						pctx.terminal_tracks.remove(source_track);
					}

					let effect = evaluate_arg(&args, 2, &vars)?.as_node_structure()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					for track in tracks {
						pctx.add_track_spec(&track, TrackSpec::Effect(source_tracks.iter().map(|t| t.clone()).collect(), effect.clone())) ?;
						pctx.terminal_tracks.insert(track);
					}
				}
				"grooveCycle" => {
					(*pctx).groove_cycle = evaluate_arg(&args, 0, &mut pctx.vars)?.as_float()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ? as i32;
				},
				"groove" => {
					let tracks = evaluate_arg(&args, 0, &mut pctx.vars)?.as_track_set()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					if tracks.len() != 1 { return Err(Error::TooManyTracks); }
					let control_track = &tracks[0];
					let target_tracks = evaluate_arg(&args, 1, &mut pctx.vars)?.as_track_set()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					let body = evaluate_arg(&args, 2, &mut pctx.vars)?.as_node_structure()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					pctx.add_track_spec(control_track, TrackSpec::Groove(body)) ?;
					// groove トラック自体の制御もそれ自体の groove の上で行う（even で行うことも可能だが）
					pctx.grooves.insert(control_track.clone(), make_seq_tag(Some(&control_track), &mut pctx.seq_tags));
					for track in &target_tracks {
						if pctx.grooves.contains_key(track) { return Err(Error::GrooveTargetDuplicate { track: track.clone() }); }
						pctx.grooves.insert(track.clone(), make_seq_tag(Some(&control_track), &mut pctx.seq_tags));
					}
				}
				"let" => {
					let name = evaluate_arg(&args, 0, &mut pctx.vars)?.as_identifier_literal()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					let value = evaluate_arg(&args, 1, &mut pctx.vars) ?;
					pctx.vars.borrow_mut().set(&name, value) ?;
				}
				"waveform" => {
					let name = evaluate_arg(&args, 0, &mut pctx.vars)?.as_identifier_literal()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					let value = evaluate_arg(&args, 1, &mut pctx.vars) ?;
					let path = value.as_string().ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					// TODO 読み込み失敗時のエラー処理
					let index = pctx.waveforms.add(read_wav_file(path.as_str(), None, None, None, None) ?);
					pctx.vars.borrow_mut().set(&name, Value::WaveformIndex(index));
				}
				"ticksPerBar" => {
					let value = evaluate_arg(&args, 0, &mut pctx.vars) ?.as_float()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					// TODO さらに、正の整数であることを検証
					(*pctx).ticks_per_bar = value as i32;
				}
				"ticksPerBeat" => {
					let value = evaluate_arg(&args, 0, &mut pctx.vars) ?.as_float()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					// TODO さらに、正の整数であることを検証
					(*pctx).ticks_per_bar = 4 * value as i32;
				}
				"mute" => {
					let tracks = evaluate_arg(&args, 0, &mut pctx.vars)?.as_track_set()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					set_mute_solo(MuteSolo::Mute, &tracks, pctx);
				}
				"solo" => {
					let tracks = evaluate_arg(&args, 0, &mut pctx.vars)?.as_track_set()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					set_mute_solo(MuteSolo::Solo, &tracks, pctx);
				}
				"import" => {
					let path = evaluate_arg(&args, 0, &mut pctx.vars) ?.as_string()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					let imported_vars = import(&path, pctx.moddl_path.as_str(), pctx.sample_rate) ?;
					imported_vars.iter().try_for_each(|(name, value)| {
						pctx.vars.borrow_mut().set(name, value.clone())
					}) ?;
				}
				other => {
					println!("unknown directive: {}", other);
				}
			}
		}
		Statement::Mml { tracks, mml } => {
			for track in tracks {
				if let Some(mml_concat) = pctx.mmls.get_mut(track) {
					mml_concat.push_str(mml.as_str());
				} else {
					pctx.mmls.insert(track.clone(), mml.clone());
				}
			}
		}
	}

	Ok(())
}
/// シーケンサのタグ名を生成する。また生成したタグ名を記録する
fn make_seq_tag(track: Option<&String>, tags: &mut HashSet<String>) -> String {
	let tag = match track {
		None => "#seq".to_string(),
		Some(track) => format!("#seq_{}", track),
	};
	tags.insert(tag.clone());

	tag
}

fn set_mute_solo(mute_solo: MuteSolo, tracks: &Vec<String>, pctx: &mut PlayerContext) {
	(*pctx).mute_solo = mute_solo;
	(*pctx).mute_solo_tracks.clear();
	tracks.iter().for_each(|t| {
		(*pctx).mute_solo_tracks.insert(t.clone());
	});
}

fn evaluate_arg(args: &Vec<Expr>, index: usize, vars: &Rc<RefCell<Scope>>) -> ModdlResult<Value> {
	if index < args.len() {
		evaluate(&args[index], vars)
	} else {
		Err(Error::DirectiveArgNotFound)
	}
}

struct EventIter {

}
impl Iterator for EventIter {
	type Item = Box<dyn crate::core::event::Event>;
	fn next(&mut self) -> Option<Box<dyn crate::core::event::Event>> { None }
}

// TODO 引数を整理できるか
fn build_nodes_by_mml<'a>(track: &str, instrm_def: &NodeStructure, mml: &'a str, ticks_per_bar: i32, seq_tag: &String, nodes: &mut AllNodes, submachine_idx: MachineIndex, placeholders: &mut PlaceholderStack, override_input: Option<NodeId>, timer: NodeId, groove_cycle: i32)
		-> ModdlResult<NodeId> {
	let (_, ast) = default_mml_parser::compilation_unit()(mml) ?; // TODO パーズエラーをちゃんとラップする
	let freq_tag = format!("{}_freq", track);

	let tag_set = TagSet {
		freq: freq_tag.clone(),
		note: track.to_string(),
	};
	let (seqs, features) = generate_sequences(&ast, ticks_per_bar, &tag_set, format!("{}.", &track).as_str());
	let _seqr = nodes.add_node_with_tag(submachine_idx, seq_tag.to_string(), Box::new(Sequencer::new(NodeBase::new(0), seqs)));

	let mut input = match override_input {
		Some(input) => input,
		None => nodes.add_node_with_tag(submachine_idx, freq_tag.clone(), Box::new(Var::new(NodeBase::new(0), 0f32))),
	};
	if features.contains(&Feature::Detune) {
		// セント単位のデチューン
		// freq_detuned = freq * 2 ^ (detune / 1200)
		// TODO タグ名は feature requirements として generate_sequences の際に受け取る
		let detune = nodes.add_node_with_tag(submachine_idx, format!("{}.#detune", &track), Box::new(Var::new(NodeBase::new(0), 0f32)));
		let cents_per_oct = nodes.add_node(submachine_idx, Box::new(Constant::new(1200f32)));
		let detune_oct = divide(Some(track), nodes, submachine_idx, detune, cents_per_oct) ?; // 必ず成功するはず
		let const_2 = nodes.add_node(submachine_idx, Box::new(Constant::new(2f32)));
		let freq_ratio = power(Some(track), nodes, submachine_idx, const_2, detune_oct) ?; // 必ず成功するはず
		let freq_detuned = multiply(Some(track), nodes, submachine_idx, input, freq_ratio) ?; // 必ず成功するはず
		input = freq_detuned;
	}
	
	let instrm = build_instrument(track, instrm_def, nodes, submachine_idx, input, placeholders) ?;

	let mut output = instrm;
	if features.contains(&Feature::Velocity) {
		// TODO タグ名は feature requirements として generate_sequences の際に受け取る
		// Var に渡す 1 は velocity, volume の初期値（1 が最大）
		let vel = nodes.add_node_with_tag(submachine_idx, format!("{}.#velocity", &track), Box::new(Var::new(NodeBase::new(0), 1f32)));
		let output_vel = multiply(Some(track), nodes, submachine_idx, output, vel) ?; // 必ず成功するはず
		output = output_vel;
	}
	if features.contains(&Feature::Volume) {
		// TODO タグ名は feature requirements として generate_sequences の際に受け取る
		// Var に渡す 1 は velocity, volume の初期値（1 が最大）
		let vol = nodes.add_node_with_tag(submachine_idx, format!("{}.#volume", &track), Box::new(Var::new(NodeBase::new(0), 1f32)));
		let output_vol = multiply(Some(track), nodes, submachine_idx, output, vol) ?; // 必ず成功するはず
		output = output_vol;
	}

	let (tick_node, tick_delay) = ensure_on_machine(nodes, timer, submachine_idx);
	nodes.add_node(submachine_idx, Box::new(Tick::new(NodeBase::new(tick_delay), tick_node.as_mono(), groove_cycle, seq_tag.clone())));

	Ok(output)
}

pub type PlaceholderStack = Stack<HashMap<String, NodeId>>;

fn build_instrument(track: &str, instrm_def: &NodeStructure, nodes: &mut AllNodes, submachine_idx: MachineIndex, freq: NodeId, placeholders: &mut PlaceholderStack) -> ModdlResult<NodeId> {
	fn visit_struct(track: &str, strukt: &NodeStructure, nodes: &mut AllNodes, submachine_idx: MachineIndex, input: NodeId, default_tag: Option<String>, placeholders: &mut PlaceholderStack) -> ModdlResult<NodeId> {
		// 関数にするとライフタイム関係？のエラーが取れなかったので…
		macro_rules! recurse {
			// $const_tag は、直下が定数値（ノードの種類としては Var）であった場合に付与するタグ
			($strukt: expr, $input: expr, $const_tag: expr) => { visit_struct(track, $strukt, nodes, submachine_idx, $input, Some($const_tag), placeholders) };
			($strukt: expr, $input: expr) => { visit_struct(track, $strukt, nodes, submachine_idx, $input, None, placeholders) };
		}
		// 関数にすると（同上）
		macro_rules! add_node {
			// トラックに属する node は全てトラック名のタグをつける
			($new_node: expr) => { Ok(nodes.add_node_with_tag(submachine_idx, track.to_string(), $new_node)) }
		}

		// ノードの引数をデフォルトを考慮して解決する
		let mut make_node_args = |args: &HashMap<String, Value>, fact: &Rc<dyn NodeFactory>/* , label: String */|
				-> ModdlResult<(NodeArgs, u32)> {
			let specs = fact.node_arg_specs();
			let mut node_args = NodeArgs::new();
			let mut max_delay = 0u32;
			for NodeArgSpec { name, channels, default } in specs {
				let arg_val = args.iter().find(|(n, _)| **n == *name );
				let strukt = if let Some(arg_val) = arg_val {
					arg_val.1.as_node_structure()
							// node_args に指定された引数なのに NodeStructure に変換できない
							.ok_or_else(|| Error::NodeFactoryNotFound) ?
				} else if let Some(default) = default {
					Value::Float(default).as_node_structure().unwrap()
				} else {
					// 必要な引数が与えられていない
					Err(Error::NodeFactoryNotFound) ?
				};
				// ラベルが明示されていればそちらを使う
				let arg_name = arg_val.map(|(_, value)| value.label()).flatten().unwrap_or(name.clone());
				let arg_node = recurse!(&strukt, input, arg_name) ?;
				let coerced_arg_node = match coerce_input(Some(track), nodes, submachine_idx, arg_node, channels) {
					Some(result) => result,
					// モノラルであるべき node_arg にステレオが与えられた場合、
					// 勝手にモノラルに変換するとロスが発生するのでエラーにする
					None => Err(Error::ChannelMismatch),
				} ?;
				// node_args.insert(name.clone(), ensure_on_machine(nodes, coerced_arg_node, submachine_idx));
				let (node_idx, delay) = ensure_on_machine(nodes, coerced_arg_node, submachine_idx);
				node_args.insert(name.clone(), node_idx);
				max_delay = max_delay.max(delay);
			}
			Ok((node_args, max_delay))
		};
		
		match strukt {
			NodeStructure::Calc { node_factory, args } => {
				// TODO Result が絡んでるときも map できれいに書きたい
				let mut arg_nodes = vec![];
				for arg in args {
					arg_nodes.push(recurse!(arg, input) ?);
				}

				create_calc_node(Some(track), nodes, submachine_idx, arg_nodes, node_factory.borrow())
			},

			NodeStructure::Connect(lhs, rhs) => {
				// TODO mono/stereo 変換
				let l_node = recurse!(lhs, input) ?;
				recurse!(rhs, l_node)
			},

			NodeStructure::Condition { cond, then, els } => {
				let cond_result = recurse!(cond, input) ?;
				let cond_result = ensure_on_machine(nodes, cond_result, submachine_idx);
				let then_result = recurse!(then, input) ?;
				let then_result = ensure_on_machine(nodes, then_result, submachine_idx);
				let else_result = recurse!(els, input) ?;
				let else_result = ensure_on_machine(nodes, else_result, submachine_idx);
				// let max_delay = * vec![cond_result_on_machine.1, then_result_on_machine.1, else_result_on_machine.1].iter().max().unwrap();
				let max_delay = cond_result.1.max(then_result.1).max(else_result.1);

				// TODO ステレオ対応（入力のどれかがステレオならステレオに拡張する）
				// let mut to_mono = |node| ensure_on_machine(nodes, node, submachine_idx).as_mono();
				let node = Box::new(Condition::new(
					// NodeBase::new(nodes.calc_delay(vec![cond_result, then_result, else_result], false)),
					NodeBase::new(max_delay),
					cond_result.0.as_mono(), then_result.0.as_mono(), else_result.0.as_mono()));
				add_node!(node)
			},

			NodeStructure::Lambda { input_param, body } => {
				placeholders.push_clone();
				placeholders.top_mut().insert(input_param.clone(), input);

				let result = recurse!(body, input);

				placeholders.pop();

				result
			}

			// NodeStructure::Identifier(id) => {
			// 	// id は今のところ引数なしのノード生成しかない
			// 	let fact = factories.get(id).ok_or_else(|| Error::NodeFactoryNotFound) ?;
			// 	apply_input(Some(track), nodes, fact, &ValueArgs::new(), &NodeArgs::new(), input)
			// },
			NodeStructure::NodeFactory(fact) => {
				let (node_args, delay) = make_node_args(&HashMap::new(), fact) ?;
				apply_input(Some(track), nodes, submachine_idx, fact, delay, &node_args, input)
			},
			NodeStructure::NodeWithArgs { factory, label: _, args } => {
				// 引数ありのノード生成
				let fact = match &**factory {
					NodeStructure::NodeFactory(fact) => Ok(fact),
					_ => { dbg!("poke"); Err(Error::DirectiveArgTypeMismatch) },
				} ?;
				let (node_args, delay) = make_node_args(args, fact/* , &label */) ?;

				apply_input(Some(track), nodes, submachine_idx, fact, delay, &node_args, input)
			},
			NodeStructure::Constant { value, label } => {
				let node = Box::new(Var::new(NodeBase::new(0), *value));
				let local_tag = label.as_ref().or(default_tag.as_ref());
				let full_tag = local_tag.map(|tag| format!("{}.{}", track, tag.clone()));
				// dbg!(label, &default_tag, &local_tag, &full_tag);
				match full_tag {
					Some(tag) => Ok(nodes.add_node_with_tags(submachine_idx, vec![track.to_string(), tag], node)),
					None => add_node!(node),
				}
				
			},
			NodeStructure::Placeholder { name } => {
				// 名前に対応する placeholder は必ずある
				Ok(placeholders.top()[name])
			},
		}
	}

	visit_struct(track, instrm_def, nodes, submachine_idx, freq, None, placeholders)
}

/// 入力のチャンネル数が指定の数になるよう、必要に応じて変換をかます。
/// 変換が必要なければ入力をそのまま返す。
/// 変換できない場合は None を返す
fn coerce_input(
	track: Option<&str>,
	nodes: &mut AllNodes,
	submachine_idx: MachineIndex,
	input: NodeId,
	expected_channels: i32
) -> Option<ModdlResult<NodeId>> {
	// TODO 共通化
	macro_rules! add_node {
		// トラックに属する node は全てトラック名のタグをつける
		($new_node: expr) => {
			Ok(match track {
				Some(track) => nodes.add_node_with_tag(submachine_idx, track.to_string(), $new_node),
				None => nodes.add_node(submachine_idx, $new_node),
			})
		}
	}
	match (input.channels(), expected_channels) {
		(1, 1) => Some(Ok(input)),
		(1, 2) => {
			Some(add_node!(Box::new(MonoToStereo::new(NodeBase::new(nodes.delay(input)), input.node(submachine_idx).as_mono()))))
		},
		(2, 1) => None, // ステレオの入力をモノラルに入れる場合、状況によってすべきことが異なるので、呼び出し元に任せる
		(2, 2) => Some(Ok(input)),
		_ => Some(Err(Error::ChannelMismatch)),
	}
}


fn apply_input(
	track: Option<&str>,
	nodes: &mut AllNodes,
	submachine_idx: MachineIndex,
	fact: &Rc<dyn NodeFactory>,
	max_node_arg_delay: u32,
	node_args: &NodeArgs,
	input: NodeId,
) -> ModdlResult<NodeId> {
	// TODO 共通化
	macro_rules! add_node {
		// トラックに属する node は全てトラック名のタグをつける
		($new_node: expr) => {
			Ok(match track {
				Some(track) => nodes.add_node_with_tag(submachine_idx, track.to_string(), $new_node),
				None => nodes.add_node(submachine_idx, $new_node),
			})
		}
	}

	match coerce_input(track, nodes, submachine_idx, input, fact.input_channels()) {
		Some(result) => {
			let coerced_input = result ?;
			// add_node!(fact.create_node(node_args, coerced_input.node(submachine_idx)))
			let (input_idx, input_delay) = ensure_on_machine(nodes, coerced_input, submachine_idx);
			let max_delay = max_node_arg_delay.max(input_delay);
			add_node!(fact.create_node(NodeBase::new(max_delay), node_args, input_idx))
		},
		None => {
			// 一旦型を明記した変数に取らないとなぜか E0282 になる
			// TODO ここも Some の場合と同様に ensure_on_machine が必要？
			let (input_idx, input_delay) = ensure_on_machine(nodes, input, submachine_idx);
			let input_l = {
				let result: ModdlResult<NodeId> = add_node!(Box::new(
						Split::new(NodeBase::new(input_delay), input_idx.as_stereo(), 0)));
				result ?
			};
			let input_r = {
				let result: ModdlResult<NodeId> = add_node!(Box::new(
						Split::new(NodeBase::new(input_delay), input_idx.as_stereo(), 1)));
				result ?
			};
			let max_delay = max_node_arg_delay.max(input_delay);
			let result_l = {
				let result: ModdlResult<NodeId> = add_node!(
						fact.create_node(NodeBase::new(max_delay), node_args, input_l.node(submachine_idx)));
				result ?
			};
			let result_r = {
				let result: ModdlResult<NodeId> = add_node!(
						fact.create_node(NodeBase::new(max_delay), node_args, input_r.node(submachine_idx)));
				result ?
			};
			add_node!(Box::new(Join::new(NodeBase::new(max_delay), vec![result_l.node(submachine_idx).as_mono(), result_r.node(submachine_idx).as_mono()])))
		}
	}
}

const MACHINE_MAIN: MachineIndex = MachineIndex(0usize);
struct AllNodes {
	machines: Vec<MachineSpec>,
	sends_to_receives: HashMap<NodeId, NodeId>,
	delays: HashMap<NodeId, u32>,
}
impl AllNodes {
	pub fn new() -> Self {
		let mut s = Self {
			machines: vec![],
			sends_to_receives: HashMap::new(),
			delays: HashMap::new(),
		};
		s.add_submachine("main".to_string());
		s
	}
	pub fn add_submachine(&mut self, name: String) -> MachineIndex {
		self.machines.push(MachineSpec { name, nodes: NodeHost::new() });
		let submachine_idx = MachineIndex(self.machines.len() - 1);
		println!("machines[{}]: {}", submachine_idx.0, & self.machines[submachine_idx.0].name);

		submachine_idx
	}
	pub fn add_node(&mut self, machine: MachineIndex, node: Box<dyn Node>) -> NodeId {
		let delay = node.delay_samples();
		let node_idx = self.machines[machine.0].nodes.add(node);
		let result = NodeId::new(machine, node_idx);
		self.delays.insert(result, delay);

		result
	}
	pub fn add_node_with_tags(&mut self, machine: MachineIndex, tags: Vec<String>, node: Box<dyn Node>) -> NodeId {
		let delay = node.delay_samples();
		let node_idx = self.machines[machine.0].nodes.add_with_tags(tags, node);
		let result = NodeId::new(machine, node_idx);
		self.delays.insert(result, delay);

		result
	}
	pub fn add_node_with_tag(&mut self, machine: MachineIndex, tag: String, node: Box<dyn Node>) -> NodeId {
		let delay = node.delay_samples();
		let node_idx = self.machines[machine.0].nodes.add_with_tag(tag, node);
		let result = NodeId::new(machine, node_idx);
		self.delays.insert(result, delay);

		result
	}
	pub fn add_send_receive(&mut self, send: NodeId, receive: NodeId) {
		self.sends_to_receives.insert(send, receive);
	}
	pub fn result(self) -> Vec<MachineSpec> {
		self.machines
	}
	pub fn sends_to_receives(&self) -> &HashMap<NodeId, NodeId> { &self.sends_to_receives }
	pub fn delay(&self, node: NodeId) -> u32 { self.delays[&node] }
	pub fn calc_delay(&self, upstreams: Vec<NodeId>, interthread: bool) -> u32 {
		upstreams.iter().map(|u| self.delay(*u)).max().unwrap()
				+ if interthread { INTERTHREAD_BUFFER_SIZE } else { 0 }
	}
}

const INTERTHREAD_BUFFER_SIZE: u32 = 50;
use crate::node::thread::*;
// use std::thread;
use std::sync::mpsc::sync_channel;

/// 別マシン上の出力を Sender/Receiver を使って持ってくる。同一マシン上の場合はそのまま使う
/// TODO なんかいい名前あれば…
fn ensure_on_machine(nodes: &mut AllNodes, node: NodeId, dest_machine: MachineIndex) -> (ChanneledNodeIndex, u32) {
	if node.machine == dest_machine {
		// 同一マシン上のノードなのでそのまま使える
		(node.node(dest_machine), nodes.delay(node))
		// to_local_idx_and_delay(node)

	} else {
		// 別マシンなので Sender/Receiver で持ってくる
		let (sender, receiver) = sync_channel::<Vec<Sample>>(INTERTHREAD_BUFFER_SIZE as usize);
		// TODO ステレオ対応
		let sender_delay = nodes.calc_delay(vec![node], false);
		let sender_node = nodes.add_node(node.machine, Box::new(Sender::new(
				NodeBase::new(sender_delay),
				node.node_of_any_machine().as_mono(), sender, INTERTHREAD_BUFFER_SIZE as usize)));

		let receiver_delay = nodes.calc_delay(vec![sender_node], true);
		let receiver_node = nodes.add_node(dest_machine, Box::new(Receiver::new(
				NodeBase::new(receiver_delay),
				receiver)));
		nodes.add_send_receive(sender_node, receiver_node);

		(receiver_node.node(dest_machine), nodes.delay(receiver_node))
		// to_local_idx_and_delay(receiver_node)
	}
}

fn create_calc_node(
	track: Option<&str>,
	nodes: &mut AllNodes,
	submachine_idx: MachineIndex,
	arg_nodes: Vec<NodeId>,
	node_factory: &dyn CalcNodeFactoryTrait,
) -> ModdlResult<NodeId> {
	// TODO 共通化
	macro_rules! add_node {
		// トラックに属する node は全てトラック名のタグをつける
		($new_node: expr) => {
			ModdlResult::Ok(match track {
				Some(track) => nodes.add_node_with_tag(submachine_idx, track.to_string(), $new_node),
				None => nodes.add_node(submachine_idx, $new_node),
			})
		}
	}

	// 引数にモノラルとステレオが混在していたらモノラルをステレオに拡張
	// TODO モノラル以外動作確認が不十分…
	enum ChannelCombination { AllMono, AllStereo, MonoAndStereo, Other }
	let any_mono = arg_nodes.iter().any(|n| n.channels() == 1);
	let any_stereo = arg_nodes.iter().any(|n| n.channels() == 2);
	let any_unknown = arg_nodes.iter().any(|n| n.channels() != 1 && n.channels() != 2);
	let comb = if any_unknown { ChannelCombination::Other }
			else if any_mono && any_stereo { ChannelCombination::MonoAndStereo }
			else if any_mono { ChannelCombination::AllMono }
			else { ChannelCombination::AllStereo };
	match comb {
		ChannelCombination::AllMono => {
			let args: Vec<_> = arg_nodes.iter().map(|n| ensure_on_machine(nodes, *n, submachine_idx)).collect();
			let arg_node_idxs = args.iter().map(|a| a.0.as_mono()).collect();
			let delay = args.iter().map(|a| a.1).max().unwrap_or(0);
			add_node!(node_factory.create_mono(NodeBase::new(delay), arg_node_idxs))
		},
		ChannelCombination::AllStereo => {
			let args: Vec<_> = arg_nodes.iter().map(|n| ensure_on_machine(nodes, *n, submachine_idx)).collect();
			let arg_node_idxs = args.iter().map(|a| a.0.as_stereo()).collect();
			let delay = args.iter().map(|a| a.1).max().unwrap_or(0);
			add_node!(node_factory.create_stereo(NodeBase::new(delay), arg_node_idxs))
		},
		ChannelCombination::MonoAndStereo => {
			let mut coerced_arg_nodes: Vec<StereoNodeIndex> = vec![];
			let mut max_delay = 0u32;
			for n in arg_nodes {
				let (node_idx, delay) = ensure_on_machine(nodes, n, submachine_idx);
				max_delay = max_delay.max(delay);
				coerced_arg_nodes.push(if n.channels() == 1 {
					// let mono = ensure_on_machine(nodes, n, submachine_idx).as_mono();
					let stereo = add_node!(Box::new(MonoToStereo::new(NodeBase::new(delay), node_idx.as_mono()))) ?;
					// ensure_on_machine(nodes, stereo, submachine_idx).as_stereo()
					stereo.node(submachine_idx).as_stereo()
				} else {
					// ensure_on_machine(nodes, n, submachine_idx).as_stereo()
					// let (node_idx, delay) = ensure_on_machine(nodes, n, submachine_idx);
					node_idx.as_stereo()
				});
			}
			add_node!(node_factory.create_stereo(NodeBase::new(max_delay), coerced_arg_nodes))
		},
		ChannelCombination::Other => { Err(Error::ChannelMismatch) },
	}
}

macro_rules! binary {
	($name: ident, $calc: ident) => {
		fn $name(track: Option<&str>, nodes: &mut AllNodes, submachine_idx: MachineIndex,
			l_node: NodeId, r_node: NodeId) -> ModdlResult<NodeId> {
				create_calc_node(track, nodes, submachine_idx, vec![l_node, r_node], &CalcNodeFactory::<$calc>::new())
		}
	};
}
binary!(add, AddCalc);
binary!(multiply, MulCalc);
binary!(subtract, SubCalc);
binary!(divide, DivCalc);
binary!(remainder, RemCalc);
binary!(power, PowCalc);
binary!(less, LtCalc);
binary!(less_or_equal, LeCalc);
binary!(equal, EqCalc);
binary!(not_equal, NeCalc);
binary!(greater, GtCalc);
binary!(greater_or_equal, GeCalc);
binary!(and, AndCalc);
binary!(or, OrCalc);
