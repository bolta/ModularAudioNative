use super::ast::*;

use crate::common::*;

extern crate nom;
//use nom::regexp::str::*;
use nom::{
	branch::alt,
	bytes::complete::*,
	character::complete::*,
	combinator::*,
	IResult,
	multi::*,
	regexp::str::*,
	sequence::*,
};


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

parser![statement_ending, (), {
	map_res(
			// 空行を無視するよう ss! をかます。
			// 先頭の空行は compilation_unit で対応
			ss!(alt((line_ending, eof))),
			|_| Ok::<_, ()>(()))
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
parser![identifier_literal, Box<Expr>, {
	map_res(si!(delimited(char('`'), identifier(), char('`'))),
			|id| { ok(Box::new(Expr::IdentifierLiteral(id.to_string()))) })
}];
parser![string_literal, Box<Expr>, {
	// TODO " などをエスケープできるようにする
	map_res(si!(delimited(char('"'), re_find(re(r#"[^"]*"#)), char('"'))),
			|str| { ok(Box::new(Expr::StringLiteral(str.to_string()))) })
}];
parser![identifier_expr, Box<Expr>, {
	map_res(identifier(),
			|id| { ok(Box::new(Expr::Identifier(id.to_string()))) })
}];
parser![lambda_expr, Box<Expr>, {
	map_res(
		preceded(
			ss!(tag("node")),
			tuple((
				delimited(
					ss!(char('(')),
					ss!(identifier()),
					ss!(char(')')),
				),
				si!(expr()),
			)),
		),
		|(input_param, body)| { ok(Box::new(Expr::Lambda {
			input_param: input_param.to_string(),
			body,
		})) },
	)
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
		identifier_literal(),
		string_literal(),
		lambda_expr(), // キーワード node を処理するため identifier_expr よりも先に試す
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
					ss!(re_find(re($oper_regexp))),
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

parser![named_entry, (String, Box<Expr>), {
	map_res(
		tuple((
			terminated(
				ss!(identifier()),
				ss!(char(':')),
			),
			ss!(expr()),
		)),
		|(id, expr)| ok((id.to_string(), expr))
	)
}];

parser![assoc_array_literal, Box<Expr>, {
	map_res(
			// 連想配列の中は改行を許す（これだけだと式の中で改行できないので不完全だが…）
			preceded(
				ss!(char('{')),
				terminated(
					separated_list0(ss!(char(',')), ss!(named_entry())),
					tuple((
						opt(ss!(char(','))),
						si!(char('}')),
					))
				)
			),
			|entries| ok(Box::new(Expr::AssocArrayLiteral(entries)))
	)
}];

parser![function_args, (Vec<Box<Expr>>, AssocArray), {
	alt((
		map_res(
			ss!(separated_list1(ss!(char(',')), ss!(named_entry()))),
			|named_args| ok((vec![], named_args)),
		),
		map_res(
			tuple((
				separated_list0(
					ss!(char(',')),
					// 識別子はそれだけ見ても式（unnamed_args の一部）なのか引数名（named_args の一部）なのか
					// 区別できないので、直後に : があるかどうか（あれば引数名）で判別する
					ss!(terminated(
						expr(),
						peek(not(char(':'))),
					)),
				),
				opt(
					preceded(
						ss!(char(',')),
						separated_list1(ss!(char(',')), ss!(named_entry())),
					)
				),
			)),
			|(unnamed_args, named_args)| ok((unnamed_args, named_args.unwrap_or_else(|| vec![]))),
		),
	))
}];

parser![function_call, Box<Expr>, {
	map_res(
		tuple((
			si!(primary_expr()),
			opt(delimited(
				ss!(char('(')),
				ss!(function_args()),
				tuple((
					opt(ss!(char(','))),
					si!(char(')')),
				)),
			)),
		)),
		|(x, args)| ok(match args {
			None => x,
			Some((unnamed_args, named_args)) => {
				Box::new(Expr::FunctionCall {
					function: x,
					unnamed_args,
					named_args,
				})
			},
		}),
	)
}];

parser![node_with_args_expr, Box<Expr>, {
	map_res(
			tuple((
				si!(function_call()),
				opt(si!(assoc_array_literal())),
			)),
			|(x, assoc)| ok(match assoc {
				None => x,
				Some(assoc) => {
					let args = match *assoc {
						Expr::AssocArrayLiteral(args) => args,
						_ => unreachable!(),
					};
					Box::new(Expr::NodeWithArgs {
						node_def: x,
						label: "".to_string(), // 未使用
						args
					})
				},
			}))
}];

binary_expr![connective_expr, node_with_args_expr, r"[\|]", |lhs, _op, rhs| Expr::Connect { lhs, rhs }];
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
binary_expr![comparison_expr, add_sub_expr, r"<=|<|==|!=|>=|>", |lhs, op, rhs| match op {
	"<" => Expr::Less { lhs, rhs },
	"<=" => Expr::LessOrEqual { lhs, rhs },
	"==" => Expr::Equal { lhs, rhs },
	"!=" => Expr::NotEqual { lhs, rhs },
	">" => Expr::Greater { lhs, rhs },
	">=" => Expr::GreaterOrEqual { lhs, rhs },
	_ => unreachable!(),
}];
binary_expr![logical_expr, comparison_expr, r"&&|\|\|", |lhs, op, rhs| match op {
	"&&" => Expr::And { lhs, rhs },
	"||" => Expr::Or { lhs, rhs },
	_ => unreachable!(),
}];



parser![expr, Box<Expr>, {
	// 効果ない？
	|input| {
		let (input, result) = logical_expr()(input)?;

		Ok((input, result))
	}
}];

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
				mml: mml.to_string() + "\n", // 改行は行コメントの終端に必要
			}))
}];
parser![statement, Statement, {
	alt((
		directive_statement(),
		mml_statement(),
	))
}];

pub_parser![compilation_unit, CompilationUnit, {
	map_res(
			all_consuming(
					preceded(
						many0(space()),
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
