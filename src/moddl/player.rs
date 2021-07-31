use super::{
	ast::*,
	evaluator::*,
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
};

use combine::Parser;

// TODO エラー処理を全体的にちゃんとする

const TAG_SEQUENCER: &str = "seq";

pub fn play(moddl: &str) /* -> Result<(), (&str, nom::error::ErrorKind)> */ {
	// TODO パーズエラーをちゃんと処理
	let (_, CompilationUnit { statements }) = compilation_unit()(moddl).unwrap();

	let mut tempo = 120f32;
	let ticks_per_beat = 96; // TODO 外から渡せるように
	// トラックごとの MML を蓄積
	let mut mmls = BTreeMap::<String, String>::new();

	for stmt in statements { process_statement(&stmt, &mut tempo, &mut mmls)}
	
	let mut nodes = NodeHost::new();
	let tick = nodes.add(Box::new(Tick::new(tempo, ticks_per_beat, TAG_SEQUENCER.to_string())));

	let mut output_nodes = Vec::<NodeIndex>::new();
	for (track, mml) in mmls {
		build_nodes_by_mml(track.as_str(), mml.as_str(), &mut nodes, &mut output_nodes);
	}

	let mut context = Context::new(44100, 1); // TODO 値を外から渡せるように

	let mix = nodes.add(Box::new(Add::new(output_nodes)));
	let master_vol = nodes.add(Box::new(Constant::new(0.5f32))); // TODO 値を外から渡せるように
	let master = nodes.add(Box::new(Mul::new(vec![mix, master_vol])));
	nodes.add(Box::new(PortAudioOut::new(master, &context)));
	// nodes.add(Box::new(Print::new(master)));

	Machine::new().play(&mut context, &mut nodes);
}

fn process_statement(stmt: &Statement, tempo: &mut f32, mmls: &mut BTreeMap<String, String>) {
	match stmt {
		Statement::Directive { name, args } => {
			match name.as_str() {
				"tempo" => {
					// TODO ちゃんとエラー処理
					*tempo = evaluate_arg(&args, 0).as_float().unwrap();
				},
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

fn build_nodes_by_mml(track: &str, mml: &str, nodes: &mut NodeHost, output_nodes: &mut Vec<NodeIndex>) {
	// TODO ちゃんとエラー処理
	let ast = default_mml_parser::compilation_unit().parse(mml).unwrap().0;

	let tag_set = TagSet {
		freq: track.to_string(),
		note: track.to_string(),
	};
	let seqs = generate_sequences(&ast, 96, &tag_set);
	let _seqr = nodes.add_with_tag(TAG_SEQUENCER.to_string(), Box::new(Sequencer::new(seqs)));

	let freq = nodes.add_with_tag(track.to_string(), Box::new(Var::new(0f32)));

	// TODO instrument ディレクティブから生成
	let instrm = {
		// トラックに属する node は全てトラック名のタグをつける
		let osc = nodes.add_with_tag(track.to_string(), Box::new(SineOsc::new(freq)));
		let env = nodes.add_with_tag(track.to_string(), Box::new(ExpEnv::new(0.125f32)));
		nodes.add_with_tag(track.to_string(), Box::new(Mul::new(vec![osc, env])))
	};

	output_nodes.push(instrm);
}
