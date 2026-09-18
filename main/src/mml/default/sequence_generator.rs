use crate::{
	common::stack, mml::default::feature::*, moddl::{console::warn, error::{error, ErrorType, ModdlResult}}, seq::{
		instruction::*,
		sequence::*,
	}
};
extern crate parser;
use parser::{common::Location, mml::ast::*, moddl::ast::QualifiedLabel};

use std::{collections::{
	hash_map::HashMap,
	hash_set::HashSet,
}, unreachable};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ParamSignature {
	label: QualifiedLabel,
	key: String,
}
impl ParamSignature {
	pub fn new(label: QualifiedLabel, key: impl Into<String>) -> Self {
		Self { label, key: key.into() }
	}
}

pub struct TagSet {
	pub freq: QualifiedLabel, // String,
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
	param_default_keys: &HashMap<QualifiedLabel, String>,
	evaluate_expr: &mut dyn FnMut (&str) -> ModdlResult<f32>,
) -> ModdlResult<HashMap<String, Sequence>> {
	let settings = SequenceGeneratorSettings {
		ticks_per_bar,
		tag_set,
		param_prefix,
		param_default_keys,
	};
	SequenceGenerator::new(&settings, param_initials, evaluate_expr).generate_sequences(commands)
}

struct SequenceGeneratorSettings<'a> {
	ticks_per_bar: i32,
	tag_set: &'a TagSet,
	param_prefix: &'a str,
	param_default_keys: &'a HashMap<QualifiedLabel, String>,
}

struct SequenceGenerator<'a> {
	settings: &'a SequenceGeneratorSettings<'a>,
	stack: Stack,
	var_seq: i32,
	seq_seq: i32,
	sequences: HashMap<String, Sequence>,
	used_skip: bool,
	param_changes_in_macros: HashMap<String, HashSet<ParamSignature>>,
	evaluate_expr: &'a mut dyn FnMut (&str) -> ModdlResult<f32>,
}
impl <'a> SequenceGenerator<'a> {
	fn new(
		settings: &'a SequenceGeneratorSettings<'a>,
		param_initials: &HashMap<ParamSignature, f32>,
		evaluate_expr: &'a mut dyn FnMut (&str) -> ModdlResult<f32>,
	) -> Self {
		Self {
			settings,
			stack: init_stack(param_initials),
			var_seq: 0,
			seq_seq: 0,
			sequences: HashMap::new(),
			used_skip: false,
			param_changes_in_macros: HashMap::new(),
			evaluate_expr,
		}
	}

	fn generate_sequences(mut self, commands: &[Command]) -> ModdlResult<HashMap<String, Sequence>> {
		self.generate_sequence(SEQUENCE_NAME_MAIN, commands) ?;
		if self.used_skip {
			self.sequences.get_mut(SEQUENCE_NAME_MAIN).unwrap().insert(0usize, Instruction::EnterSkipMode);
		}

		Ok(self.sequences)
	}

	fn generate_sequence(&mut self, seq_name: &str, commands: &[Command]) -> ModdlResult<()> {
		let mut seq = vec![];
		for command in commands {
			match command {
				Command::Octave(val) => { self.stack.mml_state_mut().octave = self.evaluate(val) ?; }
				Command::OctaveIncr => { self.stack.mml_state_mut().octave += 1f32; }
				Command::OctaveDecr => { self.stack.mml_state_mut().octave -= 1f32; }
				Command::Length(val) => { self.stack.mml_state_mut().length = *val; }
				Command::GateRate(val) => { self.stack.mml_state_mut().gate_rate = self.evaluate(val)?.max(0f32).min(MAX_GATE_RATE); }
				Command::Tone { tone_name, length, slur } => {
					let step_ticks = calc_ticks_from_length(&length, self.settings.ticks_per_bar, self.stack.mml_state().length) ?;
					let gate_ticks = (step_ticks as f32 * self.stack.mml_state().gate_rate / MAX_GATE_RATE) as i32;


					// TODO 本当は temperament を挟む
					let freq = calc_freq_from_tone(self.stack.mml_state().octave, tone_name);
					
					// TODO ちゃんとエラー処理
					let key = self.settings.param_default_keys.get(&self.settings.tag_set.freq).unwrap();
					// TODO タグは intern したい
					seq.push(Instruction::Value { tag: self.settings.tag_set.freq.to_string(), key: key.clone(), value: freq });
					if ! self.stack.mml_state().slur {
						seq.push(Instruction::Note { tag: self.settings.tag_set.note.clone(), note_on: true });
					}
					seq.push(Instruction::Wait(gate_ticks));
					if ! *slur {
						seq.push(Instruction::Note { tag: self.settings.tag_set.note.clone(), note_on: false });
					}
					if step_ticks - gate_ticks > 0 {
						seq.push(Instruction::Wait(step_ticks - gate_ticks));
					}

					self.stack.mml_state_mut().slur = *slur;
				}
				Command::Rest(val) => {
					let ticks = calc_ticks_from_length(&val, self.settings.ticks_per_bar, self.stack.mml_state().length) ?;
					seq.push(Instruction::Wait(ticks));
				}
				Command::Parameter { name, key, value } => {
					// TODO ここで track prefix をかますことで MML には書かないでいいように
					// seq.push(Instruction::Value { tag: format!("{}{}", param_prefix, &name), value: *value });
					let value = self.evaluate(value) ?;
					self.push_param_instrc(&mut seq, &name, key, value);
				}
				Command::Volume(value) => {
					let value = self.evaluate(value) ?;
					self.push_param_instrc(&mut seq, PARAM_NAME_VOLUME, &None, value / MAX_VOLUME);
				}
				Command::Velocity(value) => {
					let value = self.evaluate(value) ?;
					self.push_param_instrc(&mut seq, PARAM_NAME_VELOCITY, &None, value / MAX_VELOCITY);
				}
				Command::Detune(value) => {
					let value = self.evaluate(value) ?;
					self.push_param_instrc(&mut seq, PARAM_NAME_DETUNE, &None, value);
				}
				Command::Tempo(value) => {
					let value = self.evaluate(value) ?;
					self.push_param_instrc_with_prefix(&mut seq, "" /* global */, PARAM_NAME_TEMPO, &None, value);
				}
				Command::MacroCall { name } => {
					// TODO 位置情報対応
					let seq_name = self.stack.macro_names().get(name).ok_or_else(|| error(
							ErrorType::MacroNotFound { name: name.clone() }, Location::dummy())) ?;

					seq.push(Instruction::Call { seq_name: seq_name.clone() });

					let names_to_restore = self.param_changes_in_macros.get(seq_name).ok_or_else(|| error(
						ErrorType::UnknownError { message: format!("cannot resolve macro information: {} ({})", name, &seq_name) },
						Location::dummy(),
					)) ?;
					let restore_instrcs = self.param_restoration_instrcs(names_to_restore.iter(), 0);

					for i in restore_instrcs { seq.push(i) }
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
						let var_name = self.make_var_name();
						seq.push(Instruction::NewVar { name: var_name.clone(), value: times - 1 });
						Some(var_name)
					} else {
						None
					};
					let loop_start = seq.len();
					self.push();
					let content1_name = self.make_seq_name();
					self.generate_sequence(content1_name.as_str(), content1) ?;
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
						let content2_name = self.make_seq_name();
						self.generate_sequence(content2_name.as_str(), content2) ?;
						seq.push(Instruction::Call { seq_name: content2_name });
					}
					if let Some(var_name) = &var_name {
						seq.push(Instruction::If0 {
							var: var_name.clone(),
							then: Box::new(Instruction::JumpRel { offset: 3 }),
						});
						seq.push(Instruction::DecrVar { name: var_name.clone() });
					}
					let cur_idx = seq.len();
					seq.push(Instruction::JumpRel { offset: -((cur_idx - loop_start) as i32) });
					// TODO : で脱出したときは 5 つ前が Jump であることを assert する
					if let Some(var_name) = &var_name {
						seq.push(Instruction::DeleteVar { name: var_name.clone() });
					}
					self.pop_and_restore_params(&mut seq);
				}
				Command::Stack { content } => {
					self.push();
					// 別シーケンスに分ける必要はないかもだが、generate_sequence で再帰するとシーケンスが生成される
					let content_name = self.make_seq_name();
					self.generate_sequence(content_name.as_str(), content) ?;
					seq.push(Instruction::Call { seq_name: content_name });
					self.pop_and_restore_params(&mut seq)
				}
				Command::MacroDef { name, content } => {
					self.push();
					let seq_name = self.make_seq_name();
					self.generate_sequence(seq_name.as_str(), content) ?;
					// コンパイルするだけなので params の復元は不要
					// pop_and_restore_params(stack, param_prefix, &mut seq);
					// その代わり、いじったレジスタを記録しておく（呼び出し後の復元に使うため）
					self.param_changes_in_macros.insert(seq_name.clone(), self.stack.params().keys().map(|p| p.clone()).collect());
					self.stack.pop();
					self.stack.macro_names_mut().insert(name.clone(), seq_name);
				}
				Command::Skip => {
					seq.push(Instruction::ExitSkipMode);
					self.used_skip = true;
				}
				Command::ExpandMacro { name: _ } => unimplemented!(),
			}
		}

		// 始点と終点が一致すると問題になるケースがあるので、空のシーケンスは作らない
		if seq.is_empty() {
			seq.push(Instruction::Nop);
		}
		self.sequences.insert(seq_name.to_string(), seq);

		Ok(())
	}

	fn push(&mut self) {
		let mml_state = self.stack.mml_state().clone();
		let params = HashMap::new();
		// TODO params と同様、新規にして参照時に検索するようにしたい
		let macro_names = self.stack.macro_names().clone();

		self.stack.push(StackFrame {
			mml_state,
			params,
			macro_names,
		});
	}

	/// スタックのトップで設定したパラメータについて以前の値を復元する instrc 列を生成しつつ、
	/// スタックを pop する
	fn pop_and_restore_params(&mut self, seq: &mut Vec<Instruction>) {
		let names_to_restore = self.stack.params().keys();
		let restore_instrcs = self.param_restoration_instrcs(names_to_restore, 1);

		self.stack.pop();

		for i in restore_instrcs { seq.push(i) }
	}

	fn param_restoration_instrcs<'b>(&self, reg_sigs: impl Iterator<Item = &'b ParamSignature>, /* stack: &Stack, */ skip_frames: usize) -> Vec<Instruction> {
		reg_sigs.map(|sig @ ParamSignature { label, key }| {
			// 直前で設定された値を探す。先頭フレームを飛ばす場合と飛ばさない場合があるため skip_frames を受け取る
			let prev_value = self.stack.iter_frames().skip(skip_frames).find_map(|frame| frame.params.get(sig));
			if prev_value.is_none() {
				warn(format!("Could not find the previous value of {}:{} (maybe a bug)", label, key));
			}

			prev_value.map(|value| Instruction::Value { tag: label.to_string(), key: key.clone(), value: *value })
		}).filter(|i| i.is_some())
				.map(|i| i.unwrap())
				.collect()
	}

	fn evaluate(&mut self, number_or_expr: &NumberOrExpr) -> ModdlResult<f32> {
		match number_or_expr {
			NumberOrExpr::Number(num) => Ok(*num),
			NumberOrExpr::Expr(expr) => (self.evaluate_expr)(expr.as_str()),
		}
	}

	fn push_param_instrc(&mut self, seq: &mut Vec<Instruction>, name: &str, key: &Option<String>, value: f32) {
		self.push_param_instrc_with_prefix(seq, self.settings.param_prefix, name, key, value);
	}
	fn push_param_instrc_with_prefix(&mut self, seq: &mut Vec<Instruction>, param_prefix: &str, name: &str, key: &Option<String>, value: f32) {
		// TODO name を最初から QLabel にする
		let param_name = qualified_param_name(param_prefix, name);
		let param_name_elems = param_name.split('.').collect::<Vec<_>>();
		if param_name_elems.len() == 0 { unreachable!() }; // TODO そもそも QLabel で渡るようになれば不要なチェック
		let param_name_qlabel = QualifiedLabel::new(
			param_name_elems[0 .. param_name_elems.len() - 1].iter().map(|q| q.to_string()).collect(),
			param_name_elems[param_name_elems.len() - 1],
		);
		let key = key.as_ref().or_else(|| self.settings.param_default_keys.get(&param_name_qlabel));
		match key {
			Some(key) => {
				seq.push(Instruction::Value { tag: param_name.clone(), key: key.clone(), value });
				self.stack.params_mut().insert(ParamSignature { label: param_name_qlabel, key: key.clone() }, value);
			},
			None => {
				warn(format!("default key for param {} not found (maybe due to wrong param name)", param_name));
			},
		}
	}

	fn make_name(prefix: &str, count: &mut i32) -> String {
		let name = format!("#{}{}", prefix, count);
		*count += 1;
		name
	}
	fn make_var_name(&mut self) -> String {
		Self::make_name("var", &mut self.var_seq)
	}
	fn make_seq_name(&mut self) -> String {
		Self::make_name("seq", &mut self.seq_seq)
	}
}

fn qualified_param_name(prefix: &str, name: &str) -> String {
	// ここで prefix は トラック名 + '.' であり、name と直結することで QLabel の絶対表記となる
	// TODO 最初から QLabel でここまで流すようにする
	format!("{}{}", prefix, name)
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
