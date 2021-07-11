mod core;
mod node;

use crate::core::{
	context::*,
	machine::*,
	node_host::*,
};
use crate::node::{
	arith::*,
	audio::*,
	event_scheduler::*,
	osc::*,
	prim::*,
	var::*,
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

	let var = nodes.add_with_id("freq", Box::new(Var::new(440f32)));
	let sin = nodes.add(Box::new(SineOsc::new(var)));
	let vol = nodes.add(Box::new(Constant::new(0.5f32)));
	let master = nodes.add(Box::new(Mul::new(vec![sin, vol])));
	nodes.add(Box::new(PortAudioOut::new(master, &context)));

	let mut sched = Box::new(EventScheduler::new());
	sched.add_event(44100, Box::new(SetEvent::new(String::from("freq"), 880f32)));
	sched.add_event(3 * 44100, Box::new(TerminateEvent { }));
	nodes.add(sched);

	machine.play(&mut context, &mut nodes);
}
