use super::common::*;

pub trait Event {
	// TODO 名前空間を規定する
	fn event_type(&self) -> &str;
}

pub struct SetVar {
	name: String,
	value: Sample,
}
impl SetVar {
	pub fn new(name: String, value: Sample) -> Self {
		Self { name, value }
	}
	pub fn name(&self) -> &String { &self.name }
	pub fn value(&self) -> Sample { self.value }
}
impl Event for SetVar {
	fn event_type(&self) -> &str { "SetVar" }
}
