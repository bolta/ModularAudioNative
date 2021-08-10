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


// fn main() {
// 	let result = moddl::parser::compilation_unit().parse(
// r"@tempo 120 * 2, 222

// ");

// 	match result {
// 		Ok((moddl::ast::CompilationUnit { statements }, s)) => {
// 			println!("{:?}", statements);
// 			println!("{:?}", s);
// 		}
// 		Err(e) => {
// 			println!("error: {:?}", e);
// 		}
// 	}
// }

use crate::moddl::parser::*;
fn main() {
	if let Err(e) = play() {
		format!("error: {:?}", e);
	}
}

fn play() -> ModdlResult<()> {
	player::play(r"
@tempo 144
@instrument ^ab, stereoTestOsc * env1
@instrument ^c, stereoTestOsc * env1


a	o4l4q8
b	o3l4q8
c	o3l2q8

a	b>ef+<b> a2g+f+ ed+8e8f+ee2d+2 c+f+g+c+ b2ag+ f+f8g+8f+c+ g+2f+2<
b	b2.>c+8d+8ec+d+f+ c+d+8e8f+g+8a+8 bf+d+<b> f+2ff+8g+8 af+fb a2a+2 b2a2<
c	ed+ c+<b aa+ b1> ag+ f+f f+e d+<b>

a	b>ef+<b> a2g+f+ ed+8e8f+ee2d+2 c+f+g+c+ b2ag+ f+f8g+8f+c+ g+2f+f+16g+16a16b16>
b	b2.>c+8d+8ec+d+f+ c+d+8e8f+g+8a+8 bf+d+<b> f+2ff+8g+8 af+fb a2a+2 b2a2
c	ed+ c+c c+<a+ b1> ag+ f+f f+e d+<b>

a	c+2^8r8c+< b2^8r8g+ aa8g+8f+f f+g+ab>
b	l8 e4ag+a4e4 e4bag+f+eg+> d<af+4g+c+d+f f+4<b>c+def+4
c	l8 ar<ab>c+d+ef+ g+rg+f+g+rer f+4d4c+4bg+ f+4e4d4c+4<

a	d2^8r8d c+2^8r8<a bb8>c8<bb8>c8< bag+f+<
b	g2^gab a2^agf+ e4edcde4 d+2e4f+4<
c	br>babrgr f+r>c+r<ar<ab> cr>cr<ef+g4 f+4d+4<b4a4>
")
}
