mod core;
use crate::core::node::*;
use crate::core::machine::*;

fn main() {
	let mut machine = Machine::new();

	let const_441 = machine.add_node(Box::new(Constant::new(441f32)));
	let sin = machine.add_node(Box::new(SineOsc::new(const_441)));
	let const_1 = machine.add_node(Box::new(Constant::new(1f32)));
	let sum = machine.add_node(Box::new(Add::new(vec![sin, const_1])));
	let _print = machine.add_node(Box::new(Print::new(sum)));

	machine.play();
}
