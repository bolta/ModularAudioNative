use super::ast::*;

extern crate nom;
//use nom::regexp::str::*;
use nom::{
	branch::alt,
	bytes::complete::*,
	character::complete::*,
	combinator::*,
	error::{
		ErrorKind,
		ParseError,
	},
	IResult,
	multi::*,
	Parser,
	regexp::str::*,
	sequence::*,
};
use regex::Regex;

pub fn hello_parser(i: &str) -> IResult<&str, &str> {
	/* nom::bytes::complete:: */tag("hello")(i)
}
// type Parser<T> = impl Fn (&str) -> IResult<&str, T, ParseError<&str>>;

// pub fn hello_parser<'a>() -> Parser<&'a str> {
// 	tag("hello")
// }

// trait ResultMap {
// 	fn rmap<R>(self, f: fn (&'a str) -> R) -> IResult<&'a str, R>;
// }
// impl <'a> ResultMap for IResult<&'a str, &'a str> {
// 	fn rmap<R>(self, f: fn (&'a str) -> R) -> IResult<&'a str, R> {
// 		self.map(|(rest, matched)| (rest, f(matched)))
// 	}
// }
trait ResultMap<'a> {
	fn rmap<R>(self, f: fn (&'a str) -> R) -> IResult<&'a str, R>;
}
impl <'a> ResultMap<'a> for IResult<&'a str, &'a str> {
	fn rmap<R>(self, f: fn (&'a str) -> R) -> IResult<&'a str, R> {
		self.map(|(rest, matched)| (rest, f(matched)))
	}
}

fn re(pattern: &str) -> Regex {
	// re_find() はマッチするところまで入力を読み飛ばしてしまう
	// （re_find(re(r"\d"))("abc1") → Ok(("", "1"))）
	// これは都合が悪いので、^ を補うようにする
	let head_match_pattern = format!(r"^(?:{})", pattern);

	Regex::new(head_match_pattern.as_str()).unwrap()
}


//type Parser<'a, Result> = FnMut (&'a str) -> IResult<&'a str, Result>;

macro_rules! parser {
	($name: ident, $result_type: ty, $impl: expr) => {
		fn $name<'a>() -> impl FnMut (&'a str) -> IResult<&'a str, $result_type> {
			$impl
		}
	}
}
macro_rules! pub_parser {
	($name: ident, $result_type: ty, $impl: expr) => {
		pub fn $name<'a>() -> impl FnMut (&'a str) -> IResult<&'a str, $result_type> {
			$impl
		}
	}
}

/// skips following inline spaces if any
// TODO 本当は関数の方がいいかも
macro_rules! si {
	($parser: expr) => { terminated($parser, space0) }
}
/// skips following spaces including newlines if any
// TODO 本当は関数の方がいいかも
macro_rules! ss {
	($parser: expr) => { terminated($parser, multispace0) }
}

parser![float, f32, {
	map_res(re_find(re(r"[+-]?[0-9]+(\.[0-9]+)?|[+-]?\.[0-9]+")),
			|matched| matched.parse::<f32>())
}];

#[cfg(test)]
#[test]
fn test_float() {
	assert_eq!(float()("3.14"), Ok(("", 3.14f32)));
	// TODO 他にも
}

// fn map_res_ok<I: Clone, O1, O2, E: ParseError<I>, F, G>(
//     first: F, 
//     second: G
// ) -> impl FnMut(I) -> IResult<I, O2, E> 
// where
//     F: Parser<I, O1, E>,
//     G: Fn(O1) -> O2,
// {
// 	map_res(first, |res| Ok::<O2, ()>(second(res)))
// }

fn ok<T>(value: T) -> Result<T, ()> { Ok::<_, ()>(value) }

parser![statement_ending, (), {
	map_res(
			// 空行を無視するよう ss! をかます。
			// 先頭の空行は compilation_unit で対応
			ss!(alt((line_ending, eof))),
			|_| Ok::<_, ()>(()))
}];
parser![identifier, &str, {
	re_find(re(r"[a-zA-Z0-9_][a-zA-Z0-9_]*"))
}];

parser![float_literal, Box<Expr>, {
	map_res(float(), |v| ok(Box::new(Expr::FloatLiteral(v))))
}];

parser![track_set, Vec<String>, {
	map_res(many1(re_find(re(r"[a-zA-Z0-9_]"))),
			|tracks| { ok(tracks.iter().map(|t| t.to_string()).collect()) })
}];
parser![track_set_literal, Box<Expr>, {
	map_res(preceded(si!(char('^')), track_set()),
			|tracks| { ok(Box::new(Expr::TrackSetLiteral(tracks))) })
}];
parser![identifier_expr, Box<Expr>, {
	map_res(identifier(),
			|id| { ok(Box::new(Expr::Identifier(id.to_string()))) })
}];
parser![parenthesized_expr, Box<Expr>, {
	// preceded(si!(char('(')),
	// 		terminated(expr(),
	// 		si!(char(')'))))

	// ポイントフリーで書くと型が再帰してだめらしかった。ここだけ手続きで書くといけた…
	// https://qiita.com/elipmoc101/items/2b57eebb6627c69f59ff
	|input| {
		let (input, _) = si!(char('('))(input) ?;
		let (input, result) = si!(expr())(input) ?;
		let (input, _) = si!(char(')'))(input) ?;

		Ok((input, result))
	}
}];


parser![primary_expr, Box<Expr>, {
	alt((
		float_literal(),
		track_set_literal(),
		identifier_expr(),
		parenthesized_expr(),
	))
}];

macro_rules! binary_expr {
	($name: ident, $constituent_expr: expr, $oper_regexp: expr, $make_expr: expr) => {
		parser![$name, Box<Expr>, {
			// ここもポイントフリーで書くととんでもない型が生成されるらしくコンパイルできなくなる
			// 	error: reached the type-length limit while instantiating `std::intrinsics::drop_in_place::..., nom::error::Error<&str>>}]]]
			// ))`
			// --> C:\Users\fresh_000\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib/rustlib/src/rust\src\libcore\ptr\mod.rs:184:
			// 1
			// 	|
			// 184 | / pub unsafe fn drop_in_place<T: ?Sized>(to_drop: *mut T) {
			// 185 | |     // Code here does not matter - this is replaced by the
			// 186 | |     // real drop glue by the compiler.
			// 187 | |     drop_in_place(to_drop)
			// 188 | | }
			// 	| |_^
			// 	|
			// 	= note: consider adding a `#![type_length_limit="1024860143"]` attribute to your crate

			|input| {
				let (input, head) = si!($constituent_expr())(input) ?;
				let (input, tail) = opt(many1(tuple((
					si!(re_find(re($oper_regexp))),
					si!($constituent_expr()),
				))))(input) ?;
				let result = match tail {
					None => head,
					Some(mut tail) => {
						tail.drain(..).fold(head, |l, (op, r)| Box::new($make_expr(l, op, r)))
					}
				};
				Ok((input, result))
			}
		}];
	}
}
binary_expr![connective_expr, primary_expr, r"[\|]", |lhs, _op, rhs| Expr::Connect { lhs, rhs }];
// TODO ↓これだと左結合になってしまう
binary_expr![power_expr, connective_expr, r"[\^]", |lhs, _op, rhs| Expr::Power { lhs, rhs }];
binary_expr![mul_div_mod_expr, power_expr, r"[*/%]", |lhs, op, rhs| match op {
	"*" => Expr::Multiply { lhs, rhs },
	"/" => Expr::Divide { lhs, rhs },
	"%" => Expr::Remainder { lhs, rhs },
	_ => unreachable!(),
}];
binary_expr![add_sub_expr, mul_div_mod_expr, r"[+-]", |lhs, op, rhs| match op {
	"+" => Expr::Add { lhs, rhs },
	"-" => Expr::Subtract { lhs, rhs },
	_ => unreachable!(),
}];

parser![expr, Box<Expr>, {
	add_sub_expr()
}];

// TODO いちいち Ok::<_, ()>(...) を書きたくないので吸収するユーティリティを書きたい↑
parser![directive_statement, Statement, {
	map_res(
			tuple((
				si!(char('@')),
				si!(identifier()),
				opt(separated_list0(si!(char(',')), si!(expr()))),
				statement_ending(),
			)),
			|(_, name, args, _)| ok(Statement::Directive {
				name: name.to_string(),
				args: args.unwrap_or_else(|| vec![]).drain(..).map(|x| *x).collect(),
			}))
}];
parser![mml_statement, Statement, {
	map_res(
			tuple((
				si!(track_set()),
				terminated(
					si!(re_find(re(r"[^\r\n]*"))),
					statement_ending(),
				),
			)),
			|(tracks, mml)| ok(Statement::Mml {
				tracks,
				mml: mml.to_string(),
			}))
}];
parser![statement, Statement, {
	alt((
		directive_statement(),
		mml_statement(),
	))
}];

// TODO コメントに対応
pub_parser![compilation_unit, CompilationUnit, {
	map_res(
			all_consuming(
					preceded(
						multispace0,
						many0(statement()),
					)),
			|statements| ok(CompilationUnit { statements }))
}];

// TODO ちゃんとテストする
#[cfg(test)]
#[test]
fn test_directive_statement() {
	// TODO クソ書きづらい
	// if let (_, Statement::Directive{name, args}) = directive_statement()("@tempo 120\n").unwrap() {
	// 	assert_eq!(name, "tempo".to_string());
	// } else {
	// 	assert!(false);
	// }
	assert!(directive_statement()("@tempo").is_ok());
	assert!(directive_statement()("@tempo\n").is_ok());
	assert!(directive_statement()("@tempo 120\n").is_ok());
	assert!(directive_statement()("@tempo 120,240\n").is_ok());
	assert!(directive_statement()("@ tempo\t120 , 240   \n").is_ok());
	assert!(directive_statement()("@tempo 120, (240)\n").is_ok());
	assert!(directive_statement()("@tempo 2 | 3 | 4\n").is_ok());
	assert!(directive_statement()("@tempo 2 + 3 - 4\n").is_ok());

	assert!(directive_statement()("@tempo,120\n").is_err());
	assert!(directive_statement()("@tempo 120 240\n").is_err());
}
// TODO ちゃんとテストする
#[cfg(test)]
#[test]
fn test_mml_statement() {
	assert!(mml_statement()("abc o4l8v15 cde").is_ok());
	assert!(mml_statement()("abc").is_ok());
	assert!(mml_statement()("abc cde\r\n").is_ok());
}

// TODO ちゃんとテストする
#[cfg(test)]
#[test]
fn test_compilation_unit() {
	let moddl = r"

@tempo 80

@instrument ^ab, exponentialDecayPulseWave
		@instrument ^c, nesTriangle

abc o4l8v15 cde

";
	assert!(compilation_unit()(moddl).is_ok());
	
}

// parser![statement, Statement, {
// 	alt(
// 			directive_statement(),
// 			mml_statement())
// }]

// use super::ast::*;

// use combine::{
// 	chainl1,
// 	error::ParseError,
// 	many,
// 	many1,
// 	one_of,
// 	optional,
// 	Parser,
// 	parser::{
// 		char::{
// 			crlf,
// 			newline,
// 			spaces,
// 			string,
// 		},
// 		regex::find,
// 		token::{
// 			eof,
// 		}
// 	},
// 	satisfy,
// 	sep_by,
// 	sep_by1,
// 	skip_many,
// 	skip_many1,
// 	stream::RangeStream,
// 	stream::Stream,
// 	token,
// };
// use combine_proc_macro::parser;
// use regex::Regex;

// fn re(pattern: &str) -> Regex {
// 	Regex::new(pattern).unwrap()
// }

// // TODO MML と共通化
// // pub fn integer<'a, I>() -> impl Parser<I, Output = i32> + 'a
// // where
// // 	I: RangeStream<Token = char, Range = &'a str> + 'a,
// // 	I::Error: ParseError<I::Token, I::Range, I::Position>,
// // {
// parser!(fn integer() -> i32 {
// 	let token = find(re(r"^-?[0-9]+"));
// 	token.map(|v: &str| v.parse::<i32>().unwrap())
// });

// pub fn real<'a, I>() -> impl Parser<I, Output = f32> + 'a
// where
// 	I: RangeStream<Token = char, Range = &'a str> + 'a,
// 	I::Error: ParseError<I::Token, I::Range, I::Position>,
// {
// 	let token = find(re(r"[+-]?[0-9]+(\.[0-9]+)?|[+-]?\.[0-9]+"));
// 	token.map(|v: &'a str| v.parse::<f32>().unwrap())
// }

// /// 改行を除いた空白文字。
// /// combine::parser::char::space から改変
// fn inline_space<Input>() -> impl Parser<Input, Output = char, PartialState = ()>
// where
//     Input: Stream<Token = char>,
//     Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
// {
//     let f: fn(char) -> bool = |c| c != '\r' && c != '\n' && char::is_whitespace(c);
//     satisfy(f).expected("inline whitespace")
// }
// fn inline_spaces<Input>() -> impl Parser<Input, Output = ()>
// where
//     Input: Stream<Token = char>,
//     Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
// {
//     skip_many(inline_space()).expected("inline whitespaces")
// }

// // parser!
// // pub fn compilation_unit<'a, I>() -> impl Parser<I, Output = CompilationUnit> + 'a
// // where
// // 	I: RangeStream<Token = char, Range = &'a str> + 'a,
// // 	I::Error: ParseError<I::Token, I::Range, I::Position>,
// // {
// // 	// 他から参照されないパーザはとりあえずここにローカルで書く

// // 	// TODO MML と共通化？
// // 	// TODO spaces は改行も含んでいるみたいなのでまずいかも。改行・改行以外のスペース・両方、を使い分ける必要がありそう
// // 	let skip_inline_spaces = || inline_spaces().silent();
// // 	let skip_spaces = || spaces().silent();
// // 	let newline = || newline()
// // 			.or(crlf())
// // 			.or(eof().map(|_| '\n'));

// // 	let end_of_line = || inline_spaces().silent()
// // 			.with(newline().silent())
// // 			.with(spaces().silent());

// // 	// 後続の空白（改行除く）を食うパーザたち
// // 	let find_si = |arg| find(arg).skip(skip_inline_spaces());
// // 	let integer_si = || integer().skip(skip_inline_spaces());
// // 	// let one_of_si = |toks| one_of(toks).skip(skip_inline_spaces());
// // 	let real_si = || real().skip(skip_inline_spaces());
// // 	let string_si = |tok| string(tok).skip(skip_inline_spaces());
// // 	let token_si = |tok| token(tok).skip(skip_inline_spaces());

// // 	// 後続の空白（改行含む）を食うパーザたち
// // 	let find_ss = |arg| find(arg).skip(skip_spaces());
// // 	let integer_ss = || integer().skip(skip_spaces());
// // 	// let one_of_ss = |toks| one_of(toks).skip(skip_spaces());
// // 	let real_ss = || real().skip(skip_spaces());
// // 	let string_ss = |tok| string(tok).skip(skip_spaces());
// // 	let token_ss = |tok| token(tok).skip(skip_spaces());

// // 	let track_set = || many1(find(re(r"[a-zA-Z0-9]")))
// // 			.map(|tracks: Vec<&str>| tracks.iter().map(|t| t.to_string()).collect());

// // 	let float_literal = || real_si().map(|val| Box::new(Expr::FloatLiteral(val)));
// // 	let track_set_literal = || token_si('^')
// // 			.with(track_set())
// // 			.map(|tracks: Vec<String>| Box::new(Expr::TrackSetLiteral(tracks)));
// // 	let identifier_expr = || find_si(re(r"[a-zA-Z0-9_][a-zA-Z0-9_]*"))
// // 			.map(|id: &str| Box::new(Expr::Identifier(id.to_string())));

// // 	let primary_expr = || float_literal()
// // 			// .or(track_set_literal())
// // 			.or(identifier_expr())
// // 			;

// // 	let module_param_expr = || primary_expr();

// // 	// 関数でも書けるはずだが型を書くのが無理だったので…
// // 	macro_rules! binary_expr {
// // 		($constituent_expr: expr, $oper_regexp: expr, $make_expr: expr) => {
// // 			||
// // 			chainl1($constituent_expr().skip(skip_inline_spaces()),
// // 					find_ss(re($oper_regexp)).map(|op| move |lhs, rhs| Box::new($make_expr(lhs, op, rhs))))
// // 		}
// // 	}
// // 	let connective_expr = binary_expr!(module_param_expr, r"\|",
// // 			|lhs, _op, rhs| Expr::Connect { lhs, rhs });
// // 	let power_expr = binary_expr!(connective_expr, r"\^",
// // 			|lhs, _op, rhs| Expr::Power { lhs, rhs });
// // 	let mul_div_mod_expr = binary_expr!(power_expr, r"[*/%]", |lhs, op, rhs| match op {
// // 		"*" => Expr::Multiply { lhs, rhs },
// // 		"/" => Expr::Divide { lhs, rhs },
// // 		"%" => Expr::Remainder { lhs, rhs },
// // 		_ => unreachable!(),
// // 	});
// // 	let add_sub_expr = binary_expr!(mul_div_mod_expr, r"[+-]", |lhs, op, rhs| match op {
// // 		"+" => Expr::Add { lhs, rhs },
// // 		"-" => Expr::Subtract { lhs, rhs },
// // 		_ => unreachable!(),
// // 	});

// // 	// let expr = || float_literal()
// // 	// 		.or(track_set_literal())
// // 	// 		.or(add_sub_expr())
// // 			// .or()
// // 			// .or()
// // 			// .or()
// // 			// .or()
// // 			// .or()
// // 			// ; // TODO 以下続く
// // 	let expr = || add_sub_expr();

// // 	let expr_si = || expr().skip(skip_inline_spaces());
// // 	let expr_ss = || expr().skip(skip_spaces());

// // 	let directive_statement = || token_si('@')
// // 			.with(find_si(re(r"[a-zA-Z0-9_]+")))
// // 			.and(sep_by(expr_si(), token_si(',')))
// // 			.skip(end_of_line())
// // 			.map(|(name, mut args): (&str, Vec<Box<Expr>>)| Statement::Directive {
// // 				name: name.to_string(),
// // 				args: args.drain(..).map(|a| *a).collect(),
// // 			});
// // 	let mml_statement = || track_set()
// // 			.skip(skip_many1(inline_space()))
// // 			.and(find(re(r"[^\r\n]+")))
// // 			.skip(end_of_line())
// // 			.map(|(tracks, mml): (Vec<String>, &str)| Statement::Mml {
// // 				tracks,
// // 				mml: mml.to_string(),
// // 			});

// // 	let statement =
// // 			directive_statement()
// // 			.or(mml_statement())
// // 			;

// // 	// 最先頭の空白だけここで食う
// // 	spaces()
// // 			.with(many(statement).map(move |statements| CompilationUnit { statements }))
// // }
