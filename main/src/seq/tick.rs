use crate::core::{common::*, context::*, event::*, machine::*, node::*};
use node_macro::node_impl;

pub struct Tick {
	timer: MonoNodeIndex,
	cycle: f32, // 必ず整数だが、常に Sample と比較するので Sample で保持しておく
	target_tag: String,
	prev_tick_no: Sample, // 必ず整数だが、常に Sample と比較するので Sample で保持しておく
}
impl Tick {
	pub fn new(timer: MonoNodeIndex, cycle: i32, target_tag: String) -> Self {
		Self {
			timer,
			cycle: cycle as Sample,
			target_tag,
			prev_tick_no: 0f32,
		}
	}
	fn tick(&self, env: &mut Environment) {
		env.post_event(Box::new(TickEvent::new(EventTarget::Tag(
			// TODO とても効率が悪そう
			self.target_tag.clone(),
		))));
	}
}
#[node_impl]
impl Node for Tick {
	fn channels(&self) -> i32 {
		0
	}
	fn upstreams(&self) -> Upstreams {
		vec![self.timer.channeled()]
	}
	fn activeness(&self) -> Activeness {
		Activeness::Active
	}
	fn initialize(&mut self, _context: &Context, env: &mut Environment) {
		// TODO 各サンプルの直前にイベントを投げれる機会を設けた方がいい。
		// 同じ処理を 2 回書いたりサンプル数に +1 したりしなくてよくなるように
		self.tick(env);
	}
	fn update(&mut self, inputs: &Vec<Sample>, _context: &Context, env: &mut Environment) {
		let timer = inputs[0].floor();
		while self.prev_tick_no < timer {
			self.tick(env);
			self.prev_tick_no += 1f32;
		}
		self.prev_tick_no %= self.cycle;
	}
}

pub struct TickTimer {
	tempo: MonoNodeIndex,
	ticks_per_bar: i32,
	cycle: f32,
	timer: f32,
}
impl TickTimer {
	pub fn new(tempo: MonoNodeIndex, ticks_per_bar: i32, cycle: i32) -> Self {
		Self {
			tempo,
			ticks_per_bar,
			cycle: cycle as Sample,
			timer: 0f32,
		}
	}
}
#[node_impl]
impl Node for TickTimer {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![self.tempo.channeled()] }
	fn activeness(&self) -> Activeness { Activeness::Active }
	fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut [Sample], _context: &Context, _env: &mut Environment) {
		output_mono(output, self.timer);
	}
	fn update(&mut self, inputs: &Vec<Sample>, context: &Context, _env: &mut Environment) {
		let tempo = inputs[0];
		self.timer = self.timer % self.cycle + tempo * self.ticks_per_bar as f32 / 240f32 / context.sample_rate_f32();
	}
}

#[derive(Clone)]
pub struct TickEvent {
	target: EventTarget,
}
impl TickEvent {
	pub fn new(target: EventTarget) -> Self {
		Self {
			target,
		}
	}
}
impl Event for TickEvent {
	fn target(&self) -> &EventTarget {
		&self.target
	}
	fn event_type(&self) -> &str {
		EVENT_TYPE_TICK
	}
	fn clone_event(&self) -> Box<dyn Event> { clone_event(self) }
}

pub const EVENT_TYPE_TICK: &str = "Tick::Tick";

pub struct ExperGroove {
	timer: MonoNodeIndex,
}
impl ExperGroove {
	pub fn new(timer: MonoNodeIndex) -> Self { Self { timer } }
}
#[node_impl]
impl Node for ExperGroove {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![self.timer.channeled()] }
	fn activeness(&self) -> Activeness { Activeness::Passive }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut [Sample], _context: &Context, _env: &mut Environment) {
		let timer = inputs[0];
		fn l(nth: f32) -> f32 { 384f32 / nth }
		// c16c16 -> c12c24
		let result = if timer < l(16f32) {
			timer * l(24f32) / l(16f32)
		} else {
			timer * l(12f32) / l(16f32) - l(24f32)
		};
		output_mono(output, result);
	}
}
