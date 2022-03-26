use crate::{
	core::{
		machine::*,
	},
	wave::waveform_host::*,
};

use crate::core::{
	common::*,
	context::*,
	node_host::*,
};

use crate::node::{
	arith::*,
	event_scheduler::*,
	prim::*,
	util::*,
};

#[cfg(test)]
pub fn discard_lifetime<'a, T>(r: &'a T) -> &'static T {
	unsafe { &* (r as *const T) }
}
#[cfg(test)]
pub fn discard_lifetime_mut<'a, T>(r: &'a mut T) -> &'static mut T {
	unsafe { &mut * (r as *mut T) }
}

// TODO ステレオ
#[cfg(test)]
fn play_test<MakeNode: Fn (&mut NodeHost) -> ChanneledNodeIndex>(make_node: MakeNode) -> Vec<Sample> {
	let mut context = Context::new(44100);
	let mut nodes = NodeHost::new();
	let node = make_node(&mut nodes);
	let mut result: Vec<Sample> = vec![];
	let out = MemoryOut::new(/* add */node, discard_lifetime_mut(&mut result)); // ひどい
	nodes.add(Box::new(out));
	let mut sched = EventScheduler::new();
	sched.add_event(1, Box::new(TerminateEvent { }));
	nodes.add(Box::new(sched));

	let mut waveforms = WaveformHost::new();
	let mut machine = Machine::new();
	machine.play(&mut context, &mut nodes, &mut waveforms);

	result
}

#[cfg(test)]
macro_rules! test_binary_op {
	($ctor: expr, $lhs: expr, $rhs: expr, $expected: expr) => {
		let result = play_test(|nodes| {
			let lhs = nodes.add(Box::new(Constant::new($lhs))).as_mono();
			let rhs = nodes.add(Box::new(Constant::new($rhs))).as_mono();
			let add = nodes.add(Box::new($ctor(lhs, rhs)));
			add
		});
		assert_eq!(result[0], $expected);
	}
}

#[cfg(test)]
#[test]
fn test() {
	test_binary_op!(Add::new, 2f32, 3f32, 5f32);

	test_binary_op!(Less::new, 2f32, 3f32, 1f32);
	test_binary_op!(Less::new, 2f32, 2f32, -1f32);
	test_binary_op!(Less::new, 2f32, 1f32, -1f32);

	test_binary_op!(LessOrEqual::new, 2f32, 3f32, 1f32);
	test_binary_op!(LessOrEqual::new, 2f32, 2f32, 1f32);
	test_binary_op!(LessOrEqual::new, 2f32, 1f32, -1f32);

	test_binary_op!(Equal::new, 2f32, 3f32, -1f32);
	test_binary_op!(Equal::new, 2f32, 2f32, 1f32);
	test_binary_op!(Equal::new, 2f32, 1f32, -1f32);

	test_binary_op!(NotEqual::new, 2f32, 3f32, 1f32);
	test_binary_op!(NotEqual::new, 2f32, 2f32, -1f32);
	test_binary_op!(NotEqual::new, 2f32, 1f32, 1f32);

	test_binary_op!(Greater::new, 2f32, 3f32, -1f32);
	test_binary_op!(Greater::new, 2f32, 2f32, -1f32);
	test_binary_op!(Greater::new, 2f32, 1f32, 1f32);

	test_binary_op!(GreaterOrEqual::new, 2f32, 3f32, -1f32);
	test_binary_op!(GreaterOrEqual::new, 2f32, 2f32, 1f32);
	test_binary_op!(GreaterOrEqual::new, 2f32, 1f32, 1f32);

	test_binary_op!(And::new, -1f32, -1f32, -1f32);
	test_binary_op!(And::new, -1f32, 1f32, -1f32);
	test_binary_op!(And::new, 1f32, -1f32, -1f32);
	test_binary_op!(And::new, 1f32, 1f32, 1f32);

	test_binary_op!(Or::new, -1f32, -1f32, -1f32);
	test_binary_op!(Or::new, -1f32, 1f32, 1f32);
	test_binary_op!(Or::new, 1f32, -1f32, 1f32);
	test_binary_op!(Or::new, 1f32, 1f32, 1f32);
}
