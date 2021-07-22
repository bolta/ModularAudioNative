use super::ast::*;

use combine::{
	error::ParseError,
	many,
	many1,
	one_of,
	optional,
	Parser,
	parser::{
		char::{
			crlf,
			newline,
			spaces,
			string,
		},
		regex::find,
	},
	satisfy,
	sep_by,
	sep_by1,
	skip_many,
	stream::RangeStream,
	stream::Stream,
	token,
};
use regex::Regex;

fn re(pattern: &str) -> Regex {
	Regex::new(pattern).unwrap()
}

// TODO MML と共通化
pub fn integer<'a, I>() -> impl Parser<I, Output = i32> + 'a
where
	I: RangeStream<Token = char, Range = &'a str> + 'a,
	I::Error: ParseError<I::Token, I::Range, I::Position>,
{
	let token = find(re(r"^-?[0-9]+"));
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

/// 改行を除いた空白文字。
/// combine::parser::char::space から改変
fn inline_space<Input>() -> impl Parser<Input, Output = char, PartialState = ()>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let f: fn(char) -> bool = |c| c != '\r' && c != '\n' && char::is_whitespace(c);
    satisfy(f).expected("inline whitespace")
}
fn inline_spaces<Input>() -> impl Parser<Input, Output = ()>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    skip_many(inline_space()).expected("inline whitespaces")
}

// pub fn binary_expr<F, Input, P>(
// 		constituent_expr: fn() -> impl Parser<Input, Output = Box<Expr>>,
// 		oper_regexp: &str,
// 		select_node_type: fn(&str) -> fn(Box<Expr>, Box<Expr>) -> Expr)
// 		-> impl Parser<Input, Output = Box<Expr>>
// where
//     Input: Stream,
//     P: Parser<Input, Output = Box<Expr>>,
//     F: Extend<P::Output> + Default,
// {
// //	fn binary_expr (constituent_expr: fn() -> dyn Parser<Stream, Output = Box<Expr>, PartialState = ()>, oper_regexp, select_node_type|
// 	constituent_expr().skip(skip_spaces())
// 	.and(many(
// 		find_ss(re(oper_regexp))
// 		.and(constituent_expr().skip(skip_spaces()))
// 	))
// 	.map(|(lhs, mut rhss): (Box<Expr>, Vec<(&str, Box<Expr>)>)| {
// 		rhss.drain(..).fold(lhs, |accum, (op, rhs)| {
// 			Box::new(select_node_type(op)(accum, rhs))
// 		})
// 	})
// }

pub fn compilation_unit<'a, I>() -> impl Parser<I, Output = CompilationUnit> + 'a
where
	I: RangeStream<Token = char, Range = &'a str> + 'a,
	I::Error: ParseError<I::Token, I::Range, I::Position>,
{
	// 他から参照されないパーザはとりあえずここにローカルで書く

	// TODO MML と共通化？
	// TODO spaces は改行も含んでいるみたいなのでまずいかも。改行・改行以外のスペース・両方、を使い分ける必要がありそう
	let skip_inline_spaces = || inline_spaces().silent();
	let skip_spaces = || spaces().silent();
	let newline = || newline().or(crlf());

	let end_of_line = || inline_spaces().silent()
			.with(newline().silent())
			.with(spaces().silent());

	// 後続の空白（改行除く）を食うパーザたち
	let find_si = |arg| find(arg).skip(skip_inline_spaces());
	let integer_si = || integer().skip(skip_inline_spaces());
	// let one_of_si = |toks| one_of(toks).skip(skip_inline_spaces());
	let real_si = || real().skip(skip_inline_spaces());
	let string_si = |tok| string(tok).skip(skip_inline_spaces());
	let token_si = |tok| token(tok).skip(skip_inline_spaces());

	// 後続の空白（改行含む）を食うパーザたち
	let find_ss = |arg| find(arg).skip(skip_spaces());
	let integer_ss = || integer().skip(skip_spaces());
	// let one_of_ss = |toks| one_of(toks).skip(skip_spaces());
	let real_ss = || real().skip(skip_spaces());
	let string_ss = |tok| string(tok).skip(skip_spaces());
	let token_ss = |tok| token(tok).skip(skip_spaces());

	let track_set = || many(find(re(r"[a-zA-Z0-9]")))
			.map(|tracks: Vec<&str>| tracks.iter().map(|t| t.to_string()).collect());

	// let float_literal = || find(re(r"[+-]?[0-9]+(\.[0-9]+)?|[+-]?\.[0-9]+"))
	// 		.map(Expr::FloatLiteral);
	let float_literal = || real_ss().map(|val| Box::new(Expr::FloatLiteral(val)));
	let track_set_literal = || token_ss('^')
			.with(track_set())
			.map(|tracks: Vec<String>| Box::new(Expr::TrackSetLiteral(tracks)));

	macro_rules! binary_expr {
		($constituent_expr: expr, $oper_regexp: expr, $select_node_type: expr) => {
			||
			$constituent_expr().skip(skip_spaces())
			.and(many(
				find_ss(re($oper_regexp))
				.and($constituent_expr().skip(skip_spaces()))
			))
			.map(|(lhs, mut rhss): (Box<Expr>, Vec<(&str, Box<Expr>)>)| {
				rhss.drain(..).fold(lhs, |accum, (op, rhs)| {
					Box::new($select_node_type(op)(accum, rhs))
				})
			})
		}
	}
			//	let mul_div_mod_expr = || float_literal();
	let mul_div_mod_expr = binary_expr!(float_literal, r"[*/%]", |op| match op {
		"*" => |accum, rhs| Expr::Multiply { lhs: accum, rhs },
		"/" => |accum, rhs| Expr::Divide { lhs: accum, rhs },
		"%" => |accum, rhs| Expr::Remainder { lhs: accum, rhs },
		_ => unreachable!(),
	});

	let add_sub_expr = ||
			mul_div_mod_expr().skip(skip_spaces())
			.and(many(
				find_ss(re(r"[+-]"))
				.and(mul_div_mod_expr().skip(skip_spaces()))
			))
			.map(|(lhs, mut rhss): (Box<Expr>, Vec<(&str, Box<Expr>)>)| {
				rhss.drain(..).fold(lhs, |accum, (op, rhs)| {
					Box::new(match op {
						"+" => Expr::Add { lhs: accum, rhs },
						"-" => Expr::Subtract { lhs: accum, rhs },
						_ => unreachable!()
					})
				})
			});

	// let binary_expr = |constituent_expr: fn() -> dyn Parser<Stream, Output = Box<Expr>, PartialState = ()>, oper_regexp, select_node_type|
	// 		constituent_expr().skip(skip_spaces())
	// 		.and(many(
	// 			find_ss(re(oper_regexp))
	// 			.and(constituent_expr().skip(skip_spaces()))
	// 		))
	// 		.map(|(lhs, mut rhss): (Box<Expr>, Vec<(&str, Box<Expr>)>)| {
	// 			rhss.drain(..).fold(lhs, |accum, (op, rhs)| {
	// 				Box::new(select_node_type(accum, rhs))
	// 			})
	// 		});
	// let mul_div_mod_expr = binary_expr(float_literal, r"[*/%]", |op| match op {
	// 	"*" => |accum, rhs| Expr::Multiply { lhs: accum, rhs },
	// 	"/" => |accum, rhs| Expr::Divide { lhs: accum, rhs },
	// 	"%" => |accum, rhs| Expr::Remainder { lhs: accum, rhs },
	// 	_ => unreachable!(),
	// });

	let expr = || float_literal()
			.or(track_set_literal())
			// .or()
			// .or()
			// .or()
			// .or()
			// .or()
			// .or()
			; // TODO 以下続く

	let expr_si = || expr().skip(skip_inline_spaces());
	let expr_ss = || expr().skip(skip_spaces());

	let directive_statement = string_ss("@")
			.with(find_si(re(r"[a-zA-Z0-9_]+")))
			.and(sep_by(expr_ss(), token_ss(',')))
			.skip(end_of_line())
			.map(|(name, mut args): (&str, Vec<Box<Expr>>)| Statement::Directive {
				name: name.to_string(),
				args: args.drain(..).map(|a| *a).collect(),
			});
	let mml_statement = track_set()
			.skip(skip_inline_spaces())
			.and(find(re(r"[^\r\n]*")))
			.skip(end_of_line())
			.map(|(tracks, mml): (Vec<String>, &str)| Statement::Mml {
				tracks,
				mml: mml.to_string(),
			});

	let statement =
			directive_statement
			.or(mml_statement)
			;

	// 最先頭の空白だけここで食う
	spaces()
			.with(many(statement).map(move |statements| CompilationUnit { statements }))
}
