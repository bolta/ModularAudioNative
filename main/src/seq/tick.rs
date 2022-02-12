use crate::core::{
	common::*,
	context::*,
	event::*,
	machine::*,
	node::*,
};
use node_macro::node_impl;

pub struct Tick {
//	value: Sample,
	tempo: f32,
	ticks_per_bar: i32,
	timer: f32,
	target_tag: String,
}
impl Tick {
	pub fn new(tempo: f32, ticks_per_bar: i32, target_tag: String) -> Self {
		Self {
			tempo,
			ticks_per_bar,
			timer: 0f32,
			target_tag,
		}
	}
	fn tick(&self, env: &mut Environment) {
		env.events_mut().push(Box::new(TickEvent::new(EventTarget::Tag(self.target_tag.clone()))));
	}
}
#[node_impl]
impl Node for Tick {
	fn channels(&self) -> i32 { 0 }
	fn upstreams(&self) -> Upstreams { vec![] }
	fn initialize(&mut self, _context: &Context, env: &mut Environment) {
		// TODO 各サンプルの直前にイベントを投げれる機会を設けた方がいい。
		// 同じ処理を 2 回書いたりサンプル数に +1 したりしなくてよくなるように
		self.tick(env);
	}
	fn update(&mut self, _inputs: &Vec<Sample>, context: &Context, env: &mut Environment) {
		self.timer += self.tempo * self.ticks_per_bar as f32 / 240f32 / context.sample_rate_f32();
		while self.timer >= 1f32 {
			self.tick(env);
			self.timer -= 1f32;
		}
	}
}

pub struct TickEvent {
	// base: TargetedEventBase,
	target: EventTarget,
	// value: Sample,
}
impl TickEvent {
	pub fn new(target: EventTarget) -> Self {
		Self {
			target,
			// value
		}
	}
	// pub fn value(&self) -> Sample { self.value }
}
impl Event for TickEvent {
	// fn target_id(&self) -> Option<&String> { Some(&self.base.target_id) }
	fn target(&self) -> &EventTarget { &self.target }
	fn event_type(&self) -> &str { EVENT_TYPE_TICK }
}

pub const EVENT_TYPE_TICK: &str = "Tick::Tick";
