use crate::core::{
	common::*,
	context::*,
	event::*,
	machine::*,
	node::*,
};

pub struct ExpEnv {
	amplitude: Sample,
	state: ExpEnvState,
	// TODO 可変にする
	ratio_per_sec: f32,
	ratio_per_sample: f32,
}
impl ExpEnv {
	pub fn new(ratio_per_sec: f32) -> Self {
		Self {
			amplitude: 0f32,
			state: ExpEnvState::Idle,
			ratio_per_sec,
			ratio_per_sample: f32::NAN,
		}
	}
}
impl Node for ExpEnv {
	fn initialize(&mut self, context: &Context, env: &mut Environment) {
		// TODO 無駄に状態をもつのがいやだが…
		self.ratio_per_sample = self.ratio_per_sec.powf(1f32 / context.sample_rate_f32());
	}
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut Vec<Sample>, context: &Context, env: &mut Environment) {
		output_mono(output, self.amplitude);
	}
	fn update(&mut self, _inputs: &Vec<Sample>, context: &Context, env: &mut Environment) {
		if self.state == ExpEnvState::Idle { return; }

		self.amplitude *= self.ratio_per_sample;
		if self.amplitude < AMPLITUDE_MIN {
			self.amplitude = 0f32;
			self.state = ExpEnvState::Idle;
		}
	}
	fn process_event(&mut self, event: &dyn Event, context: &Context, env: &mut Environment) {
		if event.event_type() != EVENT_TYPE_NOTE { return; }

		let event = event.downcast_ref::<NoteEvent>().unwrap();
		if event.note_on() {
			self.amplitude = 1f32;
			self.state = ExpEnvState::Note;
		} else {
			self.amplitude = 0f32;
			self.state = ExpEnvState::Idle;
		}
	}

}
#[derive(Eq, PartialEq)] enum ExpEnvState { Idle, Note }
const AMPLITUDE_MIN: f32 = 1f32 / 65536f32;

// TODO 置き場所ここでいいのか？
pub struct NoteEvent {
	target: EventTarget,
	note_on: bool,
}
impl NoteEvent {
	pub fn new(target: EventTarget, note_on: bool) -> Self {
		Self {
			target,
			note_on,
		}
	}
	pub fn note_on(&self) -> bool { self.note_on }
}
impl Event for NoteEvent {
	fn target(&self) -> &EventTarget { &self.target }
	fn event_type(&self) -> &str { EVENT_TYPE_NOTE }
}

const EVENT_TYPE_NOTE: &str = "Env::Note";
