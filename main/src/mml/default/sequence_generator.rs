use crate::{
	common::stack, mml::default::feature::*, moddl::{console::warn, error::{error, ErrorType, ModdlResult}}, seq::{
		instruction::*,
		sequence::*,
	}
};
extern crate parser;
use parser::{common::Location, mml::ast::*};

use std::collections::{
	hash_map::HashMap,
	hash_set::HashSet,
};

/// (qname, key)
pub type ParamSignature = (String, String);
pub struct TagSet {
	pub freq: String,
	pub note: String,
}

// TODO 将来はディレクティブで設定できるように
const MAX_GATE_RATE: f32 = 8f32;
const MAX_VOLUME: f32 = 15f32;
const MAX_VELOCITY: f32 = 15f32;

const PARAM_NAME_VOLUME: &str = "#volume";
const PARAM_NAME_VELOCITY: &str = "#velocity";
const PARAM_NAME_DETUNE: &str = "#detune";
const PARAM_NAME_TEMPO: &str = "#tempo";

pub fn scan_features(CompilationUnit { commands }: &CompilationUnit) -> HashSet<Feature> {
	let mut result = HashSet::new();
	scan_features_(commands, &mut result);

	result
}

fn scan_features_(commands: &Vec<Command>, result: &mut HashSet<Feature>) {
	for cmd in commands {
		// Feature を使うコマンドと、内容を持つコマンドだけ処理
		match cmd {
			Command::Volume(_) => { result.insert(Feature::Volume); },
			Command::Velocity(_) => { result.insert(Feature::Velocity); },
			Command::Detune(_) => { result.insert(Feature::Detune); },

			Command::Loop { content1, content2, .. } => {
				scan_features_(content1, result);
				if let Some(content2) = content2 {
					scan_features_(content2, result);
				}
			},
			Command::Stack { content } => { scan_features_(content, result); },
			Command::MacroDef { content, .. } => { scan_features_(content, result); },
			_ => { },
		}
	}
}

pub fn generate_sequences(
	CompilationUnit { commands }: &CompilationUnit,
	ticks_per_bar: i32,
	tag_set: &TagSet,
	param_prefix: &str,
	param_initials: &HashMap<ParamSignature, f32>,
	param_default_keys: &HashMap<String, String>,
	evaluate_expr: &mut dyn FnMut (&str) -> ModdlResult<f32>,
) -> ModdlResult<HashMap<String, Sequence>> {
	let mut stack = init_stack(param_initials);
	let mut var_seq = 0;
	let mut seq_seq = 0;
	let mut sequences = HashMap::new();
	let mut used_skip = false;

	generate_sequence(SEQUENCE_NAME_MAIN, commands, ticks_per_bar, tag_set, &mut stack, &mut var_seq, &mut seq_seq, &mut sequences,
			&mut used_skip, param_prefix, param_default_keys, evaluate_expr) ?;
	if used_skip {
		sequences.get_mut(SEQUENCE_NAME_MAIN).unwrap().insert(0usize, Instruction::EnterSkipMode);
	}

	Ok(sequences)
}

fn make_name(prefix: &str, count: &mut i32) -> String {
	let name = format!("#{}{}", prefix, count);
	*count += 1;
	name
}

fn generate_sequence(
	seq_name: &str,
	commands: &Vec<Command>,
	ticks_per_bar: i32,
	tag_set: &TagSet,
	stack: &mut Stack,
	var_seq: &mut i32,
	seq_seq: &mut i32,
	sequences: &mut HashMap<String, Sequence>,
	used_skip: &mut bool,
	param_prefix: &str,
	param_default_keys: &HashMap<String, String>,
	evaluate_expr: &mut dyn FnMut (&str) -> ModdlResult<f32>,
) -> ModdlResult<()> {
	let mut seq = vec![];
	for command in commands {
		match command {
			Command::Octave(val) => { stack.mml_state_mut().octave = evaluate(val, evaluate_expr) ?; }
			Command::OctaveIncr => { stack.mml_state_mut().octave += 1f32; }
			Command::OctaveDecr => { stack.mml_state_mut().octave -= 1f32; }
			Command::Length(val) => { stack.mml_state_mut().length = *val; }
			Command::GateRate(val) => { stack.mml_state_mut().gate_rate = evaluate(val, evaluate_expr)?.max(0f32).min(MAX_GATE_RATE); }
			Command::Tone { tone_name, length, slur } => {
				let step_ticks = calc_ticks_from_length(&length, ticks_per_bar, stack.mml_state().length) ?;
				let gate_ticks = (step_ticks as f32 * stack.mml_state().gate_rate / MAX_GATE_RATE) as i32;


				// TODO 本当は temperament を挟む
				let freq = calc_freq_from_tone(stack.mml_state().octave, tone_name);
				
				// TODO ちゃんとエラー処理
				let key = param_default_keys.get(&tag_set.freq).unwrap();
				// TODO タグは intern したい
				seq.push(Instruction::Value { tag: tag_set.freq.clone(), key: key.clone(), value: freq });
				if ! stack.mml_state().slur {
					seq.push(Instruction::Note { tag: tag_set.note.clone(), note_on: true });
				}
				seq.push(Instruction::Wait(gate_ticks));
				if ! *slur {
					seq.push(Instruction::Note { tag: tag_set.note.clone(), note_on: false });
				}
				if step_ticks - gate_ticks > 0 {
					seq.push(Instruction::Wait(step_ticks - gate_ticks));
				}

				stack.mml_state_mut().slur = *slur;
			}
			Command::Rest(val) => {
				let ticks = calc_ticks_from_length(&val, ticks_per_bar, stack.mml_state().length) ?;
				seq.push(Instruction::Wait(ticks));
			}
			Command::Parameter { name, key, value } => {
				// TODO ここで track prefix をかますことで MML には書かないでいいように
				// seq.push(Instruction::Value { tag: format!("{}{}", param_prefix, &name), value: *value });
				push_param_instrc(&mut seq, stack, param_default_keys, param_prefix, &name, key, evaluate(value, evaluate_expr) ?);
			}
			Command::Volume(value) => {
				push_param_instrc(&mut seq, stack, param_default_keys, param_prefix, PARAM_NAME_VOLUME, &None, evaluate(value, evaluate_expr) ? / MAX_VOLUME);
			}
			Command::Velocity(value) => {
				push_param_instrc(&mut seq, stack, param_default_keys, param_prefix, PARAM_NAME_VELOCITY, &None, evaluate(value, evaluate_expr) ? / MAX_VELOCITY);
			}
			Command::Detune(value) => {
				push_param_instrc(&mut seq, stack, param_default_keys, param_prefix, PARAM_NAME_DETUNE, &None, evaluate(value, evaluate_expr) ?);
			}
			Command::Tempo(value) => {
				push_param_instrc(&mut seq, stack, param_default_keys, "" /* global */, PARAM_NAME_TEMPO, &None, evaluate(value, evaluate_expr) ?);
			}
			Command::MacroCall { name } => {
				push(stack);
				let seq_name = stack.macro_names().get(name);
				match seq_name {
					None => unimplemented!("macro not found"), // TODO エラーにする
					Some(seq_name) => {
						seq.push(Instruction::Call { seq_name: seq_name.clone() });
					},
				}

				pop_and_restore_params(stack, &mut seq);
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
				push(stack);
				let content1_name = make_name("seq", seq_seq);
				generate_sequence(content1_name.as_str(), content1, ticks_per_bar, tag_set, stack, var_seq, seq_seq, sequences, used_skip, param_prefix, param_default_keys, evaluate_expr) ?;
				seq.push(Instruction::Call { seq_name: content1_name });

				if let Some(content2) = content2 {
					if let Some(var_name) = &var_name {
						seq.push(Instruction::If0 {
							var: var_name.clone(),
							then: Box::new(Instruction::JumpRel { offset: 5 }),
						});
					} else {
						// TODO 無限ループに : が含まれている。エラーにする
					}

					// content1 をコンパイルした続きの状態でコンパイルする
					let content2_name = make_name("seq", seq_seq);
					generate_sequence(content2_name.as_str(), content2, ticks_per_bar, tag_set, stack, var_seq, seq_seq, sequences, used_skip, param_prefix, param_default_keys, evaluate_expr) ?;
					seq.push(Instruction::Call { seq_name: content2_name });
				}
				if let Some(var_name) = &var_name {
					seq.push(Instruction::If0 {
						var: var_name.clone(),
						then: Box::new(Instruction::JumpRel { offset: 3 }),
					});
				}
				if let Some(var_name) = &var_name {
					seq.push(Instruction::DecrVar { name: var_name.clone() });
				}
				let cur_idx = seq.len();
				seq.push(Instruction::JumpRel { offset: -((cur_idx - loop_start) as i32) });
				// TODO : で脱出したときは 5 つ前が Jump であることを assert する
				if let Some(var_name) = &var_name {
					seq.push(Instruction::DeleteVar { name: var_name.clone() });
				}
				pop_and_restore_params(stack, &mut seq);
			}
			Command::Stack { content } => {
				push(stack);
				// 別シーケンスに分ける必要はないかもだが、generate_sequence で再帰するとシーケンスが生成される
				let content_name = make_name("seq", seq_seq);
				generate_sequence(content_name.as_str(), content, ticks_per_bar, tag_set, stack, var_seq, seq_seq, sequences, used_skip, param_prefix, param_default_keys, evaluate_expr) ?;
				seq.push(Instruction::Call { seq_name: content_name });
				pop_and_restore_params(stack, &mut seq)
			}
			Command::MacroDef { name, content } => {
				push(stack);
				let seq_name = make_name("seq", seq_seq);
				generate_sequence(seq_name.as_str(), content, ticks_per_bar, tag_set, stack, var_seq, seq_seq, sequences, used_skip, param_prefix, param_default_keys, evaluate_expr) ?;
				// コンパイルするだけなので params の復元は不要
				// pop_and_restore_params(stack, param_prefix, &mut seq);
				stack.pop();
				stack.macro_names_mut().insert(name.clone(), seq_name);
			}
			Command::Skip => {
				seq.push(Instruction::ExitSkipMode);
				*used_skip = true;
			}
			Command::ExpandMacro { name: _ } => unimplemented!(),
		}
	}

	// 始点と終点が一致すると問題になるケースがあるので、空のシーケンスは作らない
	if seq.is_empty() {
		seq.push(Instruction::Nop);
	}
	sequences.insert(seq_name.to_string(), seq);

	Ok(())
}

fn push(stack: &mut Stack) {
	let mml_state = stack.mml_state().clone();
	let params = HashMap::new();
	// TODO params と同様、新規にして参照時に検索するようにしたい
	let macro_names = stack.macro_names().clone();

	stack.push(StackFrame {
		mml_state,
		params,
		macro_names,
	});
}

/// スタックのトップで設定したパラメータについて以前の値を復元する instrc 列を生成しつつ、
/// スタックを pop する
fn pop_and_restore_params(stack: &mut Stack, seq: &mut Vec<Instruction>) {
	let names_to_restore = stack.params().keys();
	let restore_instrcs: Vec<_> = names_to_restore.map(|sig @ (name, key)| {
		// 現在の（これから pop する）フレームは除き、それ以前で設定された値を探す
		let prev_value = stack.iter_frames().skip(1).find_map(|frame| frame.params.get(sig));
		if prev_value.is_none() {
			warn(format!("Could not find the previous value of {}:{} (maybe a bug)", name, key));
		}

		prev_value.map(|value| Instruction::Value { tag: name.clone(), key: key.clone(), value: *value })
	}).filter(|i| i.is_some())
			.map(|i| i.unwrap())
			.collect();

	stack.pop();

	for i in restore_instrcs { seq.push(i) }
}

fn qualified_param_name(prefix: &str, name: &str) -> String {
	format!("{}{}", prefix, name)
}

fn push_param_instrc(seq: &mut Vec<Instruction>, stack: &mut Stack, param_default_keys: &HashMap<String, String>, param_prefix: &str, name: &str, key: &Option<String>, value: f32) {
	let param_name = qualified_param_name(param_prefix, name);
	let key = key.as_ref().or_else(|| param_default_keys.get(&param_name));
	match key {
		Some(key) => {
			seq.push(Instruction::Value { tag: param_name.clone(), key: key.clone(), value });
			stack.params_mut().insert((param_name, key.clone()), value);
		},
		None => {
			warn(format!("default key for param {} not found (maybe due to wrong param name)", param_name));
		},
	}
}

fn calc_ticks_from_length(length_spec: &Length, ticks_per_bar: i32, default: i32) -> ModdlResult<i32> {
	if length_spec.is_empty() {
		return divide_ticks(ticks_per_bar, default, length_spec);
	}

	let calc_ticks_from_length_element = |e: &LengthElement| -> ModdlResult<i32> {
		let number = e.number.unwrap_or(default);
		let number_ticks = divide_ticks(ticks_per_bar, number, length_spec) ?;
		// n 個の付点（n >= 0）が付くと、音長は元の音長の 2 倍から元の音長の 2^(n+1) 分の 1 を引いた長さになる
		Ok(number_ticks * 2 - divide_ticks(number_ticks, 2i32.pow(e.dots as u32), length_spec) ?)
	};

	length_spec.iter().map(calc_ticks_from_length_element).sum()
}
fn divide_ticks(ticks: i32, denominator: i32, length_spec: &Length) -> ModdlResult<i32> {
	let result = ticks / denominator;
	if result * denominator == ticks {
		Ok(result)
	} else {
		// テンポずれ
		Err(error(ErrorType::TickUnderflow { length: length_spec.clone() }, Location::dummy()))
	}
}

fn calc_freq_from_tone(octave: f32,
		ToneName { base_name, accidental }: &ToneName) -> f32 {
	let note_a4 = 69f32;
	let freq_a4 = 440f32;
	// とりあえず平均律のみ…
	let note_number = 12f32 * (octave + 1f32) + (match base_name {
		ToneBaseName::C => 0,
		ToneBaseName::D => 2,
		ToneBaseName::E => 4,
		ToneBaseName::F => 5,
		ToneBaseName::G => 7,
		ToneBaseName::A => 9,
		ToneBaseName::B => 11,
	} + *accidental) as f32;

	freq_a4 * 2f32.powf((note_number - note_a4) as f32 / 12f32)
}

#[derive(Clone)]
struct MmlState {
	octave: f32,
	length: i32,
	/// スラーの途中（前の音符にスラーがついていた）かどうか
	slur: bool,
	gate_rate: f32,
	// detune
}
impl MmlState {
	fn init() -> Self {
		Self {
			octave: 4f32,
			length: 4,
			slur: false,
			gate_rate: MAX_GATE_RATE,
		}
	}
}

#[derive(Clone)]
struct StackFrame {
	mml_state: MmlState,
	params: HashMap<ParamSignature, f32>,
	macro_names: HashMap<String, String>,
}

type Stack = stack::Stack<StackFrame>;

fn init_stack(param_initials: &HashMap<ParamSignature, f32>) -> Stack {
	Stack::init(StackFrame {
		mml_state: MmlState::init(),
		params: param_initials.clone(),
		macro_names: HashMap::new(),
	})
}
trait StackShortcut {
	fn mml_state(&self) -> &MmlState;
	fn params(&self) -> &HashMap<ParamSignature, f32>;
	fn macro_names(&self) -> &HashMap<String, String>;
	fn mml_state_mut(&mut self) -> &mut MmlState;
	fn params_mut(&mut self) -> &mut HashMap<ParamSignature, f32>;
	fn macro_names_mut(&mut self) -> &mut HashMap<String, String>;
}
impl StackShortcut for Stack {
	fn mml_state(&self) -> &MmlState { &self.top().mml_state }
	fn params(&self) -> &HashMap<ParamSignature, f32> { &self.top().params }
	fn macro_names(&self) -> &HashMap<String, String> { &self.top().macro_names }
	fn mml_state_mut(&mut self) -> &mut MmlState { &mut self.top_mut().mml_state }
	fn params_mut(&mut self) -> &mut HashMap<ParamSignature, f32> { &mut self.top_mut().params }
	fn macro_names_mut(&mut self) -> &mut HashMap<String, String> { &mut self.top_mut().macro_names }
}

fn evaluate(number_or_expr: &NumberOrExpr, evaluate_expr: &mut dyn FnMut (&str) -> ModdlResult<f32>) -> ModdlResult<f32> {
	match number_or_expr {
		NumberOrExpr::Number(num) => Ok(*num),
		NumberOrExpr::Expr(expr) => evaluate_expr(expr.as_str()),
	}
}
