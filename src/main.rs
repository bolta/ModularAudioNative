mod core;
mod mml;
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
	default_mml_parser::*,
	sequence_generator::*,
};

use combine::Parser;

fn main() {
	let mut machine = Machine::new();
	let mut context = Context::new(44100, 1);
	let mut nodes = NodeHost::new();

	// let a4 = nodes.add(Box::new(Constant::new(440f32)));
	// let sinA4 = nodes.add(Box::new(SineOsc::new(a4)));
	// let e5 = nodes.add(Box::new(Constant::new(1.5 * 440f32)));
	// let sinE5 = nodes.add(Box::new(SineOsc::new(e5)));
	// // let const_1 = nodes.add(Box::new(Constant::new(1f32)));
	// // let sum = nodes.add(Box::new(Add::new(vec![sin, const_1])));
	// // let _print = nodes.add(Box::new(Print::new(sum)));
	// // let _print = nodes.add(Box::new(PrsinA4::new(sin)));
	// let sum = nodes.add(Box::new(Add::new(vec![sinA4, sinE5])));
	// let vol = nodes.add(Box::new(Constant::new(0.5f32)));
	// let master = nodes.add(Box::new(Mul::new(vec![sum, vol])));
	// nodes.add(Box::new(PortAudioOut::new(master, &context)));

	// let mut sched = Box::new(EventScheduler::new());
	// sched.add_event(3 * 44100, Box::new(TerminateEvent { }));
	// nodes.add(sched);

	// let var = nodes.add_with_tag(String::from("freq"), Box::new(Var::new(440f32)));
	// let sin = nodes.add(Box::new(SineOsc::new(var)));
	// let vol = nodes.add(Box::new(Constant::new(0.5f32)));
	// let master = nodes.add(Box::new(Mul::new(vec![sin, vol])));
	// nodes.add(Box::new(PortAudioOut::new(master, &context)));

	// let mut sched = Box::new(EventScheduler::new());
	// sched.add_event(0, Box::new(SetEvent::new(EventTarget::Tag(String::from("freq")), 660f32)));
	// sched.add_event(44100, Box::new(SetEvent::new(EventTarget::Tag(String::from("freq")), 880f32)));
	// sched.add_event(2 * 44100, Box::new(TerminateEvent { }));
	// nodes.add(sched);

	let var = nodes.add_with_tag(String::from("freq"), Box::new(Var::new(0f32)));
	let sin = nodes.add(Box::new(SineOsc::new(var)));
	let env = nodes.add_with_tag(String::from("note"), Box::new(ExpEnv::new(0.125f32)));
	let vol = nodes.add(Box::new(Constant::new(0.5f32)));
	let master = nodes.add(Box::new(Mul::new(vec![sin, env, vol])));
	nodes.add(Box::new(PortAudioOut::new(master, &context)));

	let ast = compilation_unit().parse("o5l16 efgrgrcr>c8<arf8dre8r8d48e48d8.^-24cc2").unwrap().0;
	let seqs = generate_sequences(&ast, 96);

	let tick = nodes.add(Box::new(Tick::new(80f32, 112, "seq".to_string())));
	let seqr = nodes.add_with_tag("seq".to_string(), Box::new(Sequencer::new(seqs)));

	let mut sched = Box::new(EventScheduler::new());
	sched.add_event(5 * context.sample_rate(), Box::new(TerminateEvent { }));
	nodes.add(sched);

	machine.play(&mut context, &mut nodes);
}
