use super::{
	default_mml_parser::*,
};
use crate::{
	core::{
		event::*,
	},
	node::{
		var::*,
	},
	seq::{
		instruction::*,
		sequence::*,
	},
};

pub fn generate_sequences(CompilationUnit { commands }: &CompilationUnit, ticks_per_beat: i32) -> Vec<Sequence> {
	let ticks_per_bar = 4 * ticks_per_beat;

	let mut state = MmlState::init();

	let mut result = vec![];
	for command in commands {
		match command {
			Command::Octave(val) => { state.octave = *val; }
			Command::OctaveIncr => { state.octave += 1; }
			Command::OctaveDecr => { state.octave -= 1; }
			Command::Length(val) => { state.length = *val; }
			Command::GateRate(val) => { state.gate_rate = *val; }
			Command::Tone { tone_name, length, slur } => {
				let step_ticks = calc_ticks_from_length(&length, ticks_per_bar, state.length);
				let gate_ticks = (step_ticks as f32 * state.gate_rate) as i32;


				// TODO 本当は temperament を挟む
				let freq = calc_freq_from_tone(state.octave, tone_name);
				
				// TODO タグは外から与えたいかも
				result.push(Instruction::Value { tag: "freq".to_string(), value: freq });
				result.push(Instruction::Note { tag: "note".to_string(), note_on: true });
				result.push(Instruction::Wait(gate_ticks));
				result.push(Instruction::Note { tag: "note".to_string(), note_on: false });
				if step_ticks - gate_ticks > 0 {
					result.push(Instruction::Wait(step_ticks - gate_ticks));
				}
				// TODO スラー対応
			}
			Command::Rest(val) => {
				let ticks = calc_ticks_from_length(&val, ticks_per_bar, state.length);
				result.push(Instruction::Wait(ticks));
			}

			_ => { unimplemented!(); }
		}
	}

	vec![result]
}

fn calc_ticks_from_length(Length { elements: length_spec }: &Length, ticks_per_bar: i32, default: i32) -> i32 {
	if length_spec.is_empty() {
		return divide_ticks(ticks_per_bar, default);
	}

	let calc_ticks_from_length_element = |e: &LengthElement| -> i32 {
		let number = e.number.unwrap_or(default);
		let number_ticks = divide_ticks(ticks_per_bar, number);
		// n 個の付点（n >= 0）が付くと、音長は元の音長の 2 倍から元の音長の 2^(n+1) 分の 1 を引いた長さになる
		number_ticks * 2 - divide_ticks(number_ticks, 2i32.pow(e.dots as u32))
	};

	length_spec.iter().map(calc_ticks_from_length_element).sum()
}
fn divide_ticks(ticks: i32, denominator: i32) -> i32 {
	let result = ticks / denominator;
	if result * denominator != ticks {
		// TODO ちゃんとエラー処理
		panic!("テンポずれ");
	}

	result
}

fn calc_freq_from_tone(octave: i32,
		ToneName { base_name, accidental }: &ToneName) -> f32 {
	let note_a4 = 69;
	let freq_a4 = 440f32;
	// とりあえず平均律のみ…
	let note_number = 12 * (octave + 1) + match base_name {
		ToneBaseName::C => 0,
		ToneBaseName::D => 2,
		ToneBaseName::E => 4,
		ToneBaseName::F => 5,
		ToneBaseName::G => 7,
		ToneBaseName::A => 9,
		ToneBaseName::B => 11,
	} + *accidental;

	freq_a4 * 2f32.powf((note_number - note_a4) as f32 / 12f32)
}

// TODO スタックにする
struct MmlState {
	octave: i32,
	length: i32,
	/// スラーの途中（前の音符にスラーがついていた）かどうか
	slur: bool,
	gate_rate: f32,
	// detune
}
impl MmlState {
	fn init() -> Self {
		Self {
			octave: 4,
			length: 4,
			slur: false,
			gate_rate: 1f32,
		}
	}
}
