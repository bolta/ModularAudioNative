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
	let mut instruments = HashMap::<String, &Expr>::new();
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
	instruments: &mut HashMap<String, &'a Expr>,
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
					let instrm = & args[1];
					for track in tracks {
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

fn build_nodes_by_mml(track: &str, instrm_def: &Expr, mml: &str, nodes: &mut NodeHost, output_nodes: &mut Vec<NodeIndex>) {
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

fn build_instrument(track: &str, instrm_def: &Expr, nodes: &mut NodeHost, freq: NodeIndex) -> NodeIndex {

	// let mut instrm: Option<NodeIndex> = None;
	fn visit_expr(track: &str, expr: &Expr, nodes: &mut NodeHost, freq: NodeIndex) -> NodeIndex {
		let mut recurse = |x| visit_expr(track, x, nodes, freq);
		let factories = node_factories();
		let new_node = match expr {
			Expr::Connect { lhs, rhs } => {
				let l_res = recurse(lhs);
				
				let r_val = evaluate(rhs);
				// TODO 右辺がただの識別子であることのみ想定している。引数つきも要対応
				let fact_name = r_val.as_identifier().unwrap();
				// TODO エラー処理
				let fact = factories.get(fact_name.as_str()).unwrap();
				let args = HashMap::<String, Value>::new();
				fact(&args, &vec![l_res])
			},
			Expr::Add { lhs, rhs } => Box::new(Add::new(vec![recurse(lhs), recurse(rhs)])),
			Expr::Multiply { lhs, rhs } => Box::new(Mul::new(vec![recurse(lhs), recurse(rhs)])),
			// TODO and more binary opers
			Expr::Identifier(id) => {
				// 現在、名前は全て factory の名前だが、一般的な変数定義も実装するとこの辺は変わる
				// TODO エラー処理
				let fact = factories.get(id.as_str()).unwrap();
				let args = HashMap::<String, Value>::new();
				// Connect の rhs である場合は Connect の中で対応済み。
				// それ以外のケースでは freq の供給が必要
				fact(&args, &vec![freq])
			},
			_ => unimplemented!(),
		};

		// トラックに属する node は全てトラック名のタグをつける
		nodes.add_with_tag(track.to_string(), new_node)
	};
	visit_expr(track, instrm_def, nodes, freq)
}

fn node_factories() -> HashMap<String, Box<NodeFactory>> {
	let mut result = HashMap::<String, Box<NodeFactory>>::new();
	macro_rules! add {
		($name: expr, $fact: expr) => {
			result.insert($name.to_string(), Box::new($fact));
		}
	};
	add!("sineOsc", create_sine_osc);
	add!("limit", create_limit);
	add!("env1", create_env1);

	result
}

