#![allow(dead_code)]
#![type_length_limit="106079600"]

mod core;
mod mml;
mod moddl;
mod node;
mod seq;

use crate::core::{
	context::*,
	machine::*,
	node_host::*,
	event::*
};
use crate::node::{
	arith::*,
	audio::*,
	env::*,
	event_scheduler::*,
	osc::*,
	prim::*,
	stereo::*,
	var::*,
};
use crate::seq::{
	instruction::*,
	sequencer::*,
	tick::*,
};

use crate::mml::default::{
	default_mml_parser::*,
	sequence_generator::*,
};

use crate::moddl::{
	error::*,
	player,
};

use std::{
	env,
	process,
};
use combine::Parser;

extern crate nom;

// fn main() {
// 	let mut machine = Machine::new();
// 	let mut context = Context::new(44100/* , 2 */);
// 	let mut nodes = NodeHost::new();

// 	let var = nodes.add_with_tag(String::from("freq"), Box::new(Var::new(0f32))).as_mono();
// 	let osc = nodes.add(Box::new(StereoTestOsc::new(var))).as_stereo();
// 	// let env = nodes.add_with_tag(String::from("note"), Box::new(ExpEnv::new(0.125f32)));
// 	let vol = nodes.add(Box::new(Constant::new(0.5f32)));
// 	let vol_stereo = nodes.add(Box::new(MonoToStereo::new(vol.as_mono()))).as_stereo();
// 	// let master = nodes.add(Box::new(Mul::new(vec![osc, env, vol])));
// 	// let master = osc.channeled();
// 	let master = nodes.add(Box::new(StereoMul::new(osc, vol_stereo)));
// 	nodes.add(Box::new(PortAudioOut::new(master, &context)));

// 	let ast = compilation_unit().parse("o5l16 efgrgrcr>c8<arf8dre8r8d48e48d8.^-24cc2").unwrap().0;
// 	let tag_set = TagSet {
// 		freq: "freq".to_string(),
// 		note: "note".to_string(),
// 	};
// 	let seqs = generate_sequences(&ast, 96, &tag_set);

// 	let tick = nodes.add(Box::new(Tick::new(80f32, 112, "seq".to_string())));
// 	let seqr = nodes.add_with_tag("seq".to_string(), Box::new(Sequencer::new(seqs)));

// 	let mut sched = Box::new(EventScheduler::new());
// 	sched.add_event(5 * context.sample_rate(), Box::new(TerminateEvent { }));
// 	nodes.add(sched);

// 	machine.play(&mut context, &mut nodes);
// }

use crate::moddl::parser::*;
fn main() {
	if let Err(e) = play() {
		println!("An error occurred: {:?}", e);
	}
}
/*
@instrument ^ab, stereoTestOsc | limit { min: 0.5 | sineOsc / 2 - 0.5, max: 1 } * env1
@instrument ^c, stereoTestOsc * env1 * 0
 */
fn play() -> ModdlResult<()> {
	match env::args().nth(1) {
		None => {
			eprintln!("Please specify the moddl file path.");
			process::exit(1);
		}
		Some(moddl_path) => player::play_file(moddl_path.as_str()),
	}
}
