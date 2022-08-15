use super::{
	builtin::*,
	error::*,
	evaluator::*,
	value::*,
};
use crate::{
	calc::*,
	common::stack::*,
	core::{
		common::*,
		context::*,
		machine::*,
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
		var::*,
	},
	seq::{
		sequencer::*,
		tick::*,
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
	collections::btree_map::BTreeMap,
	collections::hash_map::HashMap,
	collections::hash_set::HashSet,
	fs::File,
	io::Read,
	rc::Rc,
};

// TODO エラー処理を全体的にちゃんとする

const TAG_SEQUENCER: &str = "seq";

// struct Track<'a> {
// 	instrument: &'a Expr,
// 	mml: String,
// };

pub fn play_file(moddl_path: &str) -> ModdlResult<()> {
	let mut file = File::open(moddl_path) ?;
	let mut moddl = String::new();
	file.read_to_string(&mut moddl) ?;

	play(moddl.as_str())
}

#[derive(PartialEq)]
enum MuteSolo { Mute, Solo }

enum TrackSpec {
	Instrument(NodeStructure),
	Effect(HashSet<String>, NodeStructure),
	Groove(NodeStructure),
}

struct PlayerContext {
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
	vars: VarStack,
}
impl PlayerContext {
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


pub fn play(moddl: &str) -> ModdlResult<()> {
	// TODO パーズエラーをちゃんと処理
	let (_, CompilationUnit { statements }) = compilation_unit()(moddl) ?;

	let mut pctx = PlayerContext {
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
		vars: VarStack::init(builtin_vars()),
	};

	for stmt in &statements {
		process_statement(&stmt, &mut pctx) ?;
	}
	
	let mut nodes = NodeHost::new();
	// TODO タグ名を sequence_generator と共通化
	let tempo = nodes.add_with_tag("#tempo".to_string(), Box::new(Var::new(pctx.tempo)));
	let timer = nodes.add(Box::new(TickTimer::new(tempo.as_mono(), pctx.ticks_per_bar, pctx.groove_cycle))).as_mono();

	// TODO even groove を誰も使わない場合は省略
	nodes.add(Box::new(Tick::new(timer, pctx.groove_cycle, TAG_SEQUENCER.to_string())));

	let mut output_nodes = HashMap::<String, ChanneledNodeIndex>::new();

	// for (track, mml) in &pctx.mmls {
	for (track, spec) in &pctx.track_specs {
		let mml = &pctx.mmls.get(track).map(|mml| mml.as_str()).unwrap_or("");
		let output_node = {
			// @mute で指定されているか、@solo で指定されていなければ、ミュート対象
			if pctx.mute_solo_tracks.contains(track) == (pctx.mute_solo == MuteSolo::Mute) {
				Some(nodes.add(Box::new(Constant::new(0f32))))
			} else {
				let seq_tag = pctx.grooves.get(track).map(|t| t.clone()).unwrap_or(TAG_SEQUENCER.to_string());
				match spec {
					TrackSpec::Instrument(structure) => {
						Some(build_nodes_by_mml(track.as_str(), structure, mml, pctx.ticks_per_bar, &seq_tag, &mut nodes/* , &mut output_nodes */,
								&mut PlaceholderStack::init(HashMap::new()), None) ?)
					}
					TrackSpec::Effect(source_tracks, structure) => {
						let mut placeholders = PlaceholderStack::init(HashMap::new());
						source_tracks.iter().for_each(|track| {
							placeholders.top_mut().insert(track.clone(), output_nodes[track]);
						});
						Some(build_nodes_by_mml(track.as_str(), structure, mml, pctx.ticks_per_bar, &seq_tag, &mut nodes,
								&mut placeholders, None) ?)
					}
					TrackSpec::Groove(structure) => {
						let groovy_timer = build_nodes_by_mml(track.as_str(), structure, mml, pctx.ticks_per_bar, &seq_tag, &mut nodes, &mut PlaceholderStack::init(HashMap::new()), Some(timer))?.as_mono();
						nodes.add(Box::new(Tick::new(groovy_timer, pctx.groove_cycle, seq_tag.clone())));

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
	let terminal_nodes: Vec<ChanneledNodeIndex> = terminal_tracks.iter().map(|t| output_nodes[*t]).collect();
	let mix = {
		if terminal_nodes.is_empty() {
			nodes.add(Box::new(Constant::new(0f32)))
		} else {
			// FIXME Result が絡むときの fold をきれいに書く方法
			let head = *terminal_nodes.first().unwrap();
			let tail = &terminal_nodes[1..];
			let mut sum = head;
			for t in tail {
				sum = add(None, &mut nodes, sum, *t) ?;
			}
			sum
		}
	};
	let master_vol = nodes.add(Box::new(Constant::new(0.5f32))); // TODO 値を外から渡せるように
	let master = multiply(None, &mut nodes, mix, master_vol) ?;
	nodes.add(Box::new(PortAudioOut::new(master)));
	// nodes.add(Box::new(crate::node::file::WavFileOut::new(master, "out.wav".to_string())));
	// nodes.add(Box::new(NullOut::new(master)));
	// TODO タグ名共通化
	nodes.add_with_tag("terminator".to_string(), Box::new(Terminator::new(master)));
	// nodes.add(Box::new(Print::new(master)));

	let mut context = Context::new(44100); // TODO 値を外から渡せるように

	let start = std::time::Instant::now();
	Machine::new().play(&mut context, &mut nodes, &mut pctx.waveforms);
	let end = std::time::Instant::now();
	println!("{:?}", end.duration_since(start));

	Ok(())
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
					pctx.vars.push_clone();
					for source_track in &source_tracks {
						pctx.vars.top_mut().insert(source_track.clone(),
								Value::NodeStructure(NodeStructure::Placeholder { name: source_track.clone() }));
						pctx.terminal_tracks.remove(source_track);
					}

					let effect = evaluate_arg(&args, 2, &mut pctx.vars)?.as_node_structure()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					for track in tracks {
						pctx.add_track_spec(&track, TrackSpec::Effect(source_tracks.iter().map(|t| t.clone()).collect(), effect.clone())) ?;
						pctx.terminal_tracks.insert(track);
					}
					// TODO finally にする
					pctx.vars.pop();
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
					pctx.grooves.insert(control_track.clone(), make_seq_id(Some(&control_track)));
					for track in &target_tracks {
						if pctx.grooves.contains_key(track) { return Err(Error::GrooveTargetDuplicate { track: track.clone() }); }
						pctx.grooves.insert(track.clone(), make_seq_id(Some(&control_track)));
					}
				}
				"let" => {
					let name = evaluate_arg(&args, 0, &mut pctx.vars)?.as_identifier_literal()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					let value = evaluate_arg(&args, 1, &mut pctx.vars) ?;
					pctx.vars.top_mut().insert(name, value);
				}
				"waveform" => {
					let name = evaluate_arg(&args, 0, &mut pctx.vars)?.as_identifier_literal()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					let value = evaluate_arg(&args, 1, &mut pctx.vars) ?;
					let path = value.as_string_literal().ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					// TODO 読み込み失敗時のエラー処理
					let index = pctx.waveforms.add(read_wav_file(path.as_str(), None, None, None, None) ?);
					pctx.vars.top_mut().insert(name, Value::WaveformIndex(index));
					// vars.insert(name, value);
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
fn make_seq_id(track: Option<&String>) -> String {
	match track {
		None => "#seq".to_string(),
		Some(track) => format!("#seq_{}", track),
	}
}

fn set_mute_solo(mute_solo: MuteSolo, tracks: &Vec<String>, pctx: &mut PlayerContext) {
	(*pctx).mute_solo = mute_solo;
	(*pctx).mute_solo_tracks.clear();
	tracks.iter().for_each(|t| {
		(*pctx).mute_solo_tracks.insert(t.clone());
	});
}

fn evaluate_arg(args: &Vec<Expr>, index: usize, vars: &mut VarStack) -> ModdlResult<Value> {
	if index < args.len() {
		evaluate(&args[index], vars)
	} else {
		Err(Error::DirectiveArgNotFound)
	}
}

fn build_nodes_by_mml<'a>(track: &str, instrm_def: &NodeStructure, mml: &'a str, ticks_per_bar: i32, seq_tag: &String, nodes: &mut NodeHost/* , output_nodes: &mut Vec<ChanneledNodeIndex> */, placeholders: &mut PlaceholderStack, override_input: Option<MonoNodeIndex>)
		-> ModdlResult<ChanneledNodeIndex> {
	let (_, ast) = default_mml_parser::compilation_unit()(mml) ?; // TODO パーズエラーをちゃんとラップする
	let freq_tag = format!("{}_freq", track);

	let tag_set = TagSet {
		freq: freq_tag.clone(),
		note: track.to_string(),
	};
	let (seqs, features) = generate_sequences(&ast, ticks_per_bar, &tag_set, format!("{}.", &track).as_str());
	let _seqr = nodes.add_with_tag(seq_tag.to_string(), Box::new(Sequencer::new(seqs)));

	let mut input = match override_input {
		Some(input) => input.channeled(),
		None => nodes.add_with_tag(freq_tag.clone(), Box::new(Var::new(0f32))),
	};
	if features.contains(&Feature::Detune) {
		// セント単位のデチューン
		// freq_detuned = freq * 2 ^ (detune / 1200)
		// TODO タグ名は feature requirements として generate_sequences の際に受け取る
		let detune = nodes.add_with_tag(format!("{}.#detune", &track), Box::new(Var::new(0f32)));
		let cents_per_oct = nodes.add(Box::new(Constant::new(1200f32)));
		let detune_oct = divide(Some(track), nodes, detune, cents_per_oct) ?; // 必ず成功するはず
		let const_2 = nodes.add(Box::new(Constant::new(2f32)));
		let freq_ratio = power(Some(track), nodes, const_2, detune_oct) ?; // 必ず成功するはず
		let freq_detuned = multiply(Some(track), nodes, input, freq_ratio) ?; // 必ず成功するはず
		input = freq_detuned;
	}
	
	let instrm = build_instrument(track, instrm_def, nodes, /* freq */input, placeholders) ?;

	let mut output = instrm;
	if features.contains(&Feature::Velocity) {
		// TODO タグ名は feature requirements として generate_sequences の際に受け取る
		// Var に渡す 1 は velocity, volume の初期値（1 が最大）
		let vel = nodes.add_with_tag(format!("{}.#velocity", &track), Box::new(Var::new(1f32)));
		let output_vel = multiply(Some(track), nodes, output, vel) ?; // 必ず成功するはず
		output = output_vel;
	}
	if features.contains(&Feature::Volume) {
		// TODO タグ名は feature requirements として generate_sequences の際に受け取る
		// Var に渡す 1 は velocity, volume の初期値（1 が最大）
		let vol = nodes.add_with_tag(format!("{}.#volume", &track), Box::new(Var::new(1f32)));
		let output_vol = multiply(Some(track), nodes, output, vol) ?; // 必ず成功するはず
		output = output_vol;
	}

	Ok(output)
}

pub type PlaceholderStack = Stack<HashMap<String, ChanneledNodeIndex>>;

fn build_instrument(track: &str, instrm_def: &NodeStructure, nodes: &mut NodeHost, freq: ChanneledNodeIndex, placeholders: &mut PlaceholderStack) -> ModdlResult<ChanneledNodeIndex> {
	fn visit_struct(track: &str, strukt: &NodeStructure, nodes: &mut NodeHost, input: ChanneledNodeIndex, default_tag: Option<String>, placeholders: &mut PlaceholderStack) -> ModdlResult<ChanneledNodeIndex> {
		// 関数にするとライフタイム関係？のエラーが取れなかったので…
		macro_rules! recurse {
			// $const_tag は、直下が定数値（ノードの種類としては Var）であった場合に付与するタグ
			($strukt: expr, $input: expr, $const_tag: expr) => { visit_struct(track, $strukt, nodes, $input, Some($const_tag), placeholders) };
			($strukt: expr, $input: expr) => { visit_struct(track, $strukt, nodes, $input, None, placeholders) };
		}
		// 関数にすると（同上）
		macro_rules! add_node {
			// トラックに属する node は全てトラック名のタグをつける
			($new_node: expr) => { Ok(nodes.add_with_tag(track.to_string(), $new_node)) }
		}

		// ノードの引数をデフォルトを考慮して解決する
		let mut make_node_args = |args: &HashMap<String, Value>, fact: &Rc<dyn NodeFactory>/* , label: String */|
				-> ModdlResult<HashMap<String, ChanneledNodeIndex>> {
			let specs = fact.node_arg_specs();
			let mut node_args = NodeArgs::new();
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
				let coerced_arg_node = match coerce_input(Some(track), nodes, arg_node, channels) {
					Some(result) => result,
					// モノラルであるべき node_arg にステレオが与えられた場合、
					// 勝手にモノラルに変換するとロスが発生するのでエラーにする
					None => Err(Error::ChannelMismatch),
				} ?;
				node_args.insert(name.clone(), coerced_arg_node);
			}
			Ok(node_args)
		};
		
		match strukt {
			NodeStructure::Calc { node_factory, args } => {
				// TODO Result が絡んでるときも map できれいに書きたい
				let mut arg_nodes = vec![];
				for arg in args {
					arg_nodes.push(recurse!(arg, input) ?);
				}

				create_calc_node(Some(track), nodes, arg_nodes, node_factory.borrow())
			},

			NodeStructure::Connect(lhs, rhs) => {
				// TODO mono/stereo 変換
				let l_node = recurse!(lhs, input) ?;
				recurse!(rhs, l_node)
			},

			NodeStructure::Condition { cond, then, els } => {
				let cond_result = recurse!(cond, input) ?;
				let then_result = recurse!(then, input) ?;
				let else_result = recurse!(els, input) ?;

				// TODO ステレオ対応（入力のどれかがステレオならステレオに拡張する）
				add_node!(Box::new(Condition::new(
						cond_result.as_mono(), then_result.as_mono(), else_result.as_mono())))
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
				let node_args = make_node_args(&HashMap::new(), fact) ?;
				apply_input(Some(track), nodes, fact, &node_args, input)
			},
			NodeStructure::NodeWithArgs { factory, label: _, args } => {
				// 引数ありのノード生成
				let fact = match &**factory {
					NodeStructure::NodeFactory(fact) => Ok(fact),
					_ => { dbg!("poke"); Err(Error::DirectiveArgTypeMismatch) },
				} ?;
				let node_args = make_node_args(args, fact/* , &label */) ?;

				apply_input(Some(track), nodes, fact, &node_args, input)
			},
			NodeStructure::Constant { value, label } => {
				let node = Box::new(Var::new(*value));
				let local_tag = label.as_ref().or(default_tag.as_ref());
				let full_tag = local_tag.map(|tag| format!("{}.{}", track, tag.clone()));
				// dbg!(label, &default_tag, &local_tag, &full_tag);
				match full_tag {
					Some(tag) => Ok(nodes.add_with_tags(vec![track.to_string(), tag], node)),
					None => add_node!(node),
				}
				
			},
			NodeStructure::Placeholder { name } => {
				// 名前に対応する placeholder は必ずある
				Ok(placeholders.top()[name])
			},
		}
	}

	visit_struct(track, instrm_def, nodes, freq, None, placeholders)
}

fn coerce_input(
	track: Option<&str>,
	nodes: &mut NodeHost,
	input: ChanneledNodeIndex,
	expected_channels: i32
) -> Option<ModdlResult<ChanneledNodeIndex>> {
	// TODO 共通化
	macro_rules! add_node {
		// トラックに属する node は全てトラック名のタグをつける
		($new_node: expr) => {
			Ok(match track {
				Some(track) => nodes.add_with_tag(track.to_string(), $new_node),
				None => nodes.add($new_node),
			})
		}
	}
	match (input.channels(), expected_channels) {
		(1, 1) => Some(Ok(input)),
		(1, 2) => Some(add_node!(Box::new(MonoToStereo::new(input.as_mono())))),
		(2, 1) => None, // ステレオの入力をモノラルに入れる場合、状況によってすべきことが異なるので、呼び出し元に任せる
		(2, 2) => Some(Ok(input)),
		_ => Some(Err(Error::ChannelMismatch)),
	}
}


fn apply_input(
	track: Option<&str>,
	nodes: &mut NodeHost,
	fact: &Rc<dyn NodeFactory>,
	node_args: &NodeArgs,
	input: ChanneledNodeIndex
) -> ModdlResult<ChanneledNodeIndex> {
	// TODO 共通化
	macro_rules! add_node {
		// トラックに属する node は全てトラック名のタグをつける
		($new_node: expr) => {
			Ok(match track {
				Some(track) => nodes.add_with_tag(track.to_string(), $new_node),
				None => nodes.add($new_node),
			})
		}
	}

	match coerce_input(track, nodes, input, fact.input_channels()) {
		Some(result) => {
			let coerced_input = result ?;
			add_node!(fact.create_node(node_args, coerced_input))
		},
		None => {
			// 一旦型を明記した変数に取らないとなぜか E0282 になる
			let input_l = {
				let result: ModdlResult<ChanneledNodeIndex> = add_node!(Box::new(Split::new(input.as_stereo(), 0)));
				result ?
			};
			let input_r = {
				let result: ModdlResult<ChanneledNodeIndex> = add_node!(Box::new(Split::new(input.as_stereo(), 1)));
				result ?
			};
			let result_l = {
				let result: ModdlResult<ChanneledNodeIndex> = add_node!(fact.create_node(node_args, input_l));
				result ?
			};
			let result_r = {
				let result: ModdlResult<ChanneledNodeIndex> = add_node!(fact.create_node(node_args, input_r));
				result ?
			};
			add_node!(Box::new(Join::new(vec![result_l.as_mono(), result_r.as_mono()])))
		}
	}
}

fn create_calc_node(
	track: Option<&str>,
	nodes: &mut NodeHost,
	arg_nodes: Vec<ChanneledNodeIndex>,
	node_factory: &dyn CalcNodeFactoryTrait,
) -> ModdlResult<ChanneledNodeIndex> {
	// TODO 共通化
	macro_rules! add_node {
		// トラックに属する node は全てトラック名のタグをつける
		($new_node: expr) => {
			ModdlResult::<ChanneledNodeIndex>::Ok(match track {
				Some(track) => nodes.add_with_tag(track.to_string(), $new_node),
				None => nodes.add($new_node),
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
			add_node!(node_factory.create_mono(arg_nodes.iter().map(|n| n.as_mono()).collect()))
		},
		ChannelCombination::AllStereo => {
			add_node!(node_factory.create_stereo(arg_nodes.iter().map(|n| n.as_stereo()).collect()))
		},
		ChannelCombination::MonoAndStereo => {
			let mut coerced_arg_nodes: Vec<StereoNodeIndex> = vec![];
			for n in arg_nodes {
				coerced_arg_nodes.push(if n.channels() == 1 {
					let stereo = add_node!(Box::new(MonoToStereo::new(n.as_mono()))) ?;
					stereo.as_stereo()
				} else {
					n.as_stereo()
				});
			}
			add_node!(node_factory.create_stereo(coerced_arg_nodes))
		},
		ChannelCombination::Other => { Err(Error::ChannelMismatch) },
	}
}

macro_rules! binary {
	($name: ident, $calc: ident) => {
		fn $name(track: Option<&str>, nodes: &mut NodeHost,
			l_node: ChanneledNodeIndex, r_node: ChanneledNodeIndex) -> ModdlResult<ChanneledNodeIndex> {
				create_calc_node(track, nodes, vec![l_node, r_node], &CalcNodeFactory::<$calc>::new())
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
