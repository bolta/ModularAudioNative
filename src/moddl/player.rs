use super::{
	ast::*,
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

pub fn play(moddl: &str) /* -> Result<(), (&str, nom::error::ErrorKind)> */ {
	// TODO パーズエラーをちゃんと処理
	let (_, CompilationUnit { statements }) = compilation_unit()(moddl).unwrap();

	let mut tempo = 120f32;
	let ticks_per_beat = 96; // TODO 外から渡せるように
	// トラックごとの instrument を保持
	// （イベントの発行順序が曖昧にならないよう BTreeMap で辞書順を保証）
	// let mut instruments = HashMap::<String, &Expr>::new();
	let mut instruments = HashMap::<String, NodeStructure>::new();
	// トラックごとの MML を蓄積
	let mut mmls = BTreeMap::<String, String>::new();

	for stmt in &statements { process_statement(&stmt, &mut tempo, &mut instruments, &mut mmls) }
	
	let mut nodes = NodeHost::new();
	let tick = nodes.add(Box::new(Tick::new(tempo, ticks_per_beat, TAG_SEQUENCER.to_string())));

	let mut output_nodes = Vec::<NodeIndex>::new();
	for (track, mml) in &mmls {
		// TODO instrument 未定義はエラー
		let instrm = instruments.get(track).unwrap();
		build_nodes_by_mml(track.as_str(), instrm, mml.as_str(), &mut nodes, &mut output_nodes);
	}

	let mut context = Context::new(44100, 1); // TODO 値を外から渡せるように

	let mix = nodes.add(Box::new(Add::new(output_nodes)));
	let master_vol = nodes.add(Box::new(Constant::new(0.5f32))); // TODO 値を外から渡せるように
	let master = nodes.add(Box::new(Mul::new(vec![mix, master_vol])));
	nodes.add(Box::new(PortAudioOut::new(master, &context)));
	// nodes.add(Box::new(Print::new(master)));

	Machine::new().play(&mut context, &mut nodes);
}

fn process_statement<'a>(
	stmt: &'a Statement,
	tempo: &mut f32,
	// instruments: &mut HashMap<String, &'a Expr>,
	instruments: &mut HashMap<String, NodeStructure>,
	mmls: &mut BTreeMap<String, String>,
) {
	match stmt {
		Statement::Directive { name, args } => {
			match name.as_str() {
				"tempo" => {
					// TODO ちゃんとエラー処理
					*tempo = evaluate_arg(&args, 0).as_float().unwrap();
				},
				"instrument" => {
					// TODO ちゃんとエラー処理
					let tracks = evaluate_arg(&args, 0).as_track_set().unwrap();
					// let instrm = & args[1];
					for track in tracks {
						let instrm = evaluate_arg(&args, 1).as_node_structure().unwrap();
						// TODO すでに入っていたらエラー
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
}

fn evaluate_arg(args: &Vec<Expr>, index: usize) -> Value {
	// TODO 範囲チェック
	evaluate(&args[index])
}

fn build_nodes_by_mml(track: &str, instrm_def: &NodeStructure, mml: &str, nodes: &mut NodeHost, output_nodes: &mut Vec<NodeIndex>) {
		// TODO ちゃんとエラー処理
	let ast = default_mml_parser::compilation_unit().parse(mml).unwrap().0;

	let tag_set = TagSet {
		freq: track.to_string(),
		note: track.to_string(),
	};
	let seqs = generate_sequences(&ast, 96, &tag_set);
	let _seqr = nodes.add_with_tag(TAG_SEQUENCER.to_string(), Box::new(Sequencer::new(seqs)));

	let freq = nodes.add_with_tag(track.to_string(), Box::new(Var::new(0f32)));

	let instrm = build_instrument(track, instrm_def, nodes, freq);
	output_nodes.push(instrm);
}

fn build_instrument(track: &str, instrm_def: &NodeStructure, nodes: &mut NodeHost, freq: NodeIndex) -> NodeIndex {
	fn visit_struct(track: &str, strukt: &NodeStructure, nodes: &mut NodeHost, input: NodeIndex) -> NodeIndex {
		// 関数にするとライフタイム関係？のエラーが取れなかったので…
		macro_rules! recurse {
			($strukt: expr, $input: expr) => { visit_struct(track, $strukt, nodes, $input) }
		}
		// 関数にすると（同上）
		macro_rules! add_node {
			// トラックに属する node は全てトラック名のタグをつける
			($new_node: expr) => { nodes.add_with_tag(track.to_string(), $new_node); }
		}
		let factories = node_factories();

		match strukt {
			NodeStructure::Connect(lhs, rhs) => {
				let l_node = recurse!(lhs, input);
				recurse!(rhs, l_node)
			},
			// NodeStructure::Power(lhs, rhs) => {
			// 	let l_node = recurse(lhs, input);
			// 	let r_node = recurse(rhs, input);
			// 	Box::new(Power::new()
			// },
			NodeStructure::Multiply(lhs, rhs) => {
				let l_node = recurse!(lhs, input);
				let r_node = recurse!(rhs, input);
				add_node!(Box::new(Mul::new(vec![l_node, r_node])))
			},
			// NodeStructure::Divide(lhs, rhs) => ,
			// NodeStructure::Remainder(lhs, rhs) => ,
			// NodeStructure::Add(lhs, rhs) => ,
			// NodeStructure::Subtract(lhs, rhs) => ,
			NodeStructure::Identifier(id) => {
				// id は今のところ引数なしのノード生成しかない
				// TODO エラー処理
				let fact = factories.get(id).unwrap();
				add_node!(fact.create_node(&ValueArgs::new(), &NodeArgs::new(), &vec![input]))
			},
			// NodeStructure::Lambda => ,
			NodeStructure::NodeWithArgs { factory, label: _label, args } => {
				// 引数ありのノード生成
				// 今のところ factory は id で直に指定するしかなく、id は factory の名前しかない
				let fact_id = match &**factory {
					NodeStructure::Identifier(id) => id,
					_ => unreachable!(),
				};
				// TODO エラー処理
				let fact = factories.get(fact_id).unwrap();
				let node_names = fact.node_args();
				let node_args: NodeArgs = node_names.iter().map(|name| {
					let arg_node = {
						// TODO エラー処理
						let arg_val = & args.iter().find(|(n, _)| *n == *name ).unwrap().1;
						let st = arg_val.as_node_structure().unwrap();
						recurse!(&st, input)
					};

					(name.clone(), arg_node)
				}).collect();
				let value_args: ValueArgs = args.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

				add_node!(fact.create_node(&value_args, &node_args, &vec![input]))
			},
			NodeStructure::Constant(value) => add_node!(Box::new(Constant::new(*value))),

			_ => unimplemented!(),
		}
	}

	visit_struct(track, instrm_def, nodes, freq)
}

fn node_factories() -> HashMap<String, Box<NodeFactory>> {
	let mut result = HashMap::<String, Box<NodeFactory>>::new();
	macro_rules! add {
		($name: expr, $fact: expr) => {
			result.insert($name.to_string(), Box::new($fact));
		}
	};
	add!("sineOsc", SineOscFactory { });
	add!("limit", LimitFactory { });
	add!("env1", Env1Factory { });
	// add!("expEnv", create_exp_env);

	result
}

