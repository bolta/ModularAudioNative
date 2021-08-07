#![allow(dead_code)]

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
	var::*,
};
use crate::seq::{
	instruction::*,
	sequencer::*,
	tick::*,
};

use crate::mml::default::{
	// default_mml_parser::*,
	sequence_generator::*,
};

use crate::moddl::player::*;
// use combine::Parser;

extern crate nom;


// fn main() {
// 	let mut machine = Machine::new();
// 	let mut context = Context::new(44100, 1);
// 	let mut nodes = NodeHost::new();

// 	let var = nodes.add_with_tag(String::from("freq"), Box::new(Var::new(0f32)));
// 	let sin = nodes.add(Box::new(SineOsc::new(var)));
// 	let env = nodes.add_with_tag(String::from("note"), Box::new(ExpEnv::new(0.125f32)));
// 	let vol = nodes.add(Box::new(Constant::new(0.5f32)));
// 	let master = nodes.add(Box::new(Mul::new(vec![sin, env, vol])));
// 	nodes.add(Box::new(PortAudioOut::new(master, &context)));

// 	let ast = compilation_unit().parse("o5l16 efgrgrcr>c8<arf8dre8r8d48e48d8.^-24cc2").unwrap().0;
// 	let seqs = generate_sequences(&ast, 96);

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

// use crate::moddl::parser::*;
fn main() {
	// println!("{:?}", hello_parser("hello"));
	// println!("{:?}", hello_parser("hello world"));
	// println!("{:?}", hello_parser("goodbye hello again"));
// 	println!("{:?}", real()("3.14"));
// 	println!("{:?}", real()("3.14cm"));
// 	println!("{:?}", real()("***3.14"));
	play(
r"
@tempo 144

a	o4l4q8
b	o3l4q8
c	o3l2q8

a	b>ef+<b> a2g+f+ ed+8e8f+ee2d+2 c+f+g+c+ b2ag+ f+f8g+8f+c+ g+2f+2<
b	b2.>c+8d+8ec+d+f+ c+d+8e8f+g+8a+8 bf+d+<b> f+2ff+8g+8 af+fb a2a+2 b2a2<
c	ed+ c+<b aa+ b1> ag+ f+f f+e d+<b>

");

}
