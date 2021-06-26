use super::default_mml_parser::*;

use crate::{
	musical::{
		temperament::*,
		tone::*,
	},
	node::{
		var::*,
	},
	sequencer::{
		instruction::*,
		notable::*,
		sequence::*,
		sequencer::MAIN_SEQUENCE_KEY,
		sequence_generator::*,
	},
};

// use std::cell::RefCell;
use std::collections::HashMap;
// use std::rc::Rc;

pub struct DefaultSequenceGenerator<'a> {
	ast: &'a CompilationUnit,
	freq_users: Vec<&'a mut VarController>,//Vec<Rc<RefCell<VarController>>>,
	note_users: Vec<&'a mut dyn Notable>// Vec<Rc<RefCell<dyn Notable>>>,
}

impl<'a> DefaultSequenceGenerator<'a> {
	pub fn new(ast: &'a CompilationUnit, freq_users: Vec<&'a mut VarController>, note_users: Vec<&'a mut dyn Notable>) -> Self {
		Self { ast, freq_users, note_users }
	}
}

const PARAM_TRACK_VOLUME: &str = "#volume";
const PARAM_TRACK_VELOCITY: &str = "#velocity";
const MAX_VOLUME: f32 = 15f32;
const MAX_VELOCITY: f32 = 15f32;
const MAX_GATE_RATE:f32 = 8f32;

impl<'a> SequenceGenerator for DefaultSequenceGenerator<'a> {
	fn generate_sequences(&self, ticks_per_beat: i32/*, temper: Temperament*/) -> HashMap<String, Sequence> {
		let ticks_per_bar = 4 * ticks_per_beat;
		let CompilationUnit { commands } = self.ast;
		let temper = EqualTemperament::new();

		let mut context = Context::new();
		let mut instrcs = vec![];

		for cmd in commands {
			match cmd {
				Command::Octave(value) => context.set_octave(*value),
				Command::OctaveIncr => context.set_octave(context.octave() + 1),
				Command::OctaveDecr => context.set_octave(context.octave() - 1),
				Command::Length(value) => context.set_length(*value),
				Command::GateRate(value) => context.set_gate_rate(*value / MAX_GATE_RATE),
				// Volume
				// Velocity
				// Detune
				Command::Tone { tone_name: ToneName { base_name, accidental }, length, slur } => {
					let step_ticks = calc_ticks_from_length(*length, ticks_per_bar, context.length());
					let gate_ticks = (step_ticks as f32 * context.gate_rate()) as i32;
					// TODO parser と musical で重複しているので何とかしたい
					let base_name = match base_name {
						ToneBaseName::C => BaseName::C,
						ToneBaseName::D => BaseName::D,
						ToneBaseName::E => BaseName::E,
						ToneBaseName::F => BaseName::F,
						ToneBaseName::G => BaseName::G,
						ToneBaseName::A => BaseName::A,
						ToneBaseName::B => BaseName::B,
					};
					let tone = Tone { octave: context.octave(), base_name, accidental: *accidental };
					// TODO Detune
					let freq = temper.freq(tone);
					
					instrcs.splice(instrcs.len() .. , self.freq_users.iter().map(|u| Instruction::Value {
						target: *u,
						value: freq,
					}));
				}
				_ => todo!(),
			}
		}

		let mut result = HashMap::new();
		result.insert(MAIN_SEQUENCE_KEY.to_string(), instrcs);

		result
	}
}

#[derive(Clone)]
struct MmlState {
	octave: i32,
	length: i32,
	slur: bool,
	gate_rate: f32,
	// detune: Detune,
}
impl MmlState {
	fn new() -> Self {
		Self {
			octave: 4,
			length: 4,
			slur: false,
			gate_rate: 1f32,
		}
	}
}

struct Context {
	// TODO スタックにする
	mml_state: MmlState,

	// TODO Parameter にも対応
}
impl Context {
	fn new() -> Self {
		Self { mml_state: MmlState::new() }
	}

	fn octave(&self) -> i32 { self.mml_state.octave }
	fn set_octave(&self, value: i32) { self.mml_state.octave = value; }

	fn length(&self) -> i32 { self.mml_state.length }
	fn set_length(&self, value: i32) { self.mml_state.length = value; }

	fn gate_rate(&self) -> f32 { self.mml_state.gate_rate }
	fn set_gate_rate(&self, value: f32) { self.mml_state.gate_rate = value; }
}

fn calc_ticks_from_length(length: Length, ticks_per_bar: i32, default_length: i32) -> i32 {
	let Length { elements } = length;
	elements.iter().map(|e| calc_ticks_from_length_element(*e, ticks_per_bar, default_length)).sum()
}
fn calc_ticks_from_length_element(element: LengthElement, ticks_per_bar: i32, default_length: i32) -> i32 {
	let LengthElement { number: n, dots } = element;
	let number = n.unwrap_or(default_length);
	let number_tick = divide_tick(ticks_per_bar, number);

	// n 個の付点（n >= 0）が付くと、音長は元の音長の 2 倍から元の音長の 2^(n+1) 分の 1 を引いた長さになる
	number_tick * 2 - divide_tick(number_tick, 2i32.pow(dots as u32))
}

fn divide_tick(tick: i32, denominator: i32) -> i32 {
	let result = tick / denominator;
	// TODO Option を返すようにする
	if result * denominator != tick { panic!("テンポずれ") }

	result
}
