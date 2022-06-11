use crate::core::{
	common::*,
	context::*,
	event::*,
	machine::*,
	node::*,
};
use node_macro::node_impl;

pub struct Var {
	value: Sample,
}

impl Var {
	pub fn new(value: Sample) -> Self { Self { value } }
}
#[node_impl]
impl Node for Var {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![] }
	fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut [Sample], _context: &Context, _env: &mut Environment) {
		output_mono(output, self.value);
	}

	fn process_event(&mut self, event: &dyn Event, _context: &Context, _env: &mut Environment) {
		if event.event_type() != EVENT_TYPE_SET { return; }

		let event = event.downcast_ref::<SetEvent>().unwrap();
		self.value = event.value();
	}
}

pub struct SetEvent {
	// base: TargetedEventBase,
	target: EventTarget,
	value: Sample,
}
impl SetEvent {
	pub fn new(target: EventTarget, value: Sample) -> Self {
		Self {
			target,
			value
		}
	}
	pub fn value(&self) -> Sample { self.value }
}
impl Event for SetEvent {
	fn target(&self) -> &EventTarget { &self.target }
	fn event_type(&self) -> &str { EVENT_TYPE_SET }
}

const EVENT_TYPE_SET: &str = "Var::Set";
