//use core::node_host;
mod core;

use crate::core::node::*;
use crate::core::node_handle::*;
use crate::core::node_host::*;

fn main() {
	let mut host = NodeHost::new();
	// let mut var = 0f32;
	// let gen = host.add_node(Node::from_closure(Box::new(move || {
	// 	let result = var;
	// 	var += 0.01f32;
	// 	Some(result)
	// })));

	// let neg = gen.negate();
	// let konst = NodeHandle::constant(&mut host, 100f32);

	// // 100 から減っていく
	// let sum = neg.add(&konst);

	const SMP_RATE: i32 = 44100;
	const QUANT_RATE: i32 = 16;
	const CHANNELS: i32 = 1;
	let freq = 440_f32;

	let mut phase = 0_f32;

	let osc = host.add_node(Node::from_closure(Box::new(move || {
		let value = phase.sin();
		phase += (2_f32 * std::f32::consts::PI) * freq / (SMP_RATE as f32);
		phase %= 2_f32 * std::f32::consts::PI;
		Some(value)
	})));

	let host_shared = std::rc::Rc::new(std::cell::RefCell::new(host));
	NodeHost::play(host_shared);
}

