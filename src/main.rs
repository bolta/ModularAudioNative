//use core::node_host;
mod core;

use crate::core::node::*;
use crate::core::node_handle::*;
use crate::core::node_host::*;

fn main() {
	let mut host = NodeHost::new();
	let mut var = 0f32;
	let gen = host.add_node(Node::from_closure(Box::new(move || {
		let result = var;
		var += 0.01f32;
		Some(result)
	})));

	let neg = gen.negate();
	let konst = NodeHandle::constant(&mut host, 100f32);

	// 100 から減っていく
	let sum = neg.add(&konst);

	host.play();
}
