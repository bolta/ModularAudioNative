use crate::core::{
	common::*,
	context::*,
	event::*,
	machine::*,
	node::*,
	node_factory::*,
};
use node_macro::node_impl;

pub struct ExpEnv {
	ratio_per_sec: MonoNodeIndex,

	amplitude: Sample,
	state: ExpEnvState,
}
impl ExpEnv {
	pub fn new(ratio_per_sec: MonoNodeIndex) -> Self {
		Self {
			ratio_per_sec,
			amplitude: 0f32,
			state: ExpEnvState::Idle,
		}
	}
}
#[node_impl]
impl Node for ExpEnv {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![self.ratio_per_sec.channeled()] }
	fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut Vec<Sample>, _context: &Context, _env: &mut Environment) {
		output_mono(output, self.amplitude);
	}
	fn update(&mut self, inputs: &Vec<Sample>, context: &Context, _env: &mut Environment) {
		if self.state == ExpEnvState::Idle { return; }

		// TODO 入力が変わらないときは計算しないように
		let ratio_per_sample = inputs[0].powf(1f32 / context.sample_rate_f32());
		self.amplitude *= ratio_per_sample;
		if self.amplitude < AMPLITUDE_MIN {
			self.amplitude = 0f32;
			self.state = ExpEnvState::Idle;
		}
	}
	fn process_event(&mut self, event: &dyn Event, _context: &Context, _env: &mut Environment) {
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

pub struct ExpEnvFactory { }
impl NodeFactory for ExpEnvFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![spec_with_default("ratioPerSec", 1, 0.125f32)] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, node_args: &NodeArgs, _piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		Box::new(ExpEnv::new(node_args.get("ratioPerSec").unwrap().as_mono()))
	}
}

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

pub const EVENT_TYPE_NOTE: &str = "Env::Note";
