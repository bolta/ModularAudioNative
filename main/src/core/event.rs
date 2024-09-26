use mopa::mopafy;

pub trait Event: mopa::Any + Send {
	// TODO 名前空間を規定する
	fn event_type(&self) -> &str;
	fn target(&self) -> &EventTarget;
	fn clone_event(&self) -> Box<dyn Event>;
}
mopafy!(Event);

// pub struct TargetedEventBase {
// 	pub target_id: String,
// }
// impl TargetedEventBase {
// 	pub fn new(target_id: String) -> Self { Self { target_id } }
// }

#[derive(Clone, Debug)]
pub enum EventTarget {
	Machine,
	Tag(String),
}

// TODO ファイル分ける

pub fn clone_event<E>(e: &E) -> Box<dyn Event>
where E: Event + Clone {
	Box::new(e.clone())
}

use super::common::SampleCount;
use std::sync::mpsc::Sender;

/// イベントをマシンを越えてやりとりする際のラッパー。
/// 正しいタイミングで処理できるよう、発生したタイミングの情報を持つ
// #[derive(Clone)]
pub struct GlobalEvent {
	elapsed_samples: SampleCount,
	event: Box<dyn Event>,
}
impl GlobalEvent {
	pub fn new(elapsed_samples: SampleCount, event: Box<dyn Event>) -> Self {
		Self { elapsed_samples, event }
	}
	pub fn elapsed_samples(&self) -> SampleCount { self.elapsed_samples }
	pub fn event(self) -> Box<dyn Event> { self.event }
	pub fn debug_string(&self) -> String {
		format!("{}: {} ({:?})", self.elapsed_samples(), self.event.event_type(), self.event.target())
	}
}
impl Clone for GlobalEvent {
	fn clone(&self) -> Self {
		Self::new(self.elapsed_samples, self.event.clone_event())
	}
}

#[derive(Clone)]
pub struct Broadcaster {
	senders: Vec<Sender<GlobalEvent>>,
}
impl Broadcaster {
	pub fn new(senders: Vec<Sender<GlobalEvent>>) -> Self {
		Self { senders }
	}
	pub fn broadcast(&self, event: GlobalEvent) {
		for sender in &self.senders {
			// TODO エラー処理必要？
			sender.send(event.clone());
		}
	}
}
