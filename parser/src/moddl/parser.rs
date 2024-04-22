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
	sequence::*,
};
// use nom_regex::str::re_find;

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
	map_res(loc(float()), |(v, loc)| ok(Box::new(Expr::new(ExprBody::FloatLiteral(v), loc))))
}];

parser![track_set, Vec<String>, {
	map_res(many1(re_find(re(r"[a-zA-Z0-9_]"))),
			|tracks| { ok(tracks.iter().map(|t| t.to_string()).collect()) })
}];
parser![track_set_literal, Box<Expr>, {
	map_res(preceded(si!(char('^')), loc(track_set())),
			|(tracks, loc)| { ok(Box::new(Expr::new(ExprBody::TrackSetLiteral(tracks), loc))) })
}];
parser![identifier_literal, Box<Expr>, {
	map_res(si!(delimited(char('`'), loc(identifier()), char('`'))),
			|(id, loc)| { ok(Box::new(Expr::new(ExprBody::IdentifierLiteral(id.to_string()), loc))) })
}];
parser![string_literal, Box<Expr>, {
	// TODO " などをエスケープできるようにする
	map_res(si!(delimited(char('"'), loc(re_find(re(r#"[^"]*"#))), char('"'))),
			|(str, loc)| { ok(Box::new(Expr::new(ExprBody::StringLiteral(str.to_string()), loc))) })
}];
parser![array_literal, Box<Expr>, {
	map_res(
		loc(delimited(
			ss!(char('[')),
			terminated(
				separated_list0(ss!(char(',')), ss!(expr())),
				opt(ss!(char(','))),
			),
			si!(char(']')),
		)),
		|(elems, loc)| { ok(Box::new(Expr::new(ExprBody::ArrayLiteral(elems), loc))) },
	)
}];

enum DataArrayElement {
	Value(i32),
	Sign(i32),
	Loop(Vec<(DataArrayElement, Location)>),
}


// FIXME unsigned と signed はほとんど同じコードが重複しているのでなんとかして整理したい

fn data_array_literal_unsigned<'a>(prefix: &'static str, digits: i32) -> impl FnMut (Span<'a>) -> IResult<Span<'a>, Box<Expr>, nom::error::VerboseError<Span<'a>>> {
	map_res(
		loc(delimited(
			tuple((
				tag(prefix), // x [ のようにスペースを空けるのは不可
				ss!(char('[')),
			)),
			many0(
				loc(alt((
					data_array_element_nonloop_unsigned(digits),
					data_array_element_loop_unsigned(digits),
				)))
			),
			si!(char(']')),
		)),
		|(elems, loc)| {
			ok(translate_data_array(&elems, loc, 1))
		}
	)
}
fn data_array_literal_signed<'a>(prefix: &'static str, digits: i32) -> impl FnMut (Span<'a>) -> IResult<Span<'a>, Box<Expr>, nom::error::VerboseError<Span<'a>>> {
	map_res(
		loc(delimited(
			tuple((
				tag(prefix), // x [ のようにスペースを空けるのは不可
				ss!(char('[')),
			)),
			many0(
				loc(alt((
					data_array_element_nonloop_signed(digits),
					data_array_element_loop_signed(digits),
				)))
			),
			si!(char(']')),
		)),
		|(elems, loc)| {
			ok(translate_data_array(&elems, loc, 1))
		}
	)
}
fn translate_data_array(elems: &Vec<(DataArrayElement, Location)>, outer_loc: Location, mut sign: i32) -> Box<Expr> {
	let mut result = vec![];
	// let mut sign = 1;
	for elem in elems {
		let elem_loc = &elem.1;
		match &elem.0 {
			DataArrayElement::Value(v) => {
				result.push(Box::new(Expr::new(ExprBody::FloatLiteral((sign * v) as f32), elem_loc.clone())));
			},
			DataArrayElement::Sign(s) => {
				sign = *s;
			},
			DataArrayElement::Loop(inner_elems) => {
				result.push(translate_data_array(&inner_elems, elem_loc.clone(), sign));
			},
		}
	}

	Box::new(Expr::new(ExprBody::ArrayLiteral(result), outer_loc))
}

fn data_array_element_nonloop_unsigned<'a>(digits: i32) -> impl FnMut (Span<'a>) -> IResult<Span<'a>, DataArrayElement, nom::error::VerboseError<Span<'a>>> {
	alt((
		map_res(
			ss!(re_find(re(format!(r"[0-9a-fA-F]{}{}{}", '{', digits, '}').as_str()))),
			|x| ok(DataArrayElement::Value(i32::from_str_radix(x, 16).unwrap())),
		),
		map_res(ss!(char('<')), |_| ok(DataArrayElement::Sign(-1))),
		map_res(ss!(char('>')), |_| ok(DataArrayElement::Sign(1))),
	))
}
fn data_array_element_loop_unsigned<'a>(digits: i32) -> impl FnMut (Span<'a>) -> IResult<Span<'a>, DataArrayElement, nom::error::VerboseError<Span<'a>>> {
	map_res(
		delimited(
			ss!(char('[')),
			many0(loc(ss!(data_array_element_nonloop_unsigned(digits)))),
			ss!(char(']')),
		),
		|xs| ok(DataArrayElement::Loop(xs)),
	)
}
fn data_array_element_nonloop_signed<'a>(digits: i32) -> impl FnMut (Span<'a>) -> IResult<Span<'a>, DataArrayElement, nom::error::VerboseError<Span<'a>>> {
	map_res(
		ss!(re_find(re(format!(r"[0-9a-fA-F]{}{}{}", '{', digits, '}').as_str()))),
		move |x| {
			let value = {
				let unsigned_value = i32::from_str_radix(x, 16).unwrap();
				let unsigned_max = 16i32.pow(digits as u32);
				if unsigned_value < unsigned_max / 2 { unsigned_value } else { unsigned_value - unsigned_max }
			};
			ok(DataArrayElement::Value(value))
		},
	)
}
fn data_array_element_loop_signed<'a>(digits: i32) -> impl FnMut (Span<'a>) -> IResult<Span<'a>, DataArrayElement, nom::error::VerboseError<Span<'a>>> {
	map_res(
		delimited(
			ss!(char('[')),
			many0(loc(ss!(data_array_element_nonloop_signed(digits)))),
			ss!(char(']')),
		),
		|xs| ok(DataArrayElement::Loop(xs)),
	)
}

parser![assoc_literal, Box<Expr>, {
	map_res(
		loc(delimited(
			ss!(char('{')),
			ss!(opt(assoc_entries())),
			si!(char('}')),
		)),
		|(elems, loc)| { ok(Box::new(Expr::new(ExprBody::AssocLiteral(elems.unwrap_or_else(|| Assoc::new())), loc))) },
	)
}];
parser![identifier_expr, Box<Expr>, {
	map_res(loc(identifier()),
			|(id, loc)| { ok(Box::new(Expr::new(ExprBody::Identifier(id.to_string()), loc))) })
}];
// 専用の構文は必要なかったかも…短絡評価は不要なので、関数で if(cond, then, else) でもよかったかも
// （括弧を減らせるのはメリットと思われるけど）
parser![conditional_expr, Box<Expr>, {
	map_res(
		loc(tuple((
			preceded(
				ss!(tag("if")),
				ss!(expr()),
			),
			preceded(
				ss!(tag("then")),
				ss!(expr()),
			),
			preceded(
				ss!(tag("else")),
				si!(expr()),
			),
		))),
		|((cond, then, els), loc)| ok(Box::new(Expr::new(ExprBody::Condition { cond, then, els }, loc))),
	)
}];

parser![lambda_func_expr, Box<Expr>, {
	map_res(
		loc(tuple((
			terminated(
				alt((
					delimited(
						ss!(char('(')),
						// 引数が 1 つもない関数は禁止でもいいかも（純粋関数だと無意味なので）
						separated_list0(ss!(char(',')), tuple((
							ss!(identifier()),
							opt(
								preceded(
									ss!(char('=')),
									ss!(expr()),
								)
							)
						))),
						ss!(char(')')),
					),
					map_res(si!(identifier()), |id| ok(vec![(id, None)])),
				)),
				ss!(tag("=>")),
			),
			si!(expr()),
		))),
		|((params, body), loc)| { ok(Box::new(Expr::new(ExprBody::LambdaFunction {
			params: params.into_iter().map(|(name, default)| FunctionParam {
				name: name.to_string(),
				default,
			}).collect(),
			body,
		}, loc))) },
	)
}];

parser![do_expr, Box<Expr>, {
	map_res(
		loc(tuple((
			delimited(
				ss!(tag("do")),
				ss!(identifier()),
				ss!(tag("<-")),
			),
			terminated(
				ss!(expr()),
				ss!(char(';')),
			),
			si!(expr()),
		))),
		// do <id> <- <io>; <body> は <io>->then(<id> => <body>) の糖衣構文
		// さらには then(<io>, <id> => <body>) の糖衣構文
		|((id, io, body), loc)| { ok(Box::new(Expr::new(ExprBody::FunctionCall {
			function: Box::new(Expr::new(ExprBody::Identifier("then".to_string()), loc.clone())),
			args: Args {
				unnamed: vec![
					io,
					Box::new(Expr::new(ExprBody::LambdaFunction {
						params: vec![FunctionParam { name: id.to_string(), default: None }],
						body,
					}, loc.clone()))
				],
				named: vec![],
			}
		}, loc))) }
	)
}];

parser![let_expr, Box<Expr>, {
	map_res(
		loc(tuple((
			delimited(
				ss!(tag("let")),
				si!(identifier()),
				ss!(char('=')),
			),
			terminated(
				ss!(expr()),
				ss!(char(';')),
			),
			si!(expr()),
		))),
		// <id> = <def>; <body> は (<id> => <body>)(<def>) の糖衣構文
		|((id, def, body), loc)| { ok(Box::new(Expr::new(ExprBody::FunctionCall {
			function: Box::new(Expr::new(ExprBody::LambdaFunction {
				params: vec![FunctionParam { name: id.to_string(), default: None }],
				body,
			}, loc.clone())),
			args: Args {
				unnamed: vec![def],
				named: vec![],
			}
}		, loc))) }
	)
}];

parser![lambda_node_expr, Box<Expr>, {
	map_res(
		loc(preceded(
			ss!(char('=')),
			tuple((
				terminated(
					ss!(identifier()),
					ss!(tag("=>")),
				),
				si!(expr()),
			)),
		)),
		|((input_param, body), loc)| { ok(Box::new(Expr::new(ExprBody::LambdaNode {
			input_param: input_param.to_string(),
			body,
		}, loc))) },
	)
}];
parser![negative_expr, Box<Expr>, {
	map_res(
		loc(preceded(
			ss!(char('-')),
			si!(expr()),
		)),
		|(arg, loc)| { ok(Box::new(Expr::new(ExprBody::Negate { arg }, loc)))},
	)
}];
parser![parenthesized_expr, Box<Expr>, {
	// preceded(si!(char('(')),
	// 		terminated(expr(),
	// 		si!(char(')'))))

	// ポイントフリーで書くと型が再帰してだめらしかった。ここだけ手続きで書くといけた…
	// https://qiita.com/elipmoc101/items/2b57eebb6627c69f59ff
	|input| {
		let (input, (_, loc)) = si!(loc(char('(')))(input) ?;
		let (input, inner) = si!(expr())(input) ?;
		let (input, _) = si!(char(')'))(input) ?;

		// 位置だけ開き括弧の位置に修正
		let result = Box::new(Expr::new(inner.body, loc));

		Ok((input, result))
	}
}];


parser![primary_expr, Box<Expr>, {
	alt((
		float_literal(),
		track_set_literal(),
		identifier_literal(),
		string_literal(),
		array_literal(),
		data_array_literal_unsigned("x", 1),
		data_array_literal_unsigned("xx", 2),
		data_array_literal_signed("sx", 1),
		data_array_literal_signed("sxx", 2),
		assoc_literal(),
		conditional_expr(), // キーワード if を処理するため identifier_expr よりも先に試す
		lambda_func_expr(),
		lambda_node_expr(), // キーワード node を処理するため identifier_expr よりも先に試す
		do_expr(),
		let_expr(),
		identifier_expr(),
		negative_expr(),
		parenthesized_expr(),
	))
}];

// 後置系の構文は任意の順序・回数で適用できるよう、まとめて解析する
parser![postfix_expr, Box<Expr>, {
	map_res(
		tuple((
			si!(primary_expr()),
			many0(loc(si!(postfix()))),
		)), |(lhs, postfixes)| {
			let mut result = lhs;
			for p in postfixes {
				let loc = p.1;
				result = Box::new(match p.0 {
					Postfix::Label(label) => Expr::new(ExprBody::Labeled { label, inner: result }, loc),
					Postfix::FunctionCall(args) => Expr::new(ExprBody::FunctionCall { function: result, args }, loc),
					// receiver->method(arg0, arg1, ...) は method(receiver, arg0, arg1, ...) と等価。
					// 糖衣構文として、このレイヤーで吸収してしまう
					Postfix::MethodCall { name, args } => Expr::new(ExprBody::FunctionCall {
						// TODO 位置は関数名の位置であるべきだと思われるが、-> の位置になっている
						function: Box::new(Expr::new(ExprBody::Identifier(name), loc.clone())),
						args: Args {
							unnamed: [vec![result], args.unnamed].concat(),
							named: args.named,
						},
					}, loc),
					Postfix::PropertyAccess { name } => Expr::new(ExprBody::PropertyAccess { assoc: result, name }, loc),
					Postfix::LabelFilter(specs) => Expr::new(ExprBody::LabelFilter { strukt: result, filter: specs }, loc),
					Postfix::LabelPrefix(prefix) => Expr::new(ExprBody::LabelPrefix { strukt: result, prefix }, loc),
				})
			}

			ok(result)
		}
	)
}];

enum Postfix {
	Label(QualifiedLabel),
	FunctionCall(Args),
	MethodCall { name: String, args: Args },
	PropertyAccess { name: String },
	LabelFilter(Vec<LabelFilterSpec>),
	LabelPrefix(QualifiedLabel),
}

parser![postfix, Postfix, {
	alt((
		map_res(
			preceded(
				ss!(tag("@@")),
				qualified_label(),
			),
			|prefix| ok(Postfix::LabelPrefix(prefix)),
		),
		map_res(
			preceded(
				ss!(char('@')),
				qualified_label(),
			),
			|label| ok(Postfix::Label(label)),
		),
		map_res(
			delimited(
				ss!(char('(')),
				ss!(args()),
				char(')'),
			),
			|args| ok(Postfix::FunctionCall(args)),
		),
		map_res(
			preceded(
				ss!(tag("->")),
				tuple((
					si!(identifier()),
					opt(
						delimited(
							ss!(char('(')),
							ss!(args()),
							char(')'),
						),
					),
				)),
			),
			|(name, args)| ok(Postfix::MethodCall {
				name: name.to_string(),
				args: args.unwrap_or_else(|| Args::empty()),
			}),
		),
		map_res(
			preceded(
				ss!(char('.')),
				identifier(),
			),
			|name| ok(Postfix::PropertyAccess { name: name.to_string() }),
		),
		map_res(
			delimited(
				ss!(tag("#(")),
				terminated(
					separated_list0(ss!(char(',')), ss!(label_filter_spec())),
					opt(ss!(char(','))),
				),
				char(')'),
			),
			|specs| ok(Postfix::LabelFilter(specs)),
		),
	))
}];

parser![label_filter_spec, LabelFilterSpec, {
	alt((
		map_res(
			char('*'),
			|_| ok(LabelFilterSpec::AllowAll),
		),
		map_res(
			tuple((
				ss!(qualified_label()),
				preceded(
					ss!(tag("->")),
					qualified_label(),
				),
			)),
			|(before, after)| ok(LabelFilterSpec::Rename(before, after)),
		),
		map_res(
			preceded(
				ss!(char('!')),
				qualified_label(),
			),
			|label| ok(LabelFilterSpec::Deny(label)),
		),
		map_res(
			qualified_label(),
			|label| ok(LabelFilterSpec::Allow(label)),
		),
	))
}];

parser![qualified_label, QualifiedLabel, {
	map_res(
		separated_list0(char('.'), identifier()),
		|labels| ok({
			let mut result = String::new();
			labels.iter().enumerate().for_each(|(i, label)| {
				if i > 0 { result.push('.'); }
				result.push_str(label);
			});
			QualifiedLabel(result)
		})
	)
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
				// 2 項式の位置は、演算子の位置とする
				// （先頭にすると、a + b - c + d - e のような同一優先度の演算の連鎖があったとき、
				// 全ての式の位置が a の位置になってしまい不便と思われるため）
				let (input, head) = si!($constituent_expr())(input) ?;
				let (input, tail) = opt(many1(tuple((
					loc(ss!(re_find(re($oper_regexp)))),
					si!($constituent_expr()),
				))))(input) ?;
				let result = match tail {
					None => head,
					Some(mut tail) => {
						tail.drain(..).fold(head, |l, ((op, loc), r)| Box::new(Expr::new($make_expr(l, op, r), loc)))
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

// parser![assoc_array_literal, Box<Expr>, {
// 	map_res(
// 			// 連想配列の中は改行を許す（これだけだと式の中で改行できないので不完全だが…）
// 			preceded(
// 				ss!(char('{')),
// 				terminated(
// 					separated_list0(ss!(char(',')), ss!(named_entry())),
// 					tuple((
// 						opt(ss!(char(','))),
// 						si!(char('}')),
// 					))
// 				)
// 			),
// 			|entries| ok(Box::new(Expr::new(ExprBody::AssocArrayLiteral(entries)))
// 	)
// }];

parser![args, Args, {
	terminated(
		alt((
			map_res(
				ss!(separated_list1(ss!(char(',')), ss!(named_entry()))),
				|named| ok(Args { unnamed: vec![], named }),
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
				|(unnamed, named)| ok(Args { unnamed, named: named.unwrap_or_else(|| vec![]) }),
			),
		)),
		opt(ss!(char(','))),
	)
}];

parser![assoc_entries, Assoc, {
	map_res(
		terminated(
			ss!(separated_list1(ss!(char(',')), ss!(named_entry()))),
			opt(ss!(char(','))),
		),
		|entries| ok(entries),
	)
}];

parser![node_with_args_expr, Box<Expr>, {
	map_res(
		tuple((
			loc(si!(postfix_expr())),
			opt(
				delimited(
					ss!(char('{')),
					si!(args()),
					si!(char('}')),
				)
			),
		)),
		|((x, loc), args)| ok(match args {
			None => x,
			Some(args) => {
				Box::new(Expr::new(ExprBody::NodeWithArgs {
					node_def: x,
					args,
				}, loc))
			},
		}),
	)
}];

binary_expr![connective_expr, node_with_args_expr, r"[\|]", |lhs, _op, rhs| ExprBody::Connect { lhs, rhs }];
// TODO ↓これだと左結合になってしまう
binary_expr![power_expr, connective_expr, r"[\^]", |lhs, _op, rhs| ExprBody::Power { lhs, rhs }];
binary_expr![mul_div_mod_expr, power_expr, r"[*/%]", |lhs, op, rhs| match op {
	"*" => ExprBody::Multiply { lhs, rhs },
	"/" => ExprBody::Divide { lhs, rhs },
	"%" => ExprBody::Remainder { lhs, rhs },
	_ => unreachable!(),
}];
binary_expr![add_sub_expr, mul_div_mod_expr, r"[+-]", |lhs, op, rhs| match op {
	"+" => ExprBody::Add { lhs, rhs },
	"-" => ExprBody::Subtract { lhs, rhs },
	_ => unreachable!(),
}];
binary_expr![comparison_expr, add_sub_expr, r"<=|<|==|!=|>=|>", |lhs, op, rhs| match op {
	"<" => ExprBody::Less { lhs, rhs },
	"<=" => ExprBody::LessOrEqual { lhs, rhs },
	"==" => ExprBody::Equal { lhs, rhs },
	"!=" => ExprBody::NotEqual { lhs, rhs },
	">" => ExprBody::Greater { lhs, rhs },
	">=" => ExprBody::GreaterOrEqual { lhs, rhs },
	_ => unreachable!(),
}];
binary_expr![logical_expr, comparison_expr, r"&&|\|\|", |lhs, op, rhs| match op {
	"&&" => ExprBody::And { lhs, rhs },
	"||" => ExprBody::Or { lhs, rhs },
	_ => unreachable!(),
}];



pub_parser![expr, Box<Expr>, {
	// 効果ない？
	|input| {
		let (input, result) = logical_expr()(input)?;

		Ok((input, result))
	}
}];

parser![directive_statement, Statement, {
	map_res(
			tuple((
				ss!(char('@')),
				si!(identifier()),
				opt(separated_list0(ss!(char(',')), si!(expr()))),
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
parser![statement, (Statement, Location), {
	loc(alt((
		directive_statement(),
		mml_statement(),
	)))
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

// TODO ちゃんとテストする
#[cfg(test)]
#[test]
fn test_args() {
	let moddl = r"foo: 42, bar: a";
	let result = args()(moddl);
	assert!(result.is_ok());
}
