use super::{
	ast::*,
};

use crate::{
	common::*,
};

extern crate nom;
//use nom::regexp::str::*;
use nom::{
	branch::alt,	
	bytes::complete::*,
	character::complete::*,
	combinator::*,
	IResult,
	multi::*,
	sequence::*,
};

macro_rules! nullary_command {
	($name_parser: expr, $value: expr) => {
		map_res(
			ss!($name_parser),|_| ok($value),
		)
	}
}

macro_rules! unary_command {
	($name_parser: expr, $arg_parser: expr, $ctor: expr) => {
		map_res(
			preceded(ss!($name_parser), ss!($arg_parser)),
			|value| ok($ctor(value)),
		)
	}
}

parser![accidentals, i32, {
	alt((
		map_res(many1_count(ss!(char('+'))), |sharps| ok(sharps as i32)),
		map_res(many1_count(ss!(char('-'))), |flats| ok(- (flats as i32))),
	))
}];

parser![length_element, LengthElement, {
	map_res(
		tuple((
			opt(ss!(integer())),
			many0_count(ss!(char('.'))),
		)),
		|(number, dots)| ok(LengthElement { number, dots: dots as i32 })
	)
}];
parser![length, Length, {
	map_res(
		separated_list0(ss!(char('^')), ss!(length_element())),
		ok,
	)
}];

parser![tone_command, Command, {
	map_res(
		tuple((
			ss!(re_find(re(r"[cdefgab]"))),
			ss!(opt(accidentals())),
			ss!(length()),
			ss!(opt(char('&'))),
		)),
		|(base_name, accidentals, length, slur)| ok(Command::Tone {
			tone_name: ToneName {
				base_name: match base_name {
					"c" => ToneBaseName::C,
					"d" => ToneBaseName::D,
					"e" => ToneBaseName::E,
					"f" => ToneBaseName::F,
					"g" => ToneBaseName::G,
					"a" => ToneBaseName::A,
					"b" => ToneBaseName::B,
					_ => unreachable!(),
				},
				accidental: accidentals.unwrap_or(0),
			},
			length,
			slur: slur.is_some(),
		})
	)
}];

parser![path, &str, {
	re_find(re(r"\.?[a-zA-Z0-9_][a-zA-Z0-9_]*(?:\.[a-zA-Z0-9_][a-zA-Z0-9_]*)*"))
}];
parser![parameter_command, Command, {
	map_res(
		tuple((
			preceded(ss!(char('y')), ss!(path())),
			opt(preceded(ss!(char(':')), ss!(identifier()))),
			preceded(ss!(char(',')), ss!(number_or_expr())),
		)),
		|(name, key, value)| ok(Command::Parameter {
			name: name.to_string(),
			key: key.map(|k| k.to_string()),
			value,
		}),
	)
}];

parser![loop_command, Command, {
	// 型の無限再帰を避けるため手続きで書く
	|input| {
		let (input, _) = ss!(char('['))(input) ?;
		let (input, times_in_mml) = opt(ss!(integer()))(input) ?;
		let (input, content1) = many0(command())(input) ?;
		let (input, content2) = opt(preceded(ss!(char(':')), many0(command())))(input) ?;
		// ss!(char('['))(input) ?;
		// let (input, content) = many0(command())(input) ?;
		let (input, _) = ss!(char(']'))(input) ?;

		// MML では回数省略は 2、AST では回数 None は無限ループ
		let times = match times_in_mml {
			None => Some(2),
			Some(t) => if t == 0 { None } else { Some(t) },
		};

		Ok((input, Command::Loop { times, content1, content2 }))
	}
}];

parser![stack_command, Command, {
	// 型の無限再帰を避けるため手続きで書く
	|input| {
		let (input, _) = ss!(char('{'))(input) ?;
		let (input, content) = many0(command())(input) ?;
		let (input, _) = ss!(char('}'))(input) ?;

		Ok((input, Command::Stack { content }))
	}
}];
parser![skip_command, Command, {
	map_res(
		ss!(tag("***")),
		|_| ok(Command::Skip),
	)
}];

parser![macro_def_command, Command, {
	// 型の無限再帰を避けるため手続きで書く
	|input| {
		let (input, _) = ss!(tag("@$"))(input) ?;
		let (input, name) = ss!(identifier())(input) ?;
		let (input, _) = ss!(char('['))(input) ?;
		let (input, content) = many0(command())(input) ?;
		let (input, _) = ss!(char(']'))(input) ?;

		Ok((input, Command::MacroDef { name: name.to_string(), content }))
	}
}];

parser![number_or_expr, NumberOrExpr, {
	alt((
		map_res(float(), |num| ok(NumberOrExpr::Number(num))),
		map_res(delimited(char('='), many0(none_of(";")), char(';')),
				|chars| ok(NumberOrExpr::Expr(chars.into_iter().collect())))
	))
}];

parser![command, Command, {
	alt((
		unary_command!(char('o'), number_or_expr(), Command::Octave),
		nullary_command!(char('>'), Command::OctaveIncr),
		nullary_command!(char('<'), Command::OctaveDecr),
		unary_command!(alt((char('l'), char('L'))), integer(), Command::Length),
		unary_command!(char('q'), number_or_expr(), Command::GateRate),
		unary_command!(char('V'), number_or_expr(), Command::Volume),
		unary_command!(char('v'), number_or_expr(), Command::Velocity),
		unary_command!(re_find(re(r"@d")), number_or_expr(), Command::Detune),
		tone_command(),
		unary_command!(char('r'), length(), Command::Rest),
		parameter_command(),
		unary_command!(char('t'), number_or_expr(), Command::Tempo),
		unary_command!(char('$'), identifier(), |name: &str| Command::MacroCall { name: name.to_string() }),
		loop_command(),
		stack_command(),
		macro_def_command(),
		skip_command(),
	))
}];

pub_parser![compilation_unit, CompilationUnit, {
	map_res(
			all_consuming(
					preceded(
						many0(space()),
						many0(command()),
					)),
			|commands| ok(CompilationUnit { commands }))
}];

   ////
  ////
 //// TESTS
////

#[test]
fn test_compilation_unit() {
	assert_eq!(
			compilation_unit()("o4l8v15").unwrap().1,
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
		assert_eq!(compilation_unit()("o4").unwrap().1, expected);
		assert_eq!(compilation_unit()("  o4").unwrap().1, expected);
		assert_eq!(compilation_unit()("o  4").unwrap().1, expected);
		assert_eq!(compilation_unit()("o4  ").unwrap().1, expected);
	}
	{
		let expected = CompilationUnit {
			commands: vec![Command::GateRate(7.5)],
		};
		assert_eq!(compilation_unit()("q7.5").unwrap().1, expected);
		assert_eq!(compilation_unit()("  q7.5").unwrap().1, expected);
		assert_eq!(compilation_unit()("q  7.5").unwrap().1, expected);
		assert_eq!(compilation_unit()("q7.5  ").unwrap().1, expected);
	}
	{
		let expected = CompilationUnit {
			commands: vec![Command::Octave(4), Command::GateRate(7.5)],
		};
		assert_eq!(compilation_unit()("o4 q7.5").unwrap().1, expected);
	}
	{
		let expected = CompilationUnit {
			commands: vec![Command::GateRate(7.5), Command::Octave(4)],
		};
		assert_eq!(compilation_unit()("q7.5 o4").unwrap().1, expected);
	}
	{
		let expected = CompilationUnit {
			commands: vec![Command::OctaveIncr, Command::OctaveDecr],
		};
		assert_eq!(compilation_unit()("> <").unwrap().1, expected);
	}
	{
		let expected = CompilationUnit {
			commands: vec![Command::OctaveDecr, Command::OctaveIncr],
		};
		assert_eq!(compilation_unit()("< >").unwrap().1, expected);
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
	let length_default = || Length { elements: vec![LengthElement { number: None, dots: 0 }] };

	assert_eq!(compilation_unit()("c ").unwrap().1, expected(ToneBaseName::C, 0, length_default(), false));
	assert_eq!(compilation_unit()("d+ ").unwrap().1, expected(ToneBaseName::D, 1, length_default(), false));
	assert_eq!(compilation_unit()("e ++ + ").unwrap().1, expected(ToneBaseName::E, 3, length_default(), false));
	assert_eq!(compilation_unit()("f -").unwrap().1, expected(ToneBaseName::F, -1, length_default(), false));
	assert_eq!(compilation_unit()("g -- - ").unwrap().1, expected(ToneBaseName::G, -3, length_default(), false));

	assert_eq!(compilation_unit()("a8 ").unwrap().1, expected(ToneBaseName::A, 0,
			Length {
				elements: vec![
					LengthElement { number: Some(8), dots: 0 },
				]
			}, false));
	assert_eq!(compilation_unit()("b-^4. ^ 2...^-32& ").unwrap().1, expected(ToneBaseName::B, -1,
				Length {
					elements: vec![
						LengthElement { number: None, dots: 0 },
						LengthElement { number: Some(4), dots: 1 },
						LengthElement { number: Some(2), dots: 3 },
						LengthElement { number: Some(-32), dots: 0 },
					]
				}, true));
}

#[test]
fn test_compilation_unit_rest() {
	let expected = |command| CompilationUnit { commands: vec![command] };
	let length_default = || Length { elements: vec![LengthElement { number: None, dots: 0 }] };

	assert_eq!(compilation_unit()("r").unwrap().1, expected(Command::Rest(length_default())));
	assert_eq!(compilation_unit()("r 4^-96 . ").unwrap().1, expected(Command::Rest(
		Length {
			elements: vec![
				LengthElement { number: Some(4), dots: 0 },
				LengthElement { number: Some(-96), dots: 1 },
			],
		}
	)));
}
