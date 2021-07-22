mod core;
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
	let env = nodes.add_with_tag(String::from("env"), Box::new(ExpEnv::new(0.125f32)));
	let vol = nodes.add(Box::new(Constant::new(0.5f32)));
	let master = nodes.add(Box::new(Mul::new(vec![sin, env, vol])));
	nodes.add(Box::new(PortAudioOut::new(master, &context)));

	let tick = nodes.add(Box::new(Tick::new(80f32, 96, "seq".to_string())));
	let seq = nodes.add_with_tag("seq".to_string(), Box::new(Sequencer::new(vec![
		Instruction::Value { tag: String::from("freq"), value: 440f32 },
		Instruction::Note { tag: String::from("env"), note_on: true },
		Instruction::Wait(48),
		Instruction::Value { tag: String::from("freq"), value: 493.8833012561241f32 },
		Instruction::Note { tag: String::from("env"), note_on: true },
		Instruction::Wait(24),
		Instruction::Note { tag: String::from("env"), note_on: false },
		Instruction::Wait(24),
		Instruction::Value { tag: String::from("freq"), value: 554.3652619537442f32 },
		Instruction::Note { tag: String::from("env"), note_on: true },
		Instruction::Wait(48),
		Instruction::Value { tag: String::from("freq"), value: 587.3295358348151f32 },
		Instruction::Note { tag: String::from("env"), note_on: true },
		Instruction::Wait(24),
		Instruction::Note { tag: String::from("env"), note_on: false },
		Instruction::Wait(24),
		Instruction::Value { tag: String::from("freq"), value: 659.2551138257398f32 },
		Instruction::Note { tag: String::from("env"), note_on: true },
	])));

	let mut sched = Box::new(EventScheduler::new());
	sched.add_event(5 * context.sample_rate(), Box::new(TerminateEvent { }));
	nodes.add(sched);

	machine.play(&mut context, &mut nodes);
}
