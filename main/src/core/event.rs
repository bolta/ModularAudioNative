use mopa::mopafy;

pub trait Event: mopa::Any {
	// TODO 名前空間を規定する
	fn event_type(&self) -> &str;
	fn target(&self) -> &EventTarget;
}
mopafy!(Event);

// pub struct TargetedEventBase {
// 	pub target_id: String,
// }
// impl TargetedEventBase {
// 	pub fn new(target_id: String) -> Self { Self { target_id } }
// }

#[derive(Clone)]
pub enum EventTarget {
	Machine,
	Tag(String),
}
