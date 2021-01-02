use crate::core::{
	node::*,
	node_handle::*,
	node_host::*,
};
use super::util::*;

pub fn sin_osc(freq: NodeHandle) -> NodeHandle {
	let freq_node = discard_lifetime(freq.node());

	let mut phase = 0_f32;
	let host = freq.host_mut();
	let sample_rate = host.sample_rate() as f32;
	host.add_node(Node::from_closure_named(String::from("sin_osc"), Box::new(move || {
		let value = phase.sin();
		phase += (2_f32 * std::f32::consts::PI) * freq_node.current() / sample_rate;
		phase %= 2_f32 * std::f32::consts::PI;
		Some(value)
	})))
}
