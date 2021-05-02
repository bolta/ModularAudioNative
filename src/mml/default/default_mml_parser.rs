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
	one_of,
	optional,
	Parser,
	parser::{
		char::{
			spaces,
			string,
		},
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

#[derive(Debug, PartialEq)]
pub struct CompilationUnit {
	commands: Vec<Command>,
}

#[derive(Debug, PartialEq)]
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
	Tone { tone_name: ToneName, length: Option<Length>, slur: bool },
	Rest(Length),
	Parameter { name: String, value: f32 },
	Loop { times: Option<i32>, Content: Vec<Command> },
	LoopBreak,
	Stack { content: Vec<Command> },
	ExpandMacro { name: String },
}

#[derive(Debug, PartialEq)]
pub struct Length {
	elements: Vec<LengthElement>,
}

#[derive(Debug, PartialEq)]
pub struct LengthElement {
	/// 音長を示す数値。省略の場合は None。音長 4. に対して 4、.. に対して None となる
	number: Option<i32>,

	/// 付点の数
	dots: i32,
}

#[derive(Debug, PartialEq)]
pub struct ToneName {
	base_name: ToneBaseName,
	accidental: i32,
}

#[derive(Debug, PartialEq)]
pub enum ToneBaseName {
	C, D, E, F, G, A, B,
}

pub fn compilation_unit<'a, I>() -> impl Parser<I, Output = CompilationUnit> + 'a
where
	I: RangeStream<Token = char, Range = &'a str> + 'a,
	I::Error: ParseError<I::Token, I::Range, I::Position>,
{
	// 他から参照されないパーザはとりあえずここにローカルで書く
	// TODO データ定義は別ファイルで

	let skip_spaces = || spaces().silent();

	// 後続の空白を食うパーザたち
	let integer_ss = || integer().skip(skip_spaces());
	let one_of_ss = |toks| one_of(toks).skip(skip_spaces());
	let real_ss = || real().skip(skip_spaces());
	let string_ss = |tok| string(tok).skip(skip_spaces());
	let token_ss = |tok| token(tok).skip(skip_spaces());

	let integer_command = |tok, ctor : fn (i32) -> Command| string_ss(tok)
			.with(integer_ss())
			.map(move |val| ctor(val));

	let real_command = |tok, ctor: fn (f32) -> Command| string(tok).skip(skip_spaces())
			.with(real_ss())
			.map(move |val| ctor(val));

	let octave_command = integer_command("o", Command::Octave);
	let octave_incr_command = token_ss('>').map(|_| Command::OctaveIncr);
	let octave_decr_command = token_ss('<').map(|_| Command::OctaveDecr);

	let length_command = integer_command("l", Command::Length);
	let gate_rate_command = real_command("q", Command::GateRate);
	let volume_command = real_command("V", Command::Volume);
	let velocity_command = real_command("v", Command::Velocity);
	let detune_command = real_command("@d", Command::Detune);

	// 言われるままに型注釈をつけた
	// https://docs.rs/combine/4.5.2/combine/fn.many1.html
	let accidentals = many1::<Vec<_>, _, _>(token_ss('+')).map(|sharps| sharps.len() as i32)
			.or(many1::<Vec<_>, _, _>(token_ss('-')).map(|flats| - (flats.len() as i32)));
	let tone_command = one_of_ss("cdefgab".chars())
			.and(optional(accidentals))
			.map(|(b, a)| {
		let base_name = match b {
			'c' => ToneBaseName::C,
			'd' => ToneBaseName::D,
			'e' => ToneBaseName::E,
			'f' => ToneBaseName::F,
			'g' => ToneBaseName::G,
			'a' => ToneBaseName::A,
			'b' => ToneBaseName::B,
			_ => unreachable!(),
		};

		Command::Tone {
			tone_name: ToneName {
				base_name,
				accidental: a.unwrap_or(0),
			},
			length: None,//Length { elements: vec![LengthElement] }
			slur: false,
		}
	});
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
			.or(detune_command)
			.or(tone_command);

	// 最先頭の空白だけここで食う
	spaces()
			.with(many(command).map(move |commands| CompilationUnit { commands }))
}

#[test]
fn test_compilation_unit() {
	assert_eq!(
			compilation_unit().parse("o4l8v15").unwrap().0,
			CompilationUnit {
				commands: vec![
					Command::Octave(4),
					Command::Length(8),
					Command::Velocity(15.0),
				]});
}

#[test]
fn test_compilation_unit_spaces() {
	{
		let expected = CompilationUnit {
			commands: vec![Command::Octave(4)],
		};
		assert_eq!(compilation_unit().parse("o4").unwrap().0, expected);
		assert_eq!(compilation_unit().parse("  o4").unwrap().0, expected);
		assert_eq!(compilation_unit().parse("o  4").unwrap().0, expected);
		assert_eq!(compilation_unit().parse("o4  ").unwrap().0, expected);
	}
	{
		let expected = CompilationUnit {
			commands: vec![Command::GateRate(7.5)],
		};
		assert_eq!(compilation_unit().parse("q7.5").unwrap().0, expected);
		assert_eq!(compilation_unit().parse("  q7.5").unwrap().0, expected);
		assert_eq!(compilation_unit().parse("q  7.5").unwrap().0, expected);
		assert_eq!(compilation_unit().parse("q7.5  ").unwrap().0, expected);
	}
	{
		let expected = CompilationUnit {
			commands: vec![Command::Octave(4), Command::GateRate(7.5)],
		};
		assert_eq!(compilation_unit().parse("o4 q7.5").unwrap().0, expected);
	}
	{
		let expected = CompilationUnit {
			commands: vec![Command::GateRate(7.5), Command::Octave(4)],
		};
		assert_eq!(compilation_unit().parse("q7.5 o4").unwrap().0, expected);
	}
	{
		let expected = CompilationUnit {
			commands: vec![Command::OctaveIncr, Command::OctaveDecr],
		};
		assert_eq!(compilation_unit().parse("> <").unwrap().0, expected);
	}
	{
		let expected = CompilationUnit {
			commands: vec![Command::OctaveDecr, Command::OctaveIncr],
		};
		assert_eq!(compilation_unit().parse("< >").unwrap().0, expected);
	}
}

#[test]
fn test_compilation_unit_tones() {
	let expected = |base_name, accidental, length, slur| CompilationUnit {
		commands: vec![
			Command::Tone {
				tone_name: ToneName {
					base_name,
					accidental,
				},
				length,
				slur,
			},
		]
	};

	assert_eq!(compilation_unit().parse("c ").unwrap().0, expected(ToneBaseName::C, 0, None, false));
	assert_eq!(compilation_unit().parse("d+ ").unwrap().0, expected(ToneBaseName::D, 1, None, false));
	assert_eq!(compilation_unit().parse("e ++ + ").unwrap().0, expected(ToneBaseName::E, 3, None, false));
	assert_eq!(compilation_unit().parse("f -").unwrap().0, expected(ToneBaseName::F, -1, None, false));
	assert_eq!(compilation_unit().parse("g -- - ").unwrap().0, expected(ToneBaseName::G, -3, None, false));
}
