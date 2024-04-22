use super::{
	builtin::builtin_vars, common::{make_seq_tag, read_file}, error::*, evaluator::*, executor::process_statements, import_cache::ImportCache, io::Io, player_context::{MuteSolo, TrackDef}, player_option::*, scope::*, value::*
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
		feature::Feature,
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
	vis::visualizer::*, wave::waveform_host::WaveformHost,
};
extern crate parser;
use graphviz_rust::attributes::start;
use parser::{
	common::{Location, Span}, mml::default_mml_parser, moddl::{ast::QualifiedLabel, parser::expr}
};

use std::{
	borrow::Borrow, cell::RefCell, collections::hash_map::HashMap, path::Path, rc::Rc, sync::{
		mpsc, Arc
	}, thread
};

// TODO エラー処理を全体的にちゃんとする

const TAG_SEQUENCER: &str = "seq";

pub fn play(options: &PlayerOptions) -> ModdlResult<()> {
	let moddl_path = Path::new(&options.moddl_path);
	let moddl = read_file(moddl_path) ?;
	let sample_rate = 44100; // TODO 値を外から渡せるように
	let root_vars = Scope::root(builtin_vars(sample_rate));
	let mut waveforms = WaveformHost::new();
	let mut imports = ImportCache::new(&mut waveforms);
	let mut pctx = process_statements(moddl.as_str(), root_vars, moddl_path, &mut imports) ?;
	
	// TODO シングルマシン（シングルスレッド）モードは現状これだけだとだめ（Tick が重複してすごい速さで演奏される）
	let mut nodes = AllNodes::new(false);

	// TODO タグ名を sequence_generator と共通化
	let tempo = nodes.add_node_with_tag(MACHINE_MAIN, "#tempo".to_string(), Box::new(Var::new(NodeBase::new(0), pctx.tempo)));
	let timer = nodes.add_node(MACHINE_MAIN, Box::new(TickTimer::new(
			NodeBase::new(nodes.calc_delay(vec![tempo], false)),
			tempo.node(MACHINE_MAIN).as_mono(), pctx.ticks_per_bar, pctx.groove_cycle)))/* .as_mono() */;

	// TODO even groove を誰も使わない場合は省略
	let even_tag = make_seq_tag(None, &mut pctx.seq_tags);
	nodes.add_node(MACHINE_MAIN, Box::new(Tick::new(
			NodeBase::new(nodes.calc_delay(vec![timer], false)),
			timer.node(MACHINE_MAIN).as_mono(), pctx.groove_cycle, even_tag.clone())));

	let mut output_nodes = HashMap::<String, NodeId>::new();

	// for (track, mml) in &pctx.mmls {
	for (track, spec, def_loc) in &pctx.track_defs {
		let submachine_idx = nodes.add_submachine(track.clone());
		let mml = &pctx.mmls.get(track).map(|mml| mml.as_str()).unwrap_or("");
		let output_node = {
			// @mute で指定されているか、@solo で指定されていなければ、ミュート対象
			if pctx.mute_solo_tracks.contains(track) == (pctx.mute_solo == MuteSolo::Mute) {
				Some(nodes.add_node(submachine_idx, Box::new(Constant::new(0f32))))
			} else {
				let seq_tag = match pctx.grooves.get(track) {
					Some((g, _)) => g.clone(),
					None => even_tag.clone(),
				};
				match spec {
					TrackDef::Instrument(structure) => {
						Some(build_nodes_by_mml(track.as_str(), structure, mml, pctx.moddl_path.as_path(), pctx.ticks_per_bar, &seq_tag, &mut nodes, submachine_idx,
								&mut PlaceholderStack::init(HashMap::new()), None, pctx.tempo, timer, pctx.groove_cycle, pctx.use_default_labels, &pctx.vars, &mut imports) ?)
					}
					TrackDef::Effect(source_tracks, structure) => {
						let mut placeholders = PlaceholderStack::init(HashMap::new());
						source_tracks.iter().for_each(|track| {
							placeholders.top_mut().insert(track.clone(), output_nodes[track]);
						});
						Some(build_nodes_by_mml(track.as_str(), structure, mml, pctx.moddl_path.as_path(), pctx.ticks_per_bar, &seq_tag, &mut nodes, submachine_idx,
								&mut placeholders, None, pctx.tempo, timer, pctx.groove_cycle, pctx.use_default_labels, &pctx.vars, &mut imports) ?)
					}
					TrackDef::Groove(structure) => {
						let groovy_timer = build_nodes_by_mml(track.as_str(), structure, mml, pctx.moddl_path.as_path(), pctx.ticks_per_bar, &seq_tag, &mut nodes, MACHINE_MAIN,
								&mut PlaceholderStack::init(HashMap::new()), Some(timer), pctx.tempo, timer, pctx.groove_cycle, pctx.use_default_labels, &pctx.vars, &mut imports)
								?.node(MACHINE_MAIN).as_mono();
						nodes.add_node(MACHINE_MAIN, Box::new(Tick::new(NodeBase::new(0), groovy_timer, pctx.groove_cycle, seq_tag.clone())));

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

	let machine_out = nodes.add_submachine("out".to_string());
	let (master_node, master_delay) = ensure_on_machine(&mut nodes, master, machine_out);

	match &options.output {
		PlayerOutput::Audio => {
			nodes.add_node(machine_out,
					Box::new(PortAudioOut::new(NodeBase::new(master_delay), master_node)));
		},
		PlayerOutput::Wav { path } => {
			// wav ファイルに出力
			nodes.add_node(machine_out,
					Box::new(crate::node::file::WavFileOut::new(NodeBase::new(master_delay), master_node, path.clone())));
		},
		PlayerOutput::Stdout => {
			// stdout に出力
			nodes.add_node(machine_out,
					Box::new(Print::new(NodeBase::new(master_delay), master_node)));
		},
		PlayerOutput::Null => {
			// 出力しない（パフォーマンス計測用）
			nodes.add_node(machine_out,
					Box::new(NullOut::new(NodeBase::new(master_delay), master_node)));
		},
	}

	// TODO タグ名共通化
	nodes.add_node_with_tag(machine_out, "terminator".to_string(),
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

	let broadcast_pairs = make_broadcast_pairs(nodes_result.len());
	let broadcaster = Broadcaster::new(broadcast_pairs.senders);

	// デバッグ用機能なのでとりあえず蓋をしておく
	// TODO コマンドオプションで指定されたときだけ出力する
	// output_structure(&nodes_result, &sends_to_receives);

	let waveforms = Arc::new(waveforms);
	let joins: Vec<_> = nodes_result.into_iter()
			.zip(broadcast_pairs.receivers.into_iter())
			.map(|(mut machine_spec, broadcast_receiver)| {
		let waveforms = Arc::clone(&waveforms);
		let broadcaster_ = broadcaster.clone();
		thread::spawn(move || {
			// TODO skip_mode_events が供給できていない
			let mut machine = Machine::new(machine_spec.name);

			machine.play(&mut Context::new(sample_rate), &mut machine_spec.nodes, &waveforms,
					broadcaster_, broadcast_receiver, None);
		})
	}).collect();
	for j in joins {
		j.join();
	}

	Ok(())
}

struct BroadcastPairs {
	senders: Vec<mpsc::Sender<GlobalEvent>>,
	receivers: Vec<mpsc::Receiver<GlobalEvent>>,
}
fn make_broadcast_pairs(machine_count: usize) -> BroadcastPairs {
	let mut result = BroadcastPairs { senders: vec![], receivers: vec![] };
	for _ in 0 .. machine_count {
		let (s, r) = mpsc::channel();
		result.senders.push(s);
		result.receivers.push(r);
	}

	result
}

fn output_structure(all: &Vec<MachineSpec>, sends_to_receives: &HashMap<NodeId, NodeId>) {
	output_graph(make_graph(all, sends_to_receives));
}

struct EventIter {

}
impl Iterator for EventIter {
	type Item = Box<dyn crate::core::event::Event>;
	fn next(&mut self) -> Option<Box<dyn crate::core::event::Event>> { None }
}

const VAR_DEFAULT_KEY: &str = "value"; // TODO VarFactory を設けてそこから取るようにする

// TODO 引数を整理できるか
fn build_nodes_by_mml<'a>(track: &str, instrm_def: &NodeStructure, mml: &'a str, moddl_path: &Path, ticks_per_bar: i32, seq_tag: &String, nodes: &mut AllNodes, submachine_idx: MachineIndex, placeholders: &mut PlaceholderStack, override_input: Option<NodeId>,
		tempo: f32, timer: NodeId, groove_cycle: i32, use_default_labels: bool, vars: &Rc<RefCell<Scope>>, imports: &mut ImportCache)
		-> ModdlResult<NodeId> {
	let moddl_path_rc = Rc::new(moddl_path.to_path_buf());
	let (_, ast) = default_mml_parser::compilation_unit()(Span::new_extra(mml, moddl_path_rc.clone()))
	.map_err(|e| error(ErrorType::MmlSyntax(nom_error_to_owned(e)), Location::dummy())) ?;
	let freq_tag = format!("{}_freq", track);

	// #22 generate_sequences() に各 Var の初期値が必要になったので、
	// build_instrument() で初期値が判明した後で行うことにしたが、一方 build_instrument() の入力ノードは
	// generate_sequences() によって得られていた features に依存しており、循環依存が発生してしまったので、
	// feature の有無確認を generate_sequences() から切り離して先に行うようにした

	const VELOCITY_INIT: f32 = 1f32;
	const VOLUME_INIT: f32 = 1f32;
	const DETUNE_INIT: f32 = 0f32;
	// let var_default_key = 

	let features = scan_features(&ast);

	let mut input = match override_input {
		Some(input) => input,
		None => nodes.add_node_with_tag(submachine_idx, freq_tag.clone(), Box::new(Var::new(NodeBase::new(0), 0f32))),
	};
	if features.contains(&Feature::Detune) {
		// セント単位のデチューン
		// freq_detuned = freq * 2 ^ (detune / 1200)
		// TODO タグ名は feature requirements として generate_sequences の際に受け取る
		let detune = nodes.add_node_with_tag(submachine_idx, format!("{}.#detune", &track), Box::new(Var::new(NodeBase::new(0), DETUNE_INIT)));
		let cents_per_oct = nodes.add_node(submachine_idx, Box::new(Constant::new(1200f32)));
		let detune_oct = divide(Some(track), nodes, submachine_idx, detune, cents_per_oct) ?; // 必ず成功するはず
		let const_2 = nodes.add_node(submachine_idx, Box::new(Constant::new(2f32)));
		let freq_ratio = power(Some(track), nodes, submachine_idx, const_2, detune_oct) ?; // 必ず成功するはず
		let freq_detuned = multiply(Some(track), nodes, submachine_idx, input, freq_ratio) ?; // 必ず成功するはず
		input = freq_detuned;
	}

	let mut inits: HashMap<(String, String), Sample> = vec![
		((format!("{}.#velocity", &track), VAR_DEFAULT_KEY.to_string()), VELOCITY_INIT),
		((format!("{}.#volume", &track), VAR_DEFAULT_KEY.to_string()), VOLUME_INIT),
		((format!("{}.#detune", &track), VAR_DEFAULT_KEY.to_string()), DETUNE_INIT),
		(("#tempo".to_string(), VAR_DEFAULT_KEY.to_string()), tempo),
	].into_iter().collect();
	let mut label_defaults: HashMap<String, String> = inits.iter().map(|((label, key), _)| (label.clone(), key.clone())).into_iter().collect();
	// TODO DRY
	label_defaults.insert(format!("{}_freq", track), VAR_DEFAULT_KEY.to_string());
	/* let label_defaults =  */collect_label_defaults(instrm_def, track, use_default_labels, &mut label_defaults);
	let instrm = build_instrument(track, instrm_def, nodes, submachine_idx, input, placeholders, &label_defaults, use_default_labels, &mut inits) ?;

	// let label_defaults = collect_label_defaults(instrm_def, track);

	let tag_set = TagSet {
		freq: freq_tag.clone(),
		note: track.to_string(),
	};
	let mut evaluate_expr = |expr_str: &str| {
		// TODO 位置情報の補正が必要
		let (_, expr) = expr()(Span::new_extra(expr_str, moddl_path_rc.clone()))
		.map_err(|e| error(ErrorType::Syntax(nom_error_to_owned(e)), Location::dummy())) ?;
		// match evaluate(&*expr)?.0 {
			
		// }
		// TODO evaluate_and_perform_arg と共通化
		let mut value = evaluate(&*expr, vars, imports) ?;
		while value.as_io().is_ok() {
			let (io, loc) = value.as_io().unwrap();
			value = RefCell::<dyn Io>::borrow_mut(&io).perform(&loc, imports) ?;
		}

		let body = value.0;
		match body {
			ValueBody::Float(f) => Ok(f),
			ValueBody::WaveformIndex(i) => Ok(i.0 as f32),
			_ => Err(error(ErrorType::TypeMismatchAny { expected: vec![
				ValueType::Number,
				ValueType::Waveform,
			]}, value.1.clone()))
		}
	};

	let seqs = generate_sequences(&ast, ticks_per_bar, &tag_set, format!("{}.", &track).as_str(), &inits, &label_defaults, &mut evaluate_expr) ?;
	let _seqr = nodes.add_node_with_tag(MACHINE_MAIN, seq_tag.to_string(), Box::new(Sequencer::new(NodeBase::new(0), track.to_string(), seqs)));

	let mut output = instrm;
	if features.contains(&Feature::Velocity) {
		// TODO タグ名は feature requirements として generate_sequences の際に受け取る
		let vel = nodes.add_node_with_tag(submachine_idx, format!("{}.#velocity", &track), Box::new(Var::new(NodeBase::new(0), VELOCITY_INIT)));
		let output_vel = multiply(Some(track), nodes, submachine_idx, output, vel) ?; // 必ず成功するはず
		output = output_vel;
	}
	if features.contains(&Feature::Volume) {
		// TODO タグ名は feature requirements として generate_sequences の際に受け取る
		let vol = nodes.add_node_with_tag(submachine_idx, format!("{}.#volume", &track), Box::new(Var::new(NodeBase::new(0), VOLUME_INIT)));
		let output_vol = multiply(Some(track), nodes, submachine_idx, output, vol) ?; // 必ず成功するはず
		output = output_vol;
	}

	let tick_delay = 0; // TODO 仮（遅延管理は廃止の方向）
	nodes.set_driver_delay(submachine_idx, tick_delay);

	Ok(output)
}

fn collect_label_defaults(instrm_def: &NodeStructure, track: &str, use_default_labels: bool, result: &mut HashMap<String, String>) /* -> HashMap<String, String> */ {
	fn visit_struct(strukt: &NodeStructure, track: &str, use_default_labels: bool, result: &mut HashMap<String, String>) {
		match strukt {
			NodeStructure::NodeCreation { factory, args, label } => {
				for (_, (arg, _)) in args {
					if let ValueBody::NodeStructure(arg) = arg {
						visit_struct(arg, track, use_default_labels, result);
					}
				}
				if let (Some(label), Some(default_key)) = (label, factory.default_prop_key()) {
					// TODO ラベル名をトラック名で修飾する処理は共通化する
					result.insert(format!("{}.{}", track, label.0), default_key.clone());
				}
				// 互換性対応：全て Var と見なす
				if use_default_labels {
					for arg_spec in factory.node_arg_specs() {
						// TODO ラベル名をトラック名で修飾する処理は共通化する
						result.insert(format!("{}.{}", track, arg_spec.name), VAR_DEFAULT_KEY.to_string());
					}
				}
			},
			NodeStructure::Calc { args, .. } => {
				for arg in args { visit_struct(arg, track, use_default_labels, result); }
			},
			NodeStructure::Connect(lhs, rhs) => {
				visit_struct(lhs, track, use_default_labels, result);
				visit_struct(rhs, track, use_default_labels, result);
			},
			NodeStructure::Condition { cond, then, els } => {
				visit_struct(cond, track, use_default_labels, result);
				visit_struct(then, track, use_default_labels, result);
				visit_struct(els, track, use_default_labels, result);
			},
			NodeStructure::Lambda { body, .. } => {
				visit_struct(body, track, use_default_labels, result);
			},
			NodeStructure::Constant { label, .. } => {
				if let Some(label) = label {
					// TODO ラベル名をトラック名で修飾する処理は共通化する
					// TODO VarFactory から取った方が統一感ある
					result.insert(format!("{}.{}", track, label.0), VAR_DEFAULT_KEY.to_string());
				}
			},
			NodeStructure::Placeholder { .. } => { },

		}
	}

	// let mut result = HashMap::new();
	visit_struct(instrm_def, track, use_default_labels, result);

	// result
}

pub type PlaceholderStack = Stack<HashMap<String, NodeId>>;

fn build_instrument(track: &str, instrm_def: &NodeStructure, nodes: &mut AllNodes, submachine_idx: MachineIndex, freq: NodeId, placeholders: &mut PlaceholderStack, label_defaults: &HashMap<String, String>, use_default_labels: bool, inits: &mut HashMap<ParamSignature, f32>) -> ModdlResult<NodeId> {
	fn visit_struct(track: &str, strukt: &NodeStructure, nodes: &mut AllNodes, submachine_idx: MachineIndex, input: NodeId, default_tag: Option<QualifiedLabel>, placeholders: &mut PlaceholderStack, label_defaults: &HashMap<String, String>, use_default_labels: bool, inits: &mut HashMap<ParamSignature, f32>) -> ModdlResult<NodeId> {
		// 関数にするとライフタイム関係？のエラーが取れなかったので…
		macro_rules! recurse {
			// $const_tag は、直下が定数値（ノードの種類としては Var）であった場合に付与するタグ
			($strukt: expr, $input: expr, $const_tag: expr) => { visit_struct(track, $strukt, nodes, submachine_idx, $input, /* Some( */$const_tag/* ) */, placeholders, label_defaults, use_default_labels, inits) };
			($strukt: expr, $input: expr) => { visit_struct(track, $strukt, nodes, submachine_idx, $input, None, placeholders, label_defaults, use_default_labels, inits) };
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
					// arg_val.1.as_node_structure()
					// 		// node_args に指定された引数なのに NodeStructure に変換できない
					// 		.ok_or_else(|| error(ErrorType::NodeFactoryNotFound, Location::dummy())) ?

					// 変更前のコード↑では NodeFactoryNotFound だが、変更後↓は TypeMismatch になる。TypeMismatch でよくない？
					arg_val.1.as_node_structure().map(|v| v.0)?
				} else if let Some(default) = default {
					ValueBody::Float(default).as_node_structure().unwrap()
				} else {
					// 必要な引数が与えられていない
					Err(error(ErrorType::NodeFactoryNotFound, Location::dummy())) ?
				};
				// ラベルが明示されていればそちらを使う
				let arg_name = arg_val.map(|(_, (value, _))| value.label()).flatten()
						.or_else(|| if use_default_labels { Some(QualifiedLabel(name.clone())) } else { None })/* .unwrap_or(name.clone()) */;
				let arg_node = recurse!(&strukt, input, arg_name) ?;
				let coerced_arg_node = match coerce_input(Some(track), nodes, submachine_idx, arg_node, channels) {
					Some(result) => result,
					// モノラルであるべき node_arg にステレオが与えられた場合、
					// 勝手にモノラルに変換するとロスが発生するのでエラーにする
					None => Err(error(ErrorType::ChannelMismatch, Location::dummy())),
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
			// 	let fact = factories.get(id).ok_or_else(|| ErrorType::NodeFactoryNotFound) ?;
			// 	apply_input(Some(track), nodes, fact, &ValueArgs::new(), &NodeArgs::new(), input)
			// },
			NodeStructure::NodeCreation { factory, args, label } => {
				let (node_args, delay) = make_node_args(args, factory) ?;

				let local_tag = label.as_ref().or(default_tag.as_ref());
				// TODO 共通化
				let full_tag = local_tag.map(|tag| format!("{}.{}", track, tag.0));
				if let Some(tag) = &full_tag {
					for (key, value) in factory.initial_values() {
						inits.insert((tag.clone(), key), value);
					}
				}

				apply_input(Some(track), nodes, submachine_idx, factory, delay, &node_args, full_tag,input)
			}
			// TODO Constant は、NodeCreation で VarFactory を使ったのと同じにできるはず。共通化する
			NodeStructure::Constant { value, label } => {
				let node = Box::new(Var::new(NodeBase::new(0), *value));
				let local_tag = label.as_ref().or(default_tag.as_ref());
				// TODO 共通化
				let full_tag = local_tag.map(|tag| format!("{}.{}", track, tag.0));
				// dbg!(label, &default_tag, &local_tag, &full_tag);
				match full_tag {
					Some(tag) => {
						// TODO ここで label_defaults から見つからないことはありえないはずだが、補足できるエラー（内部エラー的な）として軟着陸させた方がよさそう
						let default = label_defaults.get(&tag).unwrap();
						inits.insert((tag.clone(), default.clone()), *value);
						Ok(nodes.add_node_with_tags(submachine_idx, vec![track.to_string(), tag], node))
					},
					None => add_node!(node),
				}
				
			},
			NodeStructure::Placeholder { name } => {
				// 名前に対応する placeholder は必ずある
				Ok(placeholders.top()[name])
			},
		}
	}

	visit_struct(track, instrm_def, nodes, submachine_idx, freq, None, placeholders, label_defaults, use_default_labels, inits)
}

// fn create_node_by_factory(factory: &Rc<dyn NodeFactory>, args: &HashMap<String, Value>) {
// 	let (node_args, delay) = make_node_args(args, factory) ?;

// 	let local_tag = label.as_ref().or(default_tag.as_ref());
// 	// TODO 共通化
// 	let full_tag = local_tag.map(|tag| format!("{}.{}", track, tag.clone()));
// 	if let Some(tag) = &full_tag {
// 		for (key, value) in factory.initial_values() {
// 			inits.insert((tag.clone(), key), value);
// 		}
// 	}

// 	apply_input(Some(track), nodes, submachine_idx, factory, delay, &node_args, full_tag,input)
// }

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
		_ => Some(Err(error(ErrorType::ChannelMismatch, Location::dummy()))),
	}
}


fn apply_input(
	track: Option<&str>,
	nodes: &mut AllNodes,
	submachine_idx: MachineIndex,
	fact: &Rc<dyn NodeFactory>,
	max_node_arg_delay: u32,
	node_args: &NodeArgs,
	label: Option<String>,
	input: NodeId,
) -> ModdlResult<NodeId> {
	// TODO 共通化
	macro_rules! add_node {
		// トラックに属する node は全てトラック名のタグをつける
		($label: expr, $new_node: expr) => {
			{
				let label: &Option<String> = &$label;
				// let mut add_node = |is_labeled_node, new_node| Ok::<NodeId, Error>({
				let mut tags: Vec<String> = vec![];
				if let Some(full_tag) = label { tags.push(full_tag.clone()); }
				if let Some(track) = track { tags.push(track.to_string()); }

				Ok(nodes.add_node_with_tags(submachine_idx, tags, $new_node))
			}
		}
	}

	match coerce_input(track, nodes, submachine_idx, input, fact.input_channels()) {
		Some(result) => {
			let coerced_input = result ?;
			// add_node!(fact.create_node(node_args, coerced_input.node(submachine_idx)))
			let (input_idx, input_delay) = ensure_on_machine(nodes, coerced_input, submachine_idx);
			let max_delay = max_node_arg_delay.max(input_delay);
			add_node!(label, fact.create_node(NodeBase::new(max_delay), node_args, input_idx))
		},
		None => {
			// 一旦型を明記した変数に取らないとなぜか E0282 になる
			// TODO ここも Some の場合と同様に ensure_on_machine が必要？
			let (input_idx, input_delay) = ensure_on_machine(nodes, input, submachine_idx);
			let input_l = {
				let result: ModdlResult<NodeId> = add_node!(None, Box::new(
						Split::new(NodeBase::new(input_delay), input_idx.as_stereo(), 0)));
				result ?
			};
			let input_r = {
				let result: ModdlResult<NodeId> = add_node!(None, Box::new(
						Split::new(NodeBase::new(input_delay), input_idx.as_stereo(), 1)));
				result ?
			};
			let max_delay = max_node_arg_delay.max(input_delay);
			let result_l = {
				let result: ModdlResult<NodeId> = add_node!(label, 
						fact.create_node(NodeBase::new(max_delay), node_args, input_l.node(submachine_idx)));
				result ?
			};
			let result_r = {
				let result: ModdlResult<NodeId> = add_node!(label, 
						fact.create_node(NodeBase::new(max_delay), node_args, input_r.node(submachine_idx)));
				result ?
			};
			add_node!(None, Box::new(Join::new(NodeBase::new(max_delay), vec![result_l.node(submachine_idx).as_mono(), result_r.node(submachine_idx).as_mono()])))
		}
	}
}

const MACHINE_MAIN: MachineIndex = MachineIndex(0usize);
struct AllNodes {
	single_machine: bool,
	machines: Vec<MachineSpec>,
	sends_to_receives: HashMap<NodeId, NodeId>,
	delays: HashMap<NodeId, u32>,

	/// マシンごとに、そのマシン内の各ノードをイベントで駆動するノード（要は Tick）の遅延数。
	/// 遅延管理のために設けたが、結局 Machine で遅延補償は行っておらず（行うとかえっておかしくなる）、
	/// 不要かもしれない
	driver_delays: HashMap<MachineIndex, u32>,
}
impl AllNodes {
	pub fn new(single_machine: bool) -> Self {
		let mut s = Self {
			single_machine,
			machines: vec![],
			sends_to_receives: HashMap::new(),
			delays: HashMap::new(),
			driver_delays: HashMap::new(),
		};
		s.add_submachine("main".to_string());
		s
	}
	pub fn add_submachine(&mut self, name: String) -> MachineIndex {
		if self.single_machine && self.machines.len() > 0 {
			return MachineIndex(0);
		}

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
	pub fn set_driver_delay(&mut self, machine: MachineIndex, delay: u32) {
		let delay = self.driver_delays.get(&machine).unwrap_or(&0u32).max(&delay);
		self.driver_delays.insert(machine, *delay);
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
		debug_assert!(upstreams.len() > 0);
		let upstream_delay = upstreams.iter().map(|u| self.delay(*u)).max().unwrap();

		if interthread {
			let upstream_machine = upstreams[0].machine;
			// let driver_delay = * self.driver_delays.get(&upstream_machine).unwrap_or(&0u32);
			let driver_delay = 0u32;
			upstream_delay.max(driver_delay) + INTERTHREAD_BUFFER_SIZE

		} else {
			upstream_delay
		}
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
		let (sender, receiver) = sync_channel::<Vec<Sample>>(0);
		// TODO ステレオ対応
		let sender_delay = nodes.calc_delay(vec![node], false);
		let sender_node = nodes.add_node(node.machine, Box::new(Sender::new(
				NodeBase::new(sender_delay),
				node.node_of_any_machine(), sender, INTERTHREAD_BUFFER_SIZE as usize)));

		let receiver_delay = nodes.calc_delay(vec![sender_node], true);
		let receiver_node = nodes.add_node(dest_machine, Box::new(Receiver::new(
				NodeBase::new(receiver_delay),
				node.node_of_any_machine().channels(),
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
		ChannelCombination::Other => { Err(error(ErrorType::ChannelMismatch, Location::dummy())) },
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
