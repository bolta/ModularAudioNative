use crate::core::{
	node::*,
	node_handle::*,
	node_host::*,
};
use super::util::*;

pub fn sin_osc(freq: NodeHandle) -> NodeHandle {
	simple_osc("sin_osc", freq, |phase| phase.sin())
}

const TWO_PI: f32 = 2_f32 * std::f32::consts::PI;

// デューティ比を考慮する必要があるので別で実装
pub fn pulse_osc(freq: NodeHandle, duty: NodeHandle) -> NodeHandle {
	let freq_node = discard_lifetime(freq.node());
	let duty_node = discard_lifetime(duty.node());

	let mut phase = 0_f32;
	let host = freq.host_mut();
	let sample_rate = host.sample_rate() as f32;
	host.add_node(Node::from_closure_named(String::from("pulse_osc"), Box::new(move || {
		let value = if phase % TWO_PI < TWO_PI * duty_node.current() { 1_f32 } else { -1_f32 };
		phase += TWO_PI * freq_node.current() / sample_rate;
		phase %= TWO_PI;
		Some(value)
	})))
}

pub fn simple_osc(name: &'static str, freq: NodeHandle, func: fn (f32) -> f32) -> NodeHandle {
	let freq_node = discard_lifetime(freq.node());

	let mut phase = 0_f32;
	let host = freq.host_mut();
	let sample_rate = host.sample_rate() as f32;
	host.add_node(Node::from_closure_named(String::from(name), Box::new(move || {
		let value = func(phase);
		phase += TWO_PI * freq_node.current() / sample_rate;
		phase %= TWO_PI;
		Some(value)
	})))
}
