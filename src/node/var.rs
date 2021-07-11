use crate::core::{
	common::*,
	context::*,
	event::*,
	machine::*,
	node::*,
};

pub struct Var {
	value: Sample,
}

impl Var {
	pub fn new(value: Sample) -> Self { Self { value } }
}
impl Node for Var {
	fn upstreams(&self) -> Vec<NodeIndex> { vec![] }
	fn execute(&mut self, _inputs: &Vec<Sample>, context: &Context, env: &mut Environment) -> Sample { self.value }

	fn process_event(&mut self, event: &dyn Event) {
		if event.event_type() != EVENT_TYPE_SET { return; }

		let event = event.downcast_ref::<SetEvent>().unwrap();
		self.value = event.value();
	}
}

pub struct SetEvent {
	base: TargetedEventBase,
	value: Sample,
}
impl SetEvent {
	pub fn new(target_id: String, value: Sample) -> Self {
		SetEvent {
			base: TargetedEventBase::new(target_id),
			value
		}
	}
	pub fn value(&self) -> Sample { self.value }
}
impl Event for SetEvent {
	fn target_id(&self) -> Option<&String> { Some(&self.base.target_id) }
	fn event_type(&self) -> &str { EVENT_TYPE_SET }
}

const EVENT_TYPE_SET: &str = "Var::Set";
