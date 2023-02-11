use crate::core::{
	common::*,
	context::*,
	event::*,
	machine::*,
	node::*,
	node_factory::*,
};
use node_macro::node_impl;

 ////
//// Exponential Envelope

pub struct ExpEnv {
	base_: NodeBase,
	ratio_per_sec: MonoNodeIndex,

	amplitude: Sample,
	state: ExpEnvState,
}
impl ExpEnv {
	pub fn new(base: NodeBase, ratio_per_sec: MonoNodeIndex) -> Self {
		Self {
			base_: base,
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
	fn activeness(&self) -> Activeness { Activeness::Active }
	fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut [Sample], _context: &Context, _env: &mut Environment) {
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
	fn create_node(&self, base: NodeBase, node_args: &NodeArgs, _piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		Box::new(ExpEnv::new(base, node_args.get("ratioPerSec").unwrap().as_mono()))
	}
}

 ////
//// ADSR Envelope

pub struct AdsrEnv {
	base_: NodeBase,
	attack_time: MonoNodeIndex,
	decay_time: MonoNodeIndex,
	sustain_level: MonoNodeIndex,
	release_time: MonoNodeIndex,
	initial_level: MonoNodeIndex,

	amplitude: Sample,
	state: AdsrEnvState,
}
impl AdsrEnv {
	pub fn new(
		base: NodeBase, 
		attack_time: MonoNodeIndex, 
		decay_time: MonoNodeIndex,
		sustain_level: MonoNodeIndex,
		release_time: MonoNodeIndex,
		initial_level: MonoNodeIndex, 
	) -> Self {
		Self {
			base_: base,
			attack_time,
			decay_time,
			sustain_level,
			release_time,
			initial_level,
			amplitude: 0f32,
			state: AdsrEnvState::Idle,
		}
	}
}
#[node_impl]
impl Node for AdsrEnv {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams {
		vec![
			self.attack_time.channeled(),
			self.decay_time.channeled(),
			self.sustain_level.channeled(),
			self.release_time.channeled(),
			self.initial_level.channeled(),
		]
	}
	fn activeness(&self) -> Activeness { Activeness::Active }
	fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut [Sample], _context: &Context, _env: &mut Environment) {
		output_mono(output, self.amplitude);
	}

	fn update(&mut self, inputs: &Vec<Sample>, context: &Context, _env: &mut Environment) {
		let to_rate_per_sample = |sec| { if sec <= 0f32 { 1f32 } else { 1f32 / context.sample_rate_f32() / sec } };
		let attack = |this: &mut Self| {
			let attack_rate_per_sample = to_rate_per_sample(inputs[0]);
			this.amplitude += attack_rate_per_sample;
			if this.amplitude >= 1f32 {
				this.amplitude = 1f32;
				this.state = AdsrEnvState::Decay;
			}
		};
		match self.state {
			AdsrEnvState::Idle => {
				// nop: waiting for NoteOn
			},
			AdsrEnvState::Initial => {
				// note on のイベント処理中は初期値が取れないため、ここで設定する。
				// 直後にアタック処理に流すため、最初の出力は initial_level + attack_rate_per_sample であり、
				// initial_level そのものを出力することはない
				// （そうしないと attack_time = 0 で note on の瞬間に出力が 1 にならない）
				let initial_level = inputs[4];
				self.amplitude = initial_level;
				self.state = AdsrEnvState::Attack;
				attack(self);
			},
			AdsrEnvState::Attack => {
				attack(self);
			},
			AdsrEnvState::Decay => {
				let decay_rate_per_sample = to_rate_per_sample(inputs[1]);
				let sustain_level = inputs[2];
				self.amplitude -= decay_rate_per_sample;
				if self.amplitude <= sustain_level {
					self.amplitude = sustain_level;
					self.state = AdsrEnvState::Sustain;
				}
			},
			AdsrEnvState::Sustain => {
				// nop: waiting for NoteOff
			},
			AdsrEnvState::Release => {
				let release_rate_per_sample = to_rate_per_sample(inputs[3]);
				self.amplitude -= release_rate_per_sample;
				if self.amplitude <= 0f32 {
					self.amplitude = 0f32;
					self.state = AdsrEnvState::Idle;
				}
			},
		}
		if self.state == AdsrEnvState::Idle { return; }

	}
	fn process_event(&mut self, event: &dyn Event, _context: &Context, _env: &mut Environment) {
		if event.event_type() != EVENT_TYPE_NOTE { return; }

		let event = event.downcast_ref::<NoteEvent>().unwrap();
		if event.note_on() {
			self.state = AdsrEnvState::Initial;
		} else {
			self.state = AdsrEnvState::Release;
		}
	}

}
#[derive(Eq, PartialEq)] enum AdsrEnvState { Idle, Initial, Attack, Decay, Sustain, Release }

pub struct AdsrEnvFactory { }
impl NodeFactory for AdsrEnvFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> {
		vec![
			spec_with_default("attack", 1, 0f32),
			spec_with_default("decay", 1, 0f32),
			spec_with_default("sustain", 1, 1f32),
			spec_with_default("release", 1, 0f32),
			// 状態遷移の順序的には attack より先だが、一般的な ADSR からするとオプションなので最後に置く
			spec_with_default("initial", 1, 0f32),
		]
	}
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, base: NodeBase, node_args: &NodeArgs, _piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		Box::new(AdsrEnv::new(
			base,
			node_args.get("attack").unwrap().as_mono(),
			node_args.get("decay").unwrap().as_mono(),
			node_args.get("sustain").unwrap().as_mono(),
			node_args.get("release").unwrap().as_mono(),
			node_args.get("initial").unwrap().as_mono(),
		))
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
