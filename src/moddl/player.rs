use super::{
	ast::*,
	error::*,
	evaluator::*,
	node_factory::*,
	parser::*,
	value::*,
};
use crate::{
	core::{
		common::*,
		context::*,
		machine::*,
		node::*,
		node_host::*,
		event::*
	},
	mml::default::{
		*,
		sequence_generator::*,
	},
	node::{
		arith::*,
		audio::*,
		env::*,
		osc::*,
		prim::*,
		stereo::*,
		util::*,
		var::*,
	},
	seq::{
		sequencer::*,
		tick::*,
	},
};

use std::{
	collections::btree_map::BTreeMap,
	collections::hash_map::HashMap,
};

use combine::Parser;

// TODO エラー処理を全体的にちゃんとする

const TAG_SEQUENCER: &str = "seq";

// struct Track<'a> {
// 	instrument: &'a Expr,
// 	mml: String,
// };

pub fn play(moddl: &str) -> ModdlResult<()> {
	// TODO パーズエラーをちゃんと処理
	let (_, CompilationUnit { statements }) = compilation_unit()(moddl) ?;

	let mut tempo = 120f32;
	let ticks_per_beat = 96; // TODO 外から渡せるように
	// トラックごとの instrument を保持
	// （イベントの発行順序が曖昧にならないよう BTreeMap で辞書順を保証）
	// let mut instruments = HashMap::<String, &Expr>::new();
	let mut instruments = HashMap::<String, NodeStructure>::new();
	// トラックごとの MML を蓄積
	let mut mmls = BTreeMap::<String, String>::new();

	for stmt in &statements { process_statement(&stmt, &mut tempo, &mut instruments, &mut mmls) ?; }
	
	let mut nodes = NodeHost::new();
	nodes.add(Box::new(Tick::new(tempo, ticks_per_beat, TAG_SEQUENCER.to_string())));

	let mut output_nodes = Vec::<ChanneledNodeIndex>::new();
	for (track, mml) in &mmls {
		let instrm = instruments.get(track)
				.ok_or_else(|| Error::InstrumentNotFound { track: track.clone() }) ?;

		build_nodes_by_mml(track.as_str(), instrm, mml.as_str(), &mut nodes, &mut output_nodes) ?;
	}

	let mut context = Context::new(44100); // TODO 値を外から渡せるように

	let mix = {
		if output_nodes.is_empty() {
			nodes.add(Box::new(Constant::new(0f32)))
		} else {
			// FIXME Result が絡むときの fold をきれいに書く方法
			let head = *output_nodes.first().unwrap();
			let tail = &output_nodes[1..];
			let mut sum = head;
			for t in tail {
				sum = add(None, &mut nodes, sum, *t) ?;
			}
			sum
		}
	};
	let master_vol = nodes.add(Box::new(Constant::new(0.5f32))); // TODO 値を外から渡せるように
	let master = multiply(None, &mut nodes, mix, master_vol) ?;
	nodes.add(Box::new(PortAudioOut::new(master, &context)));

	Machine::new().play(&mut context, &mut nodes);

	Ok(())
}

fn process_statement<'a>(
	stmt: &'a Statement,
	tempo: &mut f32,
	// instruments: &mut HashMap<String, &'a Expr>,
	instruments: &mut HashMap<String, NodeStructure>,
	mmls: &mut BTreeMap<String, String>,
) -> ModdlResult</* 'a, */ ()> { // TODO 寿命指定これでいいのか
	match stmt {
		Statement::Directive { name, args } => {
			match name.as_str() {
				"tempo" => {
					*tempo = evaluate_arg(&args, 0)?.as_float()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
				},
				"instrument" => {
					let tracks = evaluate_arg(&args, 0)?.as_track_set()
							.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
					// let instrm = & args[1];
					for track in tracks {
						let instrm = evaluate_arg(&args, 1)?.as_node_structure()
								.ok_or_else(|| Error::DirectiveArgTypeMismatch) ?;
						if instruments.get(&track).is_some() {
							return Err(Error::DirectiveDuplicate { msg: format!("@instrument ^{}", &track) });
						}
						instruments.insert(track, instrm);
					}
				}
				other => {
					println!("unknown directive: {}", other);
				}
			}
		}
		Statement::Mml { tracks, mml } => {
			for track in tracks {
				if let Some(mml_concat) = mmls.get_mut(track) {
					mml_concat.push_str(mml.as_str());
				} else {
					mmls.insert(track.clone(), mml.clone());
				}
			}
		}
	}

	Ok(())
}

fn evaluate_arg(args: &Vec<Expr>, index: usize) -> ModdlResult<Value> {
	if index < args.len() {
		Ok(evaluate(&args[index]))
	} else {
		Err(Error::DirectiveArgNotFound)
	}
}

fn build_nodes_by_mml<'a>(track: &str, instrm_def: &NodeStructure, mml: &'a str, nodes: &mut NodeHost, output_nodes: &mut Vec<ChanneledNodeIndex>)
		-> ModdlResult</* 'a, */ ()> {
	let ast = default_mml_parser::compilation_unit().parse(mml)
			.map_err(|_e| Error::MmlSyntax )?.0; // TODO パーズエラーをちゃんとラップする

	let tag_set = TagSet {
		freq: track.to_string(),
		note: track.to_string(),
	};
	let seqs = generate_sequences(&ast, 96, &tag_set);
	let _seqr = nodes.add_with_tag(TAG_SEQUENCER.to_string(), Box::new(Sequencer::new(seqs)));

	let freq = nodes.add_with_tag(track.to_string(), Box::new(Var::new(0f32)));

	let instrm = build_instrument(track, instrm_def, nodes, freq) ?;
	output_nodes.push(instrm);

	Ok(())
}

fn build_instrument/* <'a> */(track: &/* 'a */ str, instrm_def: &NodeStructure, nodes: &mut NodeHost, freq: ChanneledNodeIndex) -> ModdlResult<ChanneledNodeIndex> {
	fn visit_struct(track: &str, strukt: &NodeStructure, nodes: &mut NodeHost, input: ChanneledNodeIndex) -> ModdlResult<ChanneledNodeIndex> {
		// 関数にするとライフタイム関係？のエラーが取れなかったので…
		macro_rules! recurse {
			($strukt: expr, $input: expr) => { visit_struct(track, $strukt, nodes, $input) }
		}
		// 関数にすると（同上）
		macro_rules! add_node {
			// トラックに属する node は全てトラック名のタグをつける
			($new_node: expr) => { Ok(nodes.add_with_tag(track.to_string(), $new_node)) }
		}
		let factories = node_factories();

		match strukt {
			NodeStructure::Connect(lhs, rhs) => {
				// TODO mono/stereo 変換
				let l_node = recurse!(lhs, input) ?;
				recurse!(rhs, l_node)
			},
			// NodeStructure::Power(lhs, rhs) => {
			// 	let l_node = recurse(lhs, input);
			// 	let r_node = recurse(rhs, input);
			// 	Box::new(Power::new()
			// },
			NodeStructure::Multiply(lhs, rhs) => {
				let l_node = recurse!(lhs, input) ?;
				let r_node = recurse!(rhs, input) ?;
				multiply(Some(track), nodes, l_node, r_node)
				// add_node!(Box::new(Mul::new(vec![l_node, r_node])))
			},
			// NodeStructure::Divide(lhs, rhs) => ,
			// NodeStructure::Remainder(lhs, rhs) => ,
			// NodeStructure::Add(lhs, rhs) => ,
			// NodeStructure::Subtract(lhs, rhs) => ,
			NodeStructure::Identifier(id) => {
				// id は今のところ引数なしのノード生成しかない
				let fact = factories.get(id).ok_or_else(|| Error::NodeFactoryNotFound) ?;
				add_node!(fact.create_node(&ValueArgs::new(), &NodeArgs::new(), input))
			},
			// NodeStructure::Lambda => ,
			NodeStructure::NodeWithArgs { factory, label: _label, args } => {
				// 引数ありのノード生成
				// 今のところ factory は id で直に指定するしかなく、id は factory の名前しかない
				let fact_id = match &**factory {
					NodeStructure::Identifier(id) => id,
					_ => unreachable!(),
				};
				let fact = factories.get(fact_id).ok_or_else(|| Error::NodeFactoryNotFound) ?;
				let node_names = fact.node_args();
				let mut node_args = NodeArgs::new();
				for name in node_names {
					let arg_val = & args.iter().find(|(n, _)| *n == *name )
							// 必要な引数が与えられていない
							.ok_or_else(|| Error::NodeFactoryNotFound)?.1;
					let st = arg_val.as_node_structure()
							// node_args に指定された引数なのに NodeStructure に変換できない
							.ok_or_else(|| Error::NodeFactoryNotFound) ?;
					let arg_node = recurse!(&st, input) ?;

					node_args.insert(name.clone(), arg_node);
				}
				let value_args: ValueArgs = args.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

				add_node!(fact.create_node(&value_args, &node_args, input))
			},
			NodeStructure::Constant(value) => add_node!(Box::new(Constant::new(*value))),

			_ => unimplemented!(),
		}
	}

	visit_struct(track, instrm_def, nodes, freq)
}

fn add(track: Option<&str>, nodes: &mut NodeHost,
	l_node: ChanneledNodeIndex, r_node: ChanneledNodeIndex) -> ModdlResult<ChanneledNodeIndex> {
		binary(track, nodes, l_node, r_node, Add::new, StereoAdd::new)
}
fn multiply(track: Option<&str>, nodes: &mut NodeHost,
		l_node: ChanneledNodeIndex, r_node: ChanneledNodeIndex) -> ModdlResult<ChanneledNodeIndex> {
	binary(track, nodes, l_node, r_node, Mul::new, StereoMul::new)
}


fn binary<M: 'static + Node, S: 'static + Node>(
	track: Option<&str>,
	nodes: &mut NodeHost,
	l_node: ChanneledNodeIndex,
	r_node: ChanneledNodeIndex,
	create_mono: fn (Vec<MonoNodeIndex>) -> M,
	create_stereo: fn (StereoNodeIndex, StereoNodeIndex) -> S
) -> ModdlResult<ChanneledNodeIndex> {
	macro_rules! add_node {
		// トラックに属する node は全てトラック名のタグをつける
		($new_node: expr) => {
			Ok(match track {
				Some(track) => nodes.add_with_tag(track.to_string(), $new_node),
				None => nodes.add($new_node),
			})
		}
	}
	match (l_node.channels(), r_node.channels()) {
		(1, 1) => {
			add_node!(Box::new(create_mono(vec![l_node.as_mono(), r_node.as_mono()])))
		},
		// 以下ステレオ対応
		// 現状ステレオまでしか対応していないが、任意のチャンネル数に拡張できるはず
		// （両者同数の場合はそれぞれで演算する。一方がモノラルの場合はそっちを拡張する。それ以外はエラー）
		(1, 2) => {
			let lhs_stereo: ModdlResult<ChanneledNodeIndex> = add_node!(Box::new(MonoToStereo::new(l_node.as_mono())));
			add_node!(Box::new(create_stereo(lhs_stereo?.as_stereo(), r_node.as_stereo())))
		},
		(2, 1) => {
			let rhs_stereo: ModdlResult<ChanneledNodeIndex> = add_node!(Box::new(MonoToStereo::new(r_node.as_mono())));
			add_node!(Box::new(create_stereo(l_node.as_stereo(), rhs_stereo?.as_stereo())))
		},
		(2, 2) => {
			add_node!(Box::new(create_stereo(l_node.as_stereo(), r_node.as_stereo())))
		},
		_ => Err(Error::ChannelMismatch),
	}
}

fn node_factories() -> HashMap<String, Box<dyn NodeFactory>> {
	let mut result = HashMap::<String, Box<dyn NodeFactory>>::new();
	macro_rules! add {
		($name: expr, $fact: expr) => {
			result.insert($name.to_string(), Box::new($fact));
		}
	};
	add!("sineOsc", SineOscFactory { });
	add!("limit", LimitFactory { });

	// for experiments
	add!("env1", Env1Factory { });
	add!("stereoTestOsc", StereoTestOscFactory { });

	result
}
