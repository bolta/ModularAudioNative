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
