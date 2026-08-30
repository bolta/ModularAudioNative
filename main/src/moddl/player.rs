use super::{
	builtin::builtin_vars, common::read_file, error::*, executor::process_statements, import::ImportCache, player_option::*, scope::*, value::*
};
use crate::{
	core::{
		common::*,
		context::*,
		event::*,
		machine::*,
	}, moddl::{graph_builder::GraphBuilder, player_context::PlayerContext, register_settings::{RegisterSettingsSubtree, RegisterSettingsTree, RegisterSettingsTreeNode}}, vis::visualizer::*, wave::waveform_host::WaveformHost
};
extern crate parser;
use ipc::{Client, channel_name_p2c, to_bson};
use parser::moddl::ast::QualifiedLabel;

use std::{
	collections::{BTreeMap, hash_map::HashMap}, path::{Path, PathBuf}, sync::{
		Arc, mpsc::{self, SyncSender, sync_channel}
	}, thread, time::Duration
};

// TODO エラー処理を全体的にちゃんとする

const TAG_SEQUENCER: &str = "seq";

const TAG_FREQ: &str = "#freq";

fn qualify_tag(track: &str, tag: &str) -> QualifiedLabel {
	QualifiedLabel::new(vec![track], tag)
}

pub fn play(options: &PlayerOptions) -> ModdlResult<()> {
	Facade::play_moddl(&options)
}

struct Facade { }
impl Facade {
	pub fn play_moddl(PlayerOptions { moddl_path, dump_ast, no_play, output }: &PlayerOptions) -> ModdlResult<()> {
		let sample_rate = 44100; // TODO 値を外から渡せるように
		let mut waveforms = WaveformHost::new();

		// definition phase
		let (pctx, mut imports) = Self::execute_moddl(moddl_path, sample_rate, &mut waveforms, *dump_ast) ?;

		// construction phase
		let (machine_specs, reg_inits) = GraphBuilder::new(&pctx, &mut imports)
				.build(*no_play, output) ?;

		let p2c_sender = setup_p2c_sender();
		let msg = make_register_settings_message(&reg_inits);
		send_message(&p2c_sender, &msg);

		// デバッグ用機能なのでとりあえず蓋をしておく
		// TODO コマンドオプションで指定されたときだけ出力する
		// output_structure(&nodes_result, &sends_to_receives);

		// rendition phase
		run_machines(sample_rate, waveforms, machine_specs, &p2c_sender);

		Ok(())
	}

	fn execute_moddl<'a>(moddl_path: &PathBuf, sample_rate: i32, waveforms: &'a mut WaveformHost, dump_ast: bool) -> ModdlResult<(PlayerContext, ImportCache<'a>)> {
		let moddl_path = Path::new(moddl_path);
		let moddl = read_file(moddl_path) ?;
		// let mut waveforms = WaveformHost::new();
		let mut imports = ImportCache::new(waveforms, dump_ast);
		let root_vars = Scope::root(builtin_vars(sample_rate, &mut imports) ?);

		// TODO デバッグ出力？　不要そうな気もする
		if let Some(asts) = imports.asts() {
			println!("{}", serde_json::to_string(asts).unwrap());
		}

		let pctx = process_statements(moddl.as_str(), root_vars, moddl_path, &mut imports) ?;

		Ok((pctx, imports))
	}
}

fn run_machines(sample_rate: i32, waveforms: WaveformHost, machine_specs: Vec<MachineSpec>, p2c_sender: &SyncSender<Vec<u8>>) {
	let broadcast_pairs = make_broadcast_pairs(machine_specs.len());
	let broadcaster = Broadcaster::new(broadcast_pairs.senders);
	setup_termination_watcher(broadcaster.clone());

	let waveforms = Arc::new(waveforms);
	let joins: Vec<_> = machine_specs.into_iter()
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
}

fn setup_termination_watcher(broadcaster: Broadcaster) {
	// iceoryx2 は Node を作成すると SIGINT/SIGTERM のハンドリングを奪ってしまい、
	// Ctrl+C を押しても OS 標準の即時終了が起きなくなる。
	// そこで終了要求を自前で検知し、曲が終わったときと同じ経路（TerminateEvent の
	// ブロードキャスト）で各マシンを終了させる。
	thread::spawn(move || {
		match ipc::TerminationWatcher::new() {
			Ok(watcher) => {
				watcher.block_until_termination_requested(Duration::from_millis(100));
				broadcaster.broadcast(GlobalEvent::new(0, Box::new(TerminateEvent {})));
			}
			// TODO エラー処理
			Err(e) => { dbg!(&e); }
		}
	});
}

/// クライアントに向けてメッセージを送信するためのスレッドを立ち上げる。
/// このスレッドは IPC によるクライアントとの通信を一手に引き受ける。
/// スレッドの窓口は p2c_sender であり、これを clone して任意のスレッドに持っていって IPC を送ることができる
// TODO 生のバイトではなく Message を明示的に取って型安全にしたい
fn setup_p2c_sender() -> SyncSender<Vec<u8>> {
	let bound = 0usize; // TODO これでいいか？
	let (p2c_sender, p2c_receiver) = sync_channel::<Vec<u8>>(bound);
	thread::spawn(move || {
		let p2c_client = Client::new(channel_name_p2c(), Duration::from_millis(500)).unwrap();
		loop {
			match p2c_receiver.recv() {
				Ok(request_bytes) => {
					match p2c_client.send_request(& request_bytes) {
						Ok(response) => {
							println!("received response: {}", to_bson(&response).map(|doc| doc.to_string())
									.unwrap_or_else(|_| "<error parsing bson>".into()));
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

	p2c_sender
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
