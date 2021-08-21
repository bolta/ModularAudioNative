use super::{
	ast::*,
};
use crate::{
	common::{
		stack,
	},
	seq::{
		common::*,
		instruction::*,
		sequence::*,
	},
};
use std::{
	collections::hash_map::HashMap,
};

pub struct TagSet {
	pub freq: String,
	pub note: String,
}

// TODO 将来はディレクティブで設定できるように
const MAX_GATE_RATE: f32 = 8f32;

pub fn generate_sequences(CompilationUnit { commands }: &CompilationUnit, ticks_per_beat: i32, tag_set: &TagSet) -> HashMap<String, Sequence>/* Vec<Sequence> */ {
	let ticks_per_bar = 4 * ticks_per_beat;
	// let mut state = MmlState::init();
	let mut stack = init_stack();
	let mut var_seq = 0;
	let mut seq_seq = 0;
	let mut result = HashMap::<String, Sequence>::new();

	generate_sequence(SEQUENCE_NAME_MAIN, commands, ticks_per_bar, tag_set, &mut stack, &mut var_seq, &mut seq_seq, &mut result);

	return result;
}

fn make_name(prefix: &str, count: &mut i32) -> String {
	let name = format!("#{}{}", prefix, count);
	*count += 1;
	name
}

fn generate_sequence(seq_name: &str, commands: &Vec<Command>, ticks_per_bar: i32, tag_set: &TagSet, stack: &mut Stack, var_seq: &mut i32, seq_seq: &mut i32, result: &mut HashMap<String, Sequence>) {
	// let mut make_var_name = || make_name(var_seq);
	// let mut make_seq_name = || make_name(seq_seq);

	let mut seq = vec![];
	for command in commands {
		match command {
			Command::Octave(val) => { stack.mml_state().octave = *val; }
			Command::OctaveIncr => { stack.mml_state().octave += 1; }
			Command::OctaveDecr => { stack.mml_state().octave -= 1; }
			Command::Length(val) => { stack.mml_state().length = *val; }
			Command::GateRate(val) => { stack.mml_state().gate_rate = val.max(0f32).min(MAX_GATE_RATE); }
			Command::Tone { tone_name, length, slur } => {
				let step_ticks = calc_ticks_from_length(&length, ticks_per_bar, stack.mml_state().length);
				let gate_ticks = (step_ticks as f32 * stack.mml_state().gate_rate / MAX_GATE_RATE) as i32;


				// TODO 本当は temperament を挟む
				let freq = calc_freq_from_tone(stack.mml_state().octave, tone_name);
				
				// TODO タグは intern したい
				seq.push(Instruction::Value { tag: tag_set.freq.clone(), value: freq });
				seq.push(Instruction::Note { tag: tag_set.note.clone(), note_on: true });
				seq.push(Instruction::Wait(gate_ticks));
				seq.push(Instruction::Note { tag: tag_set.note.clone(), note_on: false });
				if step_ticks - gate_ticks > 0 {
					seq.push(Instruction::Wait(step_ticks - gate_ticks));
				}
				// TODO スラー対応
			}
			Command::Rest(val) => {
				let ticks = calc_ticks_from_length(&val, ticks_per_bar, stack.mml_state().length);
				seq.push(Instruction::Wait(ticks));
			}
			Command::Loop { times, content1, content2 } => {
				/*
				content1, content2 をそれぞれ別個の sequence としてコンパイルする。
				sequence には連番を含んだ一意な名前を振る（#seq0, #seq1 とする）
				また一意な名前のループカウンタ（#var0 とする）を作り、n - 1 を初期値にする
					#var0 = n - 1
				loop_start:
					call #seq0
				i		if #var0 == 0 goto loop_end
				i+1		call #seq2
				i+2		if #var0 == 0 goto loop_end
				i+3		dec #var0
				i+4		goto loop_start
					loop_end:
				i+5		delete #var0
				*/
				let var_name = if let Some(times) = times {
					assert!(*times > 0);
					let var_name = make_name("var", var_seq);
					seq.push(Instruction::NewVar { name: var_name.clone(), value: times - 1 });
					Some(var_name)
				} else {
					None
				};
				let loop_start = seq.len();
				stack.push_clone();
				let content1_name = make_name("seq", seq_seq);
				generate_sequence(content1_name.as_str(), content1, ticks_per_bar, tag_set, stack, var_seq, seq_seq, result);
				seq.push(Instruction::Call { seq_name: content1_name });

				if let Some(content2) = content2 {
					if let Some(var_name) = &var_name {
						let cur_idx = seq.len();
						seq.push(Instruction::If0 {
							var: var_name.clone(),
							then: Box::new(Instruction::Jump { seq_name: None, pos: InstructionIndex(cur_idx + 5) }),
						});
					} else {
						// TODO 無限ループに : が含まれている。エラーにする
					}

					// context1 をコンパイルした続きの状態でコンパイルする
					let content2_name = make_name("seq", seq_seq);
					generate_sequence(content2_name.as_str(), content2, ticks_per_bar, tag_set, stack, var_seq, seq_seq, result);
					seq.push(Instruction::Call { seq_name: content2_name });
				}
				if let Some(var_name) = &var_name {
					let cur_idx = seq.len();
					seq.push(Instruction::If0 {
						var: var_name.clone(),
						then: Box::new(Instruction::Jump { seq_name: None, pos: InstructionIndex(cur_idx + 3) }),
					});
				}
				if let Some(var_name) = &var_name {
					seq.push(Instruction::DecrVar { name: var_name.clone() });
				}
				seq.push(Instruction::Jump { seq_name: None, pos: InstructionIndex(loop_start) });
				// TODO : で脱出したときは 5 つ前が Jump であることを assert する
				if let Some(var_name) = &var_name {
					seq.push(Instruction::DeleteVar { name: var_name.clone() });
				}
				stack.pop();
			}
			_ => { unimplemented!(); }
		}
	}

println!("{}: {:?}", seq_name.to_string(), &seq);
	result.insert(seq_name.to_string(), seq);
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

#[derive(Clone)]
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
			gate_rate: MAX_GATE_RATE,
		}
	}
}

#[derive(Clone)]
struct StackFrame {
	mml_state: MmlState,
	// TODO parameters
}

type Stack = stack::Stack<StackFrame>;

fn init_stack() -> Stack {
	Stack::init(StackFrame { mml_state: MmlState::init() })
}
trait StackShortcut {
	fn mml_state(&mut self) -> &mut MmlState;
}
impl StackShortcut for Stack {
	fn mml_state(&mut self) -> &mut MmlState { &mut self.top_mut().mml_state }
}
