use super::{
	builtin::builtin_vars, common::{make_seq_tag, read_file}, error::*, evaluator::*, executor::process_statements, import::ImportCache, io::Io, player_context::{MuteSolo, TrackDef}, player_option::*, scope::*, value::*
};
use crate::{
	calc::*, common::stack::*, core::{
		common::*,
		context::*,
		event::*,
		machine::*,
		node::*,
		node_factory::*,
		node_host::*,
	}, mml::default::{
		feature::Feature,
		sequence_generator::*,
	}, moddl::player_context::PlayerContext, node::{
		audio::*, cond::*, ipc::MessageReceiver, prim::*, stereo::*, system::*, util::*, var::*
	}, seq::{
		sequencer::*,
		tick::*,
	}, vis::visualizer::*, wave::waveform_host::WaveformHost
};
extern crate parser;
use bson::Document;
use ipc::{Client, RegisterSettings, RegisterSettingsItem, Response, Server, Set, channel_name_c2p, channel_name_p2c, to_bson};
use itertools::Itertools;
use parser::{
	common::{Location, Span}, mml::default_mml_parser, moddl::{ast::QualifiedLabel, parser::expr}
};
use serde::{Deserialize, Serialize};

use std::{
	borrow::Borrow, cell::RefCell, collections::{BTreeMap, hash_map::HashMap}, mem::uninitialized, path::Path, rc::Rc, sync::{
		Arc, mpsc::{self, SyncSender}
	}, thread, time::Duration
};

// TODO コード整理

// TODO エラー処理を全体的にちゃんとする

const TAG_SEQUENCER: &str = "seq";

const TAG_FREQ: &str = "#freq";

// type RegisterInits = Vec<(String, f32)>;
type RegisterSettingsSubtree = BTreeMap<String, RegisterSettingsTreeNode>;
#[derive(Debug, Serialize, Deserialize)]
struct RegisterSettingsTree(pub RegisterSettingsSubtree);
#[derive(Debug, Serialize, Deserialize)]
enum RegisterSettingsTreeNode {
	Settings {
		// 今のところ複数のキーの初期値を持つことはない。
		// 複数組持つ必要がある場合は key と init を HashMap<String, f32> などで持つようにすれば問題ないはず
		reg_key: String,
		init: f32,
		domain: Option<DomainHint>,
	},
	Group(RegisterSettingsSubtree),
}
impl RegisterSettingsTree {
	pub fn new() -> Self { Self(BTreeMap::new()) }
	pub fn add_settings(&mut self, path: &QualifiedLabel/* name: &str */, reg_key: impl Into<String>, init: f32, domain: Option<DomainHint>) /* -> &mut Self */ {
		// self
		// let iter = path.0.split(".");
		let path_elems: Vec<&str> = path.elems().collect();
		/* self. */Self::add_settings_iter(&mut self.0, &path_elems, reg_key.into(), init, domain);
	}
	fn add_settings_iter(tree: &mut RegisterSettingsSubtree, path: &[&str], reg_key: String, init: f32, domain: Option<DomainHint>) {
		match path.len() {
			0 => {
				unreachable!();
			},
			1 => {
				let key = path[0].to_string();
				tree.insert(key, RegisterSettingsTreeNode::Settings { reg_key, init, domain });
			}, 
			_ => {
				let key = path[0].to_string();
				match tree.get_mut(&key) {
					None => {
						let mut subtree = RegisterSettingsSubtree::new();
						Self::add_settings_iter(&mut subtree, &path[1 ..], reg_key, init, domain);
						tree.insert(key, RegisterSettingsTreeNode::Group(subtree));
					},
					Some(RegisterSettingsTreeNode::Settings { .. }) => {
						// TODO エラーにする
						println!("(x_x)");
					},
					Some(RegisterSettingsTreeNode::Group(subtree)) => {
						Self::add_settings_iter(subtree, &path[1 ..], reg_key, init, domain);
					},
				}
			}
		}
	}
}

fn qualify_tag(track: &str, tag: &str) -> QualifiedLabel {
	// format!("{}.{}", track, tag)
	QualifiedLabel::new(vec![track], tag)
}

// fn add_register(reg_inits: &mut RegisterInits, nodes: &mut AllNodes, machine: MachineIndex, tag: String, value: f32, domain_hint: Option<DomainHint>) -> NodeId {
// 	reg_inits.push((tag.clone(), value));

// 	nodes.add_node_with_tag(machine, tag, Box::new(Var::new(value)))
// }
fn add_register(reg_inits: &mut RegisterSettingsTree, nodes: &mut AllNodes, machine: MachineIndex, path: QualifiedLabel, reg_key: impl Into<String>, value: f32, domain_hint: Option<DomainHint>) -> NodeId {
	// reg_inits.push((tag.clone(), value));
	reg_inits.add_settings(&path, reg_key, value, domain_hint);

	// TODO QLabel をどこまで QLabel のまま持っていくか？ とりあえずここで String に変換しておくが
	nodes.add_node_with_tag(machine, path.to_string(), Box::new(Var::new(value)))
}

// fn add_registers(reg_inits: &mut RegisterInits, nodes: &mut AllNodes, machine: MachineIndex, tags: Vec<String>, value: f32) -> NodeId {
// 	reg_inits.extend(tags.iter().cloned().map(|tag| (tag, value)));

// 	nodes.add_node_with_tags(machine, tags, Box::new(Var::new(value)))
// }

pub fn play(options: &PlayerOptions) -> ModdlResult<()> {
	let moddl_path = Path::new(&options.moddl_path);
	let moddl = read_file(moddl_path) ?;
	let sample_rate = 44100; // TODO 値を外から渡せるように
	let mut waveforms = WaveformHost::new();
	let mut imports = ImportCache::new(&mut waveforms, options.dump_ast);
	let root_vars = Scope::root(builtin_vars(sample_rate, &mut imports) ?);
	let mut pctx = process_statements(moddl.as_str(), root_vars, moddl_path, &mut imports) ?;
	// let mut reg_inits = vec![];
	let mut reg_inits = RegisterSettingsTree::new();

	if let Some(asts) = imports.asts() {
		println!("{}", serde_json::to_string(asts).unwrap());
	}

	// TODO シングルマシン（シングルスレッド）モードは現状これだけだとだめ（Tick が重複してすごい速さで演奏される）
	let mut nodes = AllNodes::new(false);

	// TODO タグ名を sequence_generator と共通化
	let tempo = add_register(&mut reg_inits, &mut nodes, MACHINE_MAIN, QualifiedLabel::local("#tempo"), VAR_DEFAULT_KEY, pctx.tempo,
			Some(DomainHint::range_including_both_ends(20f32, 300f32)));
	let timer = nodes.add_node(MACHINE_MAIN, Box::new(TickTimer::new(
			tempo.node(MACHINE_MAIN).as_mono(), pctx.ticks_per_bar, pctx.groove_cycle)))/* .as_mono() */;

	// TODO even groove を誰も使わない場合は省略
	let even_tag = make_seq_tag(None, &mut pctx.seq_tags);
	nodes.add_node(MACHINE_MAIN, Box::new(Tick::new(
			timer.node(MACHINE_MAIN).as_mono(), pctx.groove_cycle, even_tag.clone())));

	nodes.add_node(MACHINE_MAIN, Box::new(MessageReceiver::new(channel_name_c2p())));

	let mut output_nodes = HashMap::<String, NodeId>::new();

	// for (track, mml) in &pctx.mmls {
	for (track, spec, _) in &pctx.track_defs {
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
						Some(build_nodes_by_mml(track.as_str(), structure, mml, &pctx, &seq_tag, &mut nodes, submachine_idx,
								&mut PlaceholderStack::init(HashMap::new()), None, &mut imports, &mut reg_inits) ?)
					}
					TrackDef::Effect(source_tracks, structure) => {
						let mut placeholders = PlaceholderStack::init(HashMap::new());
						source_tracks.iter().for_each(|track| {
							placeholders.top_mut().insert(track.clone(), output_nodes[track]);
						});
						Some(build_nodes_by_mml(track.as_str(), structure, mml, &pctx, &seq_tag, &mut nodes, submachine_idx,
								&mut placeholders, None, &mut imports, &mut reg_inits) ?)
					}
					TrackDef::Groove(structure) => {
						let groovy_timer = build_nodes_by_mml(track.as_str(), structure, mml, &pctx, &seq_tag, &mut nodes, MACHINE_MAIN,
								&mut PlaceholderStack::init(HashMap::new()), Some(timer), &mut imports, &mut reg_inits)
								?.node(MACHINE_MAIN).as_mono();
						nodes.add_node(MACHINE_MAIN, Box::new(Tick::new(groovy_timer, pctx.groove_cycle, seq_tag.clone())));

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

// 	let domain_hints = collect_domain_hints(& pctx.track_defs);
// dbg!(&domain_hints);return Ok(());
	// ipc::codec::encode(&domain_hints);

	if options.no_play { return Ok(()); }

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
	let master_node = ensure_on_machine(&mut nodes, master, machine_out);

	dbg!(&reg_inits);

	match &options.output {
		PlayerOutput::Audio => {
			nodes.add_node(machine_out,
					Box::new(PortAudioOut::new(master_node)));
		},
		PlayerOutput::Wav { path } => {
			// wav ファイルに出力
			nodes.add_node(machine_out,
					Box::new(crate::node::file::WavFileOut::new(master_node, path.clone())));
		},
		PlayerOutput::Stdout => {
			// stdout に出力
			nodes.add_node(machine_out,
					Box::new(Print::new(master_node)));
		},
		PlayerOutput::Null => {
			// 出力しない（パフォーマンス計測用）
			nodes.add_node(machine_out,
					Box::new(NullOut::new(master_node)));
		},
	}

	// TODO タグ名共通化
	nodes.add_node_with_tag(machine_out, "terminator".to_string(),
			Box::new(Terminator::new(master_node, pctx.track_defs.len())));

	// 一定時間で終了
	// TODO コマンドオプションで指定できるように
	// let mut sched = crate::node::event_scheduler::EventScheduler::new();
	// sched.add_event(60 * 44100, Box::new(TerminateEvent { }));
	// nodes.add(Box::new(sched));

	// let seq_tags = pctx.seq_tags.clone(); // TODO 本来 clone 不要のはず
	// skip 時にメインループの代わりに tick を提供する関数
	// let skip_mode_events: Box<dyn Fn () -> Vec<Box<dyn Event>>> = Box::new(move || {
	// 	// 型がうまく合わないのでやむを得ずループで書く
	// 	//  seq_tags.iter().map(|tag| Box::<dyn Event>::new(TickEvent::new(EventTarget::Tag(tag.clone())))).collect::<Vec<Box<dyn Event>>>()
	// 	let mut events: Vec<Box<dyn Event>> = vec![];
	// 	for tag in &seq_tags {
	// 		let target = EventTarget::Tag(tag.clone());
	// 		events.push(Box::new(TickEvent::new(target)));
	// 	}
	// 	events
	// });

	let nodes_result = nodes.result();

	let broadcast_pairs = make_broadcast_pairs(nodes_result.len());
	let broadcaster = Broadcaster::new(broadcast_pairs.senders);

	// iceoryx2 は Node を作成すると SIGINT/SIGTERM のハンドリングを奪ってしまい、
	// Ctrl+C を押しても OS 標準の即時終了が起きなくなる。
	// そこで終了要求を自前で検知し、曲が終わったときと同じ経路（TerminateEvent の
	// ブロードキャスト）で各マシンを終了させる。
	{
		let broadcaster_for_termination = broadcaster.clone();
		thread::spawn(move || {
			match ipc::TerminationWatcher::new() {
				Ok(watcher) => {
					watcher.block_until_termination_requested(Duration::from_millis(100));
					broadcaster_for_termination.broadcast(GlobalEvent::new(0, Box::new(TerminateEvent {})));
				}
				// TODO エラー処理
				Err(e) => { dbg!(&e); }
			}
		});
	}

	let bound = 0usize; // TODO これでいいか？
	let (p2c_sender, p2c_receiver) = sync_channel::<Vec<u8>>(bound);
	thread::spawn(move || {
		let p2c_client = Client::new(channel_name_p2c(), Duration::from_millis(500)).unwrap();
		loop {
			match p2c_receiver.recv() {
				Ok(request_bytes) => {
					match p2c_client.send_request(& request_bytes) {
						Ok(response) => {
							println!("received response: {}", to_bson(&response).map(|doc| doc.to_string()).unwrap_or_else(|_| "<error parsing bson>".into()));
						},
						Err(e) => {
							println!("error sending request: {}", &e);
						},
					}
				}
				// TODO ちゃんとエラー処理
				Err(e) => todo!(),
			}
		}
	});

	let msg = make_register_settings_message(&reg_inits);
	send_message(&p2c_sender, &msg);

	// デバッグ用機能なのでとりあえず蓋をしておく
	// TODO コマンドオプションで指定されたときだけ出力する
	// output_structure(&nodes_result, &sends_to_receives);

	let waveforms = Arc::new(waveforms);
	let joins: Vec<_> = nodes_result.into_iter()
			.zip(broadcast_pairs.receivers.into_iter())
			.map(|(mut machine_spec, broadcast_receiver)| {
		let waveforms = Arc::clone(&waveforms);
		let broadcaster_ = broadcaster.clone();
		let p2c_sender_ = p2c_sender.clone();
		thread::spawn(move || {
			// TODO skip_mode_events が供給できていない
			let mut machine = Machine::new(machine_spec.name);

			machine.play(&mut Context::new(sample_rate), &mut machine_spec.nodes, &waveforms,
					broadcaster_, broadcast_receiver, None, p2c_sender_);
		})
	}).collect();
	for j in joins {
		// TODO エラー処理
		let _ = j.join();
	}

	Ok(())
}

fn make_register_settings_message(reg_inits: &RegisterSettingsTree) -> ipc::RegisterSettings {
	fn transform_subtree(subtree: &RegisterSettingsSubtree) -> BTreeMap<String, ipc::RegisterSettingsItem> {
		subtree.iter().map(|(name, node)| {
			let ipc_node = match node {
				RegisterSettingsTreeNode::Settings { reg_key, init, domain } => {
					ipc::RegisterSettingsItem::Settings {
						key: reg_key.clone(),
						initial: *init,
						// TODO controller によるオーバーライドを反映
						original: *init,
						domain: domain.as_ref().map(|domain| match domain {
							DomainHint::Range { min, includes_min, max, includes_max } => {
								ipc::DomainHint::Range {
									min: *min,
									includes_min: *includes_min,
									max: *max,
									includes_max: *includes_max,
								}
							},
							DomainHint::Enum { items } => {
								ipc::DomainHint::Enum {
									items: items.iter().map(|EnumItem { value, name }| ipc::EnumItem {
										value: *value,
										name: name.clone(),
									}).collect(),
								}
							}
						}),
					}
				},
				RegisterSettingsTreeNode::Group(subtree) => {
					ipc::RegisterSettingsItem::Group(transform_subtree(subtree))
				}
			};
			(name.clone(), ipc_node)
		}).collect()
	};

	ipc::RegisterSettings {
		items: transform_subtree(&reg_inits.0),
	}
}

fn send_message(p2c_sender: &SyncSender<Vec<u8>>,  message: &impl ipc::Message) {
	p2c_sender.send(ipc::encode(message)).unwrap_or_else(|x| {dbg!(&x);});
	// TODO エラー処理？
}

// fn collect_domain_hints(track_defs: &Vec<(String, TrackDef, Location)>) -> DomainTree {
// 	track_defs.iter().map(|(track, track_def, _)| {
// 		let def = match track_def {
// 			TrackDef::Instrument(def) => def,
// 			TrackDef::Effect(_, def) => def,
// 			TrackDef::Groove(def) => def,
// 		};
// 		(QualifiedLabel(track.clone()), DomainTreeNode::Group(collect_domain_hints_(def, None)))
// 	}).collect()
// }

// fn collect_domain_hints_(def: &ModuleDef, domain: Option<&DomainHint>) -> DomainTree {
// 	macro_rules! collect_without_domain_and_merge {
// 		($coll: expr) => {
// 			$coll.iter().map(|arg| collect_domain_hints_(&*arg, None)).fold(
// 				DomainTree::new(),
// 				|mut accum, result| { accum.extend(result.into_iter()); accum },
// 			)
// 		};
// 	}
// 	match def {
// 		ModuleDef::Calc { args, .. } => {
// 			collect_without_domain_and_merge!(args)
// 		},
// 		ModuleDef::Condition { cond, then, els } => {
// 			collect_without_domain_and_merge!([cond, then, els])
// 		},
// 		ModuleDef::Connect(lhs, rhs) => {
// 			collect_without_domain_and_merge!([lhs, rhs])
// 		},
// 		ModuleDef::Constant { value, label } => {
// 			// この値に label がついており、かつモジュールのパラメータに直結されていれば、定義域をパラメータから取得する
// 			let mut result = DomainTree::new();
// 			if let (Some(label), Some(domain)) = (label, domain) {
// 				result.insert(label.clone(), DomainTreeNode::Domain(domain.clone()));
// 			}
// 			result
// 		},
// 		ModuleDef::NodeCreation { factory, args, label } => {
// 			args.iter().map(|(arg_name, arg_value)| {
// 				// パラメータ名に割り当たった定義域ヒント（ないかもしれない）
// 				let domain = factory.domains.get(arg_name);
// 				// パラメータの値は必ず ModuleDef のはず
// 				arg_value.as_module_def().map_or(DomainTree::new(),
// 						|(arg_value, _)| collect_domain_hints_(&arg_value, domain))
// 			}).fold(
// 				DomainTree::new(),
// 				|mut accum, result| { accum.extend(result.into_iter()); accum },
// 			)
// 		},
// 		ModuleDef::LabelGuard(_) => { DomainTree::new() },
// 		ModuleDef::Lambda { input_param, body } => {
// 			collect_without_domain_and_merge!([body])
// 		},
// 		ModuleDef::Placeholder { name } => { DomainTree::new() },
// 	}
// }

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

fn build_nodes_by_mml<'a>(track: &str, instrm_def: &ModuleDef, mml: &'a str, pctx: &PlayerContext, seq_tag: &String, nodes: &mut AllNodes, submachine_idx: MachineIndex, placeholders: &mut PlaceholderStack, override_input: Option<NodeId>,
		imports: &mut ImportCache, reg_inits: &mut RegisterSettingsTree)
		-> ModdlResult<NodeId> {
	let moddl_path_rc = Rc::new(pctx.moddl_path.to_path_buf());
	let (_, ast) = default_mml_parser::compilation_unit()(Span::new_extra(mml, moddl_path_rc.clone()))
	.map_err(|e| error(ErrorType::MmlSyntax(nom_error_to_owned(e)), Location::dummy())) ?;
	// let freq_qual_tag = qualify_tag(track, TAG_FREQ);
	let freq_qual_tag = QualifiedLabel::new(vec![track], TAG_FREQ);

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
		None => add_register(reg_inits, nodes, submachine_idx, freq_qual_tag.clone(), VAR_DEFAULT_KEY, 0f32, None),
	};
	if features.contains(&Feature::Detune) {
		// セント単位のデチューン
		// freq_detuned = freq * 2 ^ (detune / 1200)
		// TODO タグ名は feature requirements として generate_sequences の際に受け取る
		// let detune = add_register(reg_inits, nodes, submachine_idx, QualifiedLabel(format!("{}.#detune", &track)), DETUNE_INIT,
		// 		Some(DomainHint::range_including_both_ends(-100f32, 100f32)));
		let detune = add_register(reg_inits, nodes, submachine_idx, QualifiedLabel::new(vec![track.to_string()], "#detune".to_string()), VAR_DEFAULT_KEY, DETUNE_INIT,
				Some(DomainHint::range_including_both_ends(-100f32, 100f32)));
		let cents_per_oct = nodes.add_node(submachine_idx, Box::new(Constant::new(1200f32)));
		let detune_oct = divide(Some(track), nodes, submachine_idx, detune, cents_per_oct) ?; // 必ず成功するはず
		let const_2 = nodes.add_node(submachine_idx, Box::new(Constant::new(2f32)));
		let freq_ratio = power(Some(track), nodes, submachine_idx, const_2, detune_oct) ?; // 必ず成功するはず
		let freq_detuned = multiply(Some(track), nodes, submachine_idx, input, freq_ratio) ?; // 必ず成功するはず
		input = freq_detuned;
	}

	let mut label_defaults: HashMap<QualifiedLabel, String> = vec![
		(qualify_tag(&track, "#velocity"), VAR_DEFAULT_KEY.to_string()),
		(qualify_tag(&track, "#volume"), VAR_DEFAULT_KEY.to_string()),
		(qualify_tag(&track, "#detune"), VAR_DEFAULT_KEY.to_string()),
		(qualify_tag(&track, TAG_FREQ), VAR_DEFAULT_KEY.to_string()),
		(QualifiedLabel::local("#tempo"), VAR_DEFAULT_KEY.to_string()),
	].into_iter().collect();
	collect_label_defaults(instrm_def, track, pctx.use_default_labels, &mut label_defaults);
	let instrm = build_instrument(track, instrm_def, nodes, submachine_idx, input, placeholders, &label_defaults, pctx.use_default_labels, /* &mut inits, */ reg_inits) ?;

	// let label_defaults = collect_label_defaults(instrm_def, track);

	let tag_set = TagSet {
		// TODO シーケンスも QLabel で書き直した方がいいかも
		freq: freq_qual_tag,
		note: track.to_string(),
	};
	let mut evaluate_expr = |expr_str: &str| {
		// TODO 位置情報の補正が必要
		let (_, expr) = expr()(Span::new_extra(expr_str, moddl_path_rc.clone()))
		.map_err(|e| error(ErrorType::Syntax(nom_error_to_owned(e)), Location::dummy())) ?;
		// match evaluate(&*expr)?.0 {
			
		// }
		// TODO evaluate_and_perform_arg と共通化
		let mut value = evaluate(&*expr, &pctx.vars, imports) ?;
		while value.as_io().is_ok() {
			let (io, loc) = value.as_io().unwrap();
			value = RefCell::<dyn Io>::borrow_mut(&io).perform(&loc, imports) ?;
		}

		let body = value.0;
		match body {
			ValueBody::Number(f) => Ok(f),
			ValueBody::WaveformIndex(i) => Ok(i.0 as f32),
			_ => Err(error(ErrorType::TypeMismatchAny { expected: vec![
				ValueType::Number,
				ValueType::Waveform,
			]}, value.1.clone()))
		}
	};

	let inits = make_param_inits_for_seq_gen(&reg_inits);

	// TODO 他のトラックにもイベントを送れるようにするには全てのトラックの初期値が必要になるはず
	let seqs = generate_sequences(&ast, pctx.ticks_per_bar, &tag_set, format!("{}.", &track).as_str(), &inits, &label_defaults, &mut evaluate_expr) ?;
	let _seqr = nodes.add_node_with_tag(MACHINE_MAIN, seq_tag.to_string(), Box::new(Sequencer::new(track.to_string(), seqs)));

	let mut output = instrm;
	if features.contains(&Feature::Velocity) {
		// TODO タグ名は feature requirements として generate_sequences の際に受け取る
		let vel = add_register(reg_inits, nodes, submachine_idx, QualifiedLabel::new(vec![track], "#velocity"), VAR_DEFAULT_KEY, VELOCITY_INIT,
				Some(DomainHint::range_including_both_ends(0f32, 2f32 * VELOCITY_INIT)));
		let output_vel = multiply(Some(track), nodes, submachine_idx, output, vel) ?; // 必ず成功するはず
		output = output_vel;
	}
	if features.contains(&Feature::Volume) {
		// TODO タグ名は feature requirements として generate_sequences の際に受け取る
		let vol = add_register(reg_inits, nodes, submachine_idx, QualifiedLabel::new(vec![track], "#volume"), VAR_DEFAULT_KEY, VOLUME_INIT,
				Some(DomainHint::range_including_both_ends(0f32, 2f32 * VOLUME_INIT)));
		let output_vol = multiply(Some(track), nodes, submachine_idx, output, vol) ?; // 必ず成功するはず
		output = output_vol;
	}

	let tick_delay = 0; // TODO 仮（遅延管理は廃止の方向）
	nodes.set_driver_delay(submachine_idx, tick_delay);

	Ok(output)
}

fn make_param_inits_for_seq_gen(tree: &RegisterSettingsTree) -> HashMap<ParamSignature, f32> {
	fn rec(tree: &RegisterSettingsSubtree, path: Option<&QualifiedLabel>) -> Vec<(ParamSignature, f32)> {
		let results = tree.iter().map(|(path_step, tree_node)| {
			let new_path = match path {
				Some(path) => path.append(path_step),
				None => QualifiedLabel::local(path_step),
			};
			match tree_node {
				RegisterSettingsTreeNode::Settings { reg_key, init, .. } => {
					vec![(ParamSignature::new(new_path, reg_key), *init)]
				},
				RegisterSettingsTreeNode::Group(subtree) => {
					rec(subtree, Some(&new_path))
				},
			}
		});
		results.concat()
	}
	rec(&tree.0, None).into_iter().collect()
}

fn collect_label_defaults(instrm_def: &ModuleDef, track: &str, use_default_labels: bool, result: &mut HashMap<QualifiedLabel, String>) /* -> HashMap<String, String> */ {
	fn visit_struct(strukt: &ModuleDef, track: &str, use_default_labels: bool, result: &mut HashMap<QualifiedLabel, String>) {
		match strukt {
			ModuleDef::NodeCreation { factory, args, label } => {
				for (_, (arg, _)) in args {
					if let ValueBody::ModuleDef(arg) = arg {
						visit_struct(arg, track, use_default_labels, result);
					}
				}
				if let (Some(label), Some(default_key)) = (label, factory.node.default_prop_key()) {
					result.insert(label.prepend(track), default_key.clone());
				}
				// 互換性対応：全て Var と見なす
				if use_default_labels {
					for arg_spec in factory.node.node_arg_specs() {
						result.insert(qualify_tag(track, arg_spec.name.as_str()), VAR_DEFAULT_KEY.to_string());
					}
				}
			},
			ModuleDef::Calc { args, .. } => {
				for arg in args { visit_struct(arg, track, use_default_labels, result); }
			},
			ModuleDef::Connect(lhs, rhs) => {
				visit_struct(lhs, track, use_default_labels, result);
				visit_struct(rhs, track, use_default_labels, result);
			},
			ModuleDef::Condition { cond, then, els } => {
				visit_struct(cond, track, use_default_labels, result);
				visit_struct(then, track, use_default_labels, result);
				visit_struct(els, track, use_default_labels, result);
			},
			ModuleDef::Lambda { body, .. } => {
				visit_struct(body, track, use_default_labels, result);
			},
			ModuleDef::Constant { label, .. } => {
				if let Some(label) = label {
					// TODO VarFactory から取った方が統一感ある
					result.insert(label.prepend(track), VAR_DEFAULT_KEY.to_string());
				}
			},
			ModuleDef::Placeholder { .. } => { },
			// この中のラベルは使わないので収集しない
			ModuleDef::LabelGuard(_) => { },

		}
	}

	// let mut result = HashMap::new();
	visit_struct(instrm_def, track, use_default_labels, result);

	// result
}

pub type PlaceholderStack = Stack<HashMap<String, NodeId>>;

fn build_instrument(
	track: &str,
	instrm_def: &ModuleDef,
	nodes: &mut AllNodes,
	submachine_idx: MachineIndex,
	freq: NodeId,
	placeholders: &mut PlaceholderStack,
	label_defaults: &HashMap<QualifiedLabel, String>,
	use_default_labels: bool,
	// inits: &mut HashMap<ParamSignature, f32>,
	reg_inits: &mut RegisterSettingsTree,
) -> ModdlResult<NodeId> {
	fn visit_struct(
		track: &str,
		strukt: &ModuleDef,
		nodes: &mut AllNodes,
		submachine_idx: MachineIndex,
		input: NodeId,
		default_tag: Option<QualifiedLabel>,
		placeholders: &mut PlaceholderStack,
		label_defaults: &HashMap<QualifiedLabel, String>,
		use_default_labels: bool,
		// inits: &mut HashMap<ParamSignature, f32>,
		reg_inits: &mut RegisterSettingsTree,
		inside_label_guard: bool,
		domain_hint: Option<&DomainHint>,
	) -> ModdlResult<NodeId> {
		struct Hoge(i32);

		// 関数にするとライフタイム関係？のエラーが取れなかったので…
		macro_rules! recurse {
			// $const_tag は、直下が定数値（ノードの種類としては Var）であった場合に付与するタグ
			($strukt: expr, $input: expr, $inside_label_guard: expr, $const_tag: expr, $domain_hint: expr) => { visit_struct(track, $strukt, nodes, submachine_idx, $input, /* Some( */$const_tag/* ) */, placeholders, label_defaults, use_default_labels, /* inits, */ reg_inits, $inside_label_guard, $domain_hint) };
			($strukt: expr, $input: expr, $inside_label_guard: expr) => { visit_struct(track, $strukt, nodes, submachine_idx, $input, None, placeholders, label_defaults, use_default_labels, /* inits, */ reg_inits, $inside_label_guard, None) };
		}
		// 関数にすると（同上）
		macro_rules! add_node {
			// トラックに属する node は全てトラック名のタグをつける
			($new_node: expr) => { Ok(nodes.add_node_with_tag(submachine_idx, track.to_string(), $new_node)) }
		}

		// ノードの引数をデフォルトを考慮して解決する
		// let mut make_node_args = |args: &HashMap<String, Value>, fact: &Rc<dyn NodeFactory>/* , label: String */|
		let mut make_node_args = |args: &HashMap<String, Value>, def: &NodeDef/* , label: String */|
				-> ModdlResult<NodeArgs> {
			let specs = def.node.node_arg_specs();
			let mut node_args = NodeArgs::new();
			for NodeArgSpec { name, channels, default } in specs {
				let arg_val = args.iter().find(|(n, _)| **n == *name );
				let strukt = if let Some(arg_val) = arg_val {
					// arg_val.1.as_module_def()
					// 		// node_args に指定された引数なのに ModuleDef に変換できない
					// 		.ok_or_else(|| error(ErrorType::NodeDefNotFound, Location::dummy())) ?

					// 変更前のコード↑では NodeDefNotFound だが、変更後↓は TypeMismatch になる。TypeMismatch でよくない？
					arg_val.1.as_module_def().map(|v| v.0)?
				} else if let Some(default) = default {
					// TODO ノードの引数で値を数値以外にすると panic する？　そうならそれは不適切で、エラーにすべき
					ValueBody::Number(default).as_module_def().unwrap()
				} else {
					// 必要な引数が与えられていない
					Err(error(ErrorType::NodeDefNotFound, Location::dummy())) ?
				};
				// ラベルが明示されていればそちらを使う
				// 👻
				let arg_name = arg_val.map(|(_, (value, _))| value.label()).flatten()
						.or_else(|| if use_default_labels { Some(QualifiedLabel::local(&name)) } else { None })/* .unwrap_or(name.clone()) */;
				/*_
					ここで arg_name は QLabel で来る。
					つまり @@ 演算子などで処理されることで修飾されてくるのか？
					def.domains はそのような修飾を経ていないのでローカル名しか持っていない。
					どうすればいいのか？
					* def.domains を arg_name のローカル名で引いてしまう
					  * いかにも乱暴だし、まずくなる場合がありそうだが…
					* ここに至るまでの過程で args_name が修飾されるのと並行して def.domains も修飾するようにする
					  * こっちが筋のような気がするが

				 */
				let arg_domain_hint = def.domains.get(&name);
				let arg_node = recurse!(&strukt, input, inside_label_guard, arg_name, arg_domain_hint) ?;
				let coerced_arg_node = match coerce_input(Some(track), nodes, submachine_idx, arg_node, channels) {
					Some(result) => result,
					// モノラルであるべき node_arg にステレオが与えられた場合、
					// 勝手にモノラルに変換するとロスが発生するのでエラーにする
					None => Err(error(ErrorType::ChannelMismatch, Location::dummy())),
				} ?;
				let node_idx = ensure_on_machine(nodes, coerced_arg_node, submachine_idx);
				node_args.insert(name.clone(), node_idx);
			}
			Ok(node_args)
		};
		
		match strukt {
			ModuleDef::Calc { node_factory, args } => {
				// TODO Result が絡んでるときも map できれいに書きたい
				let mut arg_nodes = vec![];
				for arg in args {
					arg_nodes.push(recurse!(arg, input, inside_label_guard) ?);
				}

				create_calc_node(Some(track), nodes, submachine_idx, arg_nodes, node_factory.borrow())
			},

			ModuleDef::Connect(lhs, rhs) => {
				// TODO mono/stereo 変換
				let l_node = recurse!(lhs, input, inside_label_guard) ?;
				recurse!(rhs, l_node, inside_label_guard)
			},

			ModuleDef::Condition { cond, then, els } => {
				let cond_result = recurse!(cond, input, inside_label_guard) ?;
				let cond_result = ensure_on_machine(nodes, cond_result, submachine_idx);
				let then_result = recurse!(then, input, inside_label_guard) ?;
				let then_result = ensure_on_machine(nodes, then_result, submachine_idx);
				let else_result = recurse!(els, input, inside_label_guard) ?;
				let else_result = ensure_on_machine(nodes, else_result, submachine_idx);
				// let max_delay = * vec![cond_result_on_machine.1, then_result_on_machine.1, else_result_on_machine.1].iter().max().unwrap();
				// TODO ステレオ対応（入力のどれかがステレオならステレオに拡張する）
				// let mut to_mono = |node| ensure_on_machine(nodes, node, submachine_idx).as_mono();
				let node = Box::new(Condition::new(
					cond_result.as_mono(), then_result.as_mono(), else_result.as_mono()));
				add_node!(node)
			},

			ModuleDef::Lambda { input_param, body } => {
				placeholders.push_clone();
				placeholders.top_mut().insert(input_param.clone(), input);

				let result = recurse!(body, input, inside_label_guard);

				placeholders.pop();

				result
			}

			// ModuleDef::Identifier(id) => {
			// 	// id は今のところ引数なしのノード生成しかない
			// 	let fact = factories.get(id).ok_or_else(|| ErrorType::NodeDefNotFound) ?;
			// 	apply_input(Some(track), nodes, fact, &ValueArgs::new(), &NodeArgs::new(), input)
			// },
			ModuleDef::NodeCreation { factory, args, label } => {
				let node_args = make_node_args(args, &factory/* .node */) ?;

				let local_tag = if inside_label_guard {
					None
				} else {
					label.as_ref().or(default_tag.as_ref())
				};
				let full_tag = local_tag.map(|tag| tag.prepend(track));
				if let Some(tag) = &full_tag {
					for (key, value) in factory.node.initial_values() {
						// TODO ここで label_defaults から見つからないことはありえないはずだが、補足できるエラー（内部エラー的な）として軟着陸させた方がよさそう
						let default = label_defaults.get(&tag).unwrap();
						// inits.insert((tag.clone(), key), value);

						// TODO DomainHint を factory.domains から引く？
						let domain_hint = None;
						reg_inits.add_settings(tag, default, value, domain_hint);
					}
				}

				apply_input(Some(track), nodes, submachine_idx, &factory.node, &node_args, full_tag, input)
			}
			// TODO Constant は、NodeCreation で VarFactory を使ったのと同じにできるはず。共通化する
			ModuleDef::Constant { value, label } => {
				let local_tag = if inside_label_guard {
					None
				} else {
					label.as_ref().or(default_tag.as_ref())
				};
				// let full_tag = local_tag.map(|tag| qualify_tag(track, tag.0.as_str()));
				let full_tag = local_tag.map(|tag| tag.prepend(track));
				// dbg!(label, &default_tag, &local_tag, &full_tag);
				match full_tag {
					Some(tag) => {
						// TODO ここで label_defaults から見つからないことはありえないはずだが、補足できるエラー（内部エラー的な）として軟着陸させた方がよさそう
						let default = label_defaults.get(&tag).unwrap();
						// inits.insert((tag.clone(), default.clone()), *value);
						// #56 トラック名を tag として登録するのは何か意味があったのだろうか？
						// UI を作る上で不都合なので登録しないようにする
						// Ok(add_registers(reg_inits, nodes, submachine_idx, vec![/* track.to_string(), */ tag], *value))

						//_ TODO この定数が NodeDef のパラメータに単独で付与されたものである場合、そのパラメータの DomainHint を持ってくる
						/*
							↑ の詳細：
							NodeCreation の処理から make_node_args の中で再帰を経てここへ流れてくると思われる。
							make_node_args は factory の node だけ受け取っているが、domains も含めた factory ごと受け取るようにすれば、
							各 arg の名前に応じた定義域を引いて再帰に渡すことができる。
							ここでは定義域が渡されたら add_register にそのまま渡せばよい。
							🐱🐱🐱🐱🐱🐱 次やるときはこの方針でやってみること 🐱🐱🐱🐱🐱🐱

						*/
						// TODO DomainHint を上流から受け取った factory.domains から引く
						
						Ok(add_register(reg_inits, nodes, submachine_idx, tag, default, *value, domain_hint.map(Clone::clone)))
					},
					None => add_node!(Box::new(Var::new(*value))),
				}
				
			},
			ModuleDef::Placeholder { name } => {
				// 名前に対応する placeholder は必ずある
				Ok(placeholders.top()[name])
			},
			ModuleDef::LabelGuard(inner) => {
				recurse!(inner, input, true)
			}
		}
	}

	visit_struct(track, instrm_def, nodes, submachine_idx, freq, None, placeholders, label_defaults, use_default_labels, /* inits, */ reg_inits, false, None)
}

// fn create_node_by_factory(factory: &Rc<dyn NodeDef>, args: &HashMap<String, Value>) {
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
			Some(add_node!(Box::new(MonoToStereo::new(input.node(submachine_idx).as_mono()))))
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
	node_args: &NodeArgs,
	label: Option</* String */QualifiedLabel>,
	input: NodeId,
) -> ModdlResult<NodeId> {
	// TODO 共通化
	macro_rules! add_node {
		// トラックに属する node は全てトラック名のタグをつける
		($label: expr, $new_node: expr) => {
			{
				let label: &Option<QualifiedLabel> = &$label;
				// let mut add_node = |is_labeled_node, new_node| Ok::<NodeId, Error>({
				let mut tags: Vec<QualifiedLabel> = vec![];
				if let Some(full_tag) = label { tags.push(full_tag.clone()); }
				// TODO これ必要なのだろうか？
				if let Some(track) = track { tags.push(QualifiedLabel::local(track)/* track.to_string() */); }

				let tag_paths = tags.iter().map(QualifiedLabel::to_string).collect();
				Ok(nodes.add_node_with_tags(submachine_idx, tag_paths, $new_node))
			}
		}
	}

	match coerce_input(track, nodes, submachine_idx, input, fact.input_channels()) {
		Some(result) => {
			let coerced_input = result ?;
			// add_node!(fact.create_node(node_args, coerced_input.node(submachine_idx)))
			let input_idx = ensure_on_machine(nodes, coerced_input, submachine_idx);
			add_node!(label, fact.create_node(node_args, input_idx))
		},
		None => {
			// 一旦型を明記した変数に取らないとなぜか E0282 になる
			// TODO ここも Some の場合と同様に ensure_on_machine が必要？
			let input_idx = ensure_on_machine(nodes, input, submachine_idx);
			let input_l = {
				let result: ModdlResult<NodeId> = add_node!(None, Box::new(
						Split::new(input_idx.as_stereo(), 0)));
				result ?
			};
			let input_r = {
				let result: ModdlResult<NodeId> = add_node!(None, Box::new(
						Split::new(input_idx.as_stereo(), 1)));
				result ?
			};
			let result_l = {
				let result: ModdlResult<NodeId> = add_node!(label, 
						fact.create_node(node_args, input_l.node(submachine_idx)));
				result ?
			};
			let result_r = {
				let result: ModdlResult<NodeId> = add_node!(label, 
						fact.create_node(node_args, input_r.node(submachine_idx)));
				result ?
			};
			add_node!(None, Box::new(Join::new(vec![result_l.node(submachine_idx).as_mono(), result_r.node(submachine_idx).as_mono()])))
		}
	}
}

const MACHINE_MAIN: MachineIndex = MachineIndex(0usize);
struct AllNodes {
	single_machine: bool,
	machines: Vec<MachineSpec>,
	sends_to_receives: HashMap<NodeId, NodeId>,

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
		eprintln!("machines[{}]: {}", submachine_idx.0, & self.machines[submachine_idx.0].name);

		submachine_idx
	}
	pub fn add_node(&mut self, machine: MachineIndex, node: Box<dyn Node>) -> NodeId {
		let node_idx = self.machines[machine.0].nodes.add(node);
		let result = NodeId::new(machine, node_idx);

		result
	}
	pub fn add_node_with_tags(&mut self, machine: MachineIndex, tags: Vec<String>, node: Box<dyn Node>) -> NodeId {
		let node_idx = self.machines[machine.0].nodes.add_with_tags(tags, node);
		let result = NodeId::new(machine, node_idx);

		result
	}
	pub fn add_node_with_tag(&mut self, machine: MachineIndex, tag: String, node: Box<dyn Node>) -> NodeId {
		let node_idx = self.machines[machine.0].nodes.add_with_tag(tag, node);
		let result = NodeId::new(machine, node_idx);

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
}

const INTERTHREAD_BUFFER_SIZE: u32 = 50;
use crate::node::thread::*;
// use std::thread;
use std::sync::mpsc::sync_channel;

/// 別マシン上の出力を Sender/Receiver を使って持ってくる。同一マシン上の場合はそのまま使う
/// TODO なんかいい名前あれば…
fn ensure_on_machine(nodes: &mut AllNodes, node: NodeId, dest_machine: MachineIndex) -> ChanneledNodeIndex {
	if node.machine == dest_machine {
		// 同一マシン上のノードなのでそのまま使える
		node.node(dest_machine)

	} else {
		// 別マシンなので Sender/Receiver で持ってくる
		let (sender, receiver) = sync_channel::<Vec<Sample>>(0);
		// TODO ステレオ対応
		let sender_node = nodes.add_node(node.machine, Box::new(Sender::new(
				node.node_of_any_machine(), sender, INTERTHREAD_BUFFER_SIZE as usize)));

		let receiver_node = nodes.add_node(dest_machine, Box::new(Receiver::new(
				node.node_of_any_machine().channels(),
				receiver)));
		nodes.add_send_receive(sender_node, receiver_node);

		receiver_node.node(dest_machine)
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
			let arg_node_idxs = args.iter().map(|a| a.as_mono()).collect();
			add_node!(node_factory.create_mono(arg_node_idxs))
		},
		ChannelCombination::AllStereo => {
			let args: Vec<_> = arg_nodes.iter().map(|n| ensure_on_machine(nodes, *n, submachine_idx)).collect();
			let arg_node_idxs = args.iter().map(|a| a.as_stereo()).collect();
			add_node!(node_factory.create_stereo(arg_node_idxs))
		},
		ChannelCombination::MonoAndStereo => {
			let mut coerced_arg_nodes: Vec<StereoNodeIndex> = vec![];
			for n in arg_nodes {
				let node_idx = ensure_on_machine(nodes, n, submachine_idx);
				coerced_arg_nodes.push(if n.channels() == 1 {
					// let mono = ensure_on_machine(nodes, n, submachine_idx).as_mono();
					let stereo = add_node!(Box::new(MonoToStereo::new(node_idx.as_mono()))) ?;
					// ensure_on_machine(nodes, stereo, submachine_idx).as_stereo()
					stereo.node(submachine_idx).as_stereo()
				} else {
					// ensure_on_machine(nodes, n, submachine_idx).as_stereo()
					// let (node_idx, delay) = ensure_on_machine(nodes, n, submachine_idx);
					node_idx.as_stereo()
				});
			}
			add_node!(node_factory.create_stereo(coerced_arg_nodes))
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
