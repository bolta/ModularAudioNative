extern crate combine;
extern crate combine_proc_macro;

// use combine::Parser;
// use combine::parser::char::char;
// use combine::parser::choice::optional;
// use combine::/* parser::item:: */value;
// // use combine::stream::Stream;
// // use combine_proc_macro::parser;
// use std::option;
// use combine::{many1, Parser, sep_by};
// // use combine::parser::char::{
// // 	letter,
// // 	space,
// // };

use combine::{
	error::ParseError,
	many,
	many1,
	Parser,
	parser::{
		char::string,
		regex::find,
	},
	stream::RangeStream,
	token,
};
use regex::Regex;

// // use combine::parser::EasyParser;
// // use combine::parser::range::{range, take_while1};
// // use combine::parser::repeat::{sep_by};
// // //use combine::parser::Parser;
// use combine::stream::{RangeStream/* , state::State */};
// use combine::error::ParseError;

fn re(pattern: &str) -> Regex {
	Regex::new(pattern).unwrap()
}

pub fn integer<'a, I>() -> impl Parser<I, Output = i32> + 'a
where
	I: RangeStream<Token = char, Range = &'a str> + 'a,
	I::Error: ParseError<I::Token, I::Range, I::Position>,
{
	let token = find(re(r"^[0-9]+"));
	token.map(|v: &'a str| v.parse::<i32>().unwrap())
}

pub fn real<'a, I>() -> impl Parser<I, Output = f32> + 'a
where
	I: RangeStream<Token = char, Range = &'a str> + 'a,
	I::Error: ParseError<I::Token, I::Range, I::Position>,
{
	let token = find(re(r"[+-]?[0-9]+(\.[0-9]+)?|[+-]?\.[0-9]+"));
	token.map(|v: &'a str| v.parse::<f32>().unwrap())
}

#[derive(Debug)]
pub struct CompilationUnit {
	commands: Vec<Command>,
}

#[derive(Debug)]
pub enum Command {
	// コマンドの名前が値の名前そのものである場合はパラメータ名を省略
	Octave(i32),
	OctaveIncr,
	OctaveDecr,
	Length(i32),
	GateRate(f32),
	Volume(f32),
	Velocity(f32),
	Detune(f32),
	Tone { tone_name: ToneName, length: Length, slur: bool },
	Rest(Length),
	Parameter { name: String, value: f32 },
	Loop { times: Option<i32>, Content: Vec<Command> },
	LoopBreak,
	Stack { content: Vec<Command> },
	ExpandMacro { name: String },
}

#[derive(Debug)]
pub struct Length {
	elements: Vec<LengthElement>,
}

#[derive(Debug)]
pub struct LengthElement {
	/// 音長を示す数値。省略の場合は None。音長 4. に対して 4、.. に対して None となる
	number: Option<i32>,

	/// 付点の数
	dots: i32,
}

#[derive(Debug)]
pub struct ToneName {
	base_name: ToneBaseName,
	accidental: i32,
}

#[derive(Debug)]
pub enum ToneBaseName {
	C, D, E, F, G, A, B,
}

pub fn compilation_unit<'a, I>() -> impl Parser<I, Output = CompilationUnit> + 'a
where
	I: RangeStream<Token = char, Range = &'a str> + 'a,
	I::Error: ParseError<I::Token, I::Range, I::Position>,
{
	// 他から参照されないパーザはとりあえずここにローカルで書く
	// TODO 空白を飛ばす
	// TODO データ定義は別ファイルで

	let integer_command = |tok, ctor : fn (i32) -> Command| string(tok)
			.and(integer())
			.map(move |(_, val)| ctor(val));

	let real_command = |tok, ctor: fn (f32) -> Command| string(tok)
			.and(real())
			.map(move |(_, val)| ctor(val));

	let octave_command = integer_command("o", Command::Octave);
	let octave_incr_command = token('>').map(|_| Command::OctaveIncr);
	let octave_decr_command = token('<').map(|_| Command::OctaveDecr);

	let length_command = integer_command("l", Command::Length);
	let gate_rate_command = real_command("q", Command::GateRate);
	let volume_command = real_command("V", Command::Volume);
	let velocity_command = real_command("v", Command::Velocity);
	let detune_command = real_command("@d", Command::Detune);
	// let tone_command = 
	// let rest_command = 
	// let parameter_command = 
	// let loop_command = 
	// let loop_break_command = 
	// let stack_command = 
	// let expand_macro_command = 
	
	let command =
			octave_command
			.or(octave_incr_command)
			.or(octave_decr_command)
			.or(length_command)
			.or(gate_rate_command)
			.or(volume_command)
			.or(velocity_command)
			.or(detune_command);

	many(command).map(|commands| CompilationUnit { commands })
}

