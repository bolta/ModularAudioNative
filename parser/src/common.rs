extern crate nom;
extern crate nom_locate;
use std::{fmt::Display, path::PathBuf, rc::Rc};

//use nom::regexp::str::*;
use nom::{
	branch::alt,
	bytes::complete::*,
	character::complete::*,
	combinator::*,
	error::{
		ParseError,
		VerboseError,
	},
	IResult,
	multi::*,
	Slice, AsBytes,
};
use nom_locate::{
	LocatedSpan,
	position,
};
use regex::Regex;

pub fn ok<T>(value: T) -> Result<T, ()> { Ok::<_, ()>(value) }

pub fn re(pattern: &str) -> Regex {
	// re_find() はマッチするところまで入力を読み飛ばしてしまう
	// （re_find(re(r"\d"))("abc1") → Ok(("", "1"))）
	// これは都合が悪いので、^ を補うようにする
	let head_match_pattern = format!(r"^(?:{})", pattern);

	Regex::new(head_match_pattern.as_str()).unwrap()
}

// use nom::error::VerboseError;

/// re_find の入力の型がなぜか &str に固定されており、LocatedSpan に対応できないので
/// 自前でラッパーを作った
pub fn re_find<'a>(regex: Regex) -> impl FnMut (Span<'a>) -> IResult<Span<'a>, &'a str, nom::error::VerboseError<Span<'a>>> {
	move |input| {
		let input_raw = *input.fragment();

		// FIXME エラー処理、とりあえず型だけ合わせたがこんなんでいいのか？
		let (_, result) = nom_regex::str::re_find::<'a, VerboseError<&str>>(regex.clone())(input_raw)
		.or_else(|_| Err(nom::Err::Error(VerboseError { errors: vec![] }))) ?;
		
		Ok((input.slice(result.len() ..), result))
	}
}

pub type Span<'a> = LocatedSpan<&'a str, Rc<PathBuf>>;
#[derive(Clone, Debug)]
pub struct Located<T> {
	pub body: T,
	pub loc: Location, // Option にするかも
}
impl <T> Located<T> {
	pub fn new(body: T, loc: Location) -> Self {
		Self { body, loc }
	}
}

/// LocatedSpan からエラーメッセージの表示に過不足のない情報だけ抽出したもの
/// （取り回しのためソースの寿命に依存しない形で）
#[derive(Clone, Debug)]
pub struct Location {
	pub path: Rc<PathBuf>,
	/// 行番号（1 始まり）
	pub line: u32,

	/// 列番号（1 始まり）
	pub column: usize,

	// offset: usize, // 必要なら追加
}
impl Location {
	pub fn of<T>(span: &LocatedSpan<T, Rc<PathBuf>>) -> Self
	where T: AsBytes {
		Self {
			path: span.extra.clone(),
			line: span.location_line(),
			column: span.get_utf8_column(),
		}
	}
	/// 位置情報をすぐに引っ張れないところはとりあえずこれにしておく。最終的には廃止するつもり
	pub fn dummy() -> Self {
		Self{ path: Rc::new(PathBuf::from("")), line: 0, column: 0 }
	}
}
impl Display for Location {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}, line {}, column {}", self.path.to_str().unwrap_or("(not available)"), self.line, self.column)
	}
}

#[macro_export]
macro_rules! parser {
	($name: ident, $result_type: ty, $impl: expr) => {
		fn $name<'a>() -> impl FnMut (Span<'a>) -> IResult<Span<'a>, $result_type, nom::error::VerboseError<Span<'a>>> {
			$impl
		}
	}
}
#[macro_export]
macro_rules! pub_parser {
	($name: ident, $result_type: ty, $impl: expr) => {
		pub fn $name<'a>() -> impl FnMut (Span<'a>) -> IResult<Span<'a>, $result_type, nom::error::VerboseError<Span<'a>>> {
			$impl
		}
	}
}

pub fn loc<'a, O, E/* , F */>(mut f: impl FnMut(Span<'a>) -> IResult<Span<'a>, O, E>) -> impl FnMut(Span<'a>) -> IResult<Span<'a>, (O, Location), E>
where
//   F: Parser<Span<'a>, O, E>,
  E: ParseError<Span<'a>>,
{
	move |input| {
		let (input, span) = position(input) ?;
		let (input, result) = f(input) ?;
 		Ok((input, (result, Location::of(&span))))
	}
}

pub_parser![range_comment, char, {
	// コメントに対応したところ、type_length_limit を超過してエラーになったので
	// 手続きで書いた（が、解消しなかったので結局 type_length_limit を増やした）
	|input| {
		let (input, _) = tag("/*")(input) ?;
		let (input, _) = take_until("*/")(input) ?;
		let (input, _) = tag("*/")(input) ?;
		Ok((input, ' ' /* dummy */))
	}
}];
pub_parser![line_comment, char, {
	// コメントに対応したところ、type_length_limit を超過してエラーになったので
	// 手続きで書いた（が、解消しなかったので結局 type_length_limit を増やした）
	|input| {
		let (input, _) = tag("//")(input) ?;
		let (input, _) = many0(none_of("\r\n"))(input) ?;
		Ok((input, ' ' /* dummy */))
	}
}];

pub_parser![inline_space, char, {
	alt((
		char(' '),
		char('\t'),
		map_res(tag("\\\r\n"), |_| ok('\n')),
		map_res(tag("\\\r"), |_| ok('\r')),
		map_res(tag("\\\n"), |_| ok('\n')),
		range_comment(),
		line_comment(),
	))
}];

pub_parser![space, char, {
	alt((
		char(' '),
		char('\t'),
		map_res(tag("\\\r\n"), |_| ok('\n')),
		map_res(tag("\\\r"), |_| ok('\r')),
		map_res(tag("\\\n"), |_| ok('\n')),
		char('\r'),
		char('\n'),
		range_comment(),
		line_comment(),
	))
}];

/// skips following inline spaces if any
// TODO 本当は関数の方がいいかも
#[macro_export]
macro_rules! si {
	($parser: expr) => { terminated($parser, many0(inline_space())) }
}
/// skips following spaces including newlines if any
// TODO 本当は関数の方がいいかも
#[macro_export]
macro_rules! ss {
	($parser: expr) => { terminated($parser, many0(space())) }
}

pub_parser![integer, i32, {
	map_res(re_find(re(r"-?[0-9]+")),
			|matched| matched.parse::<i32>())
}];

pub_parser![float, f32, {
	map_res(re_find(re(r"[+-]?[0-9]+(\.[0-9]+)?|[+-]?\.[0-9]+")),
			|matched| matched.parse::<f32>())
}];

#[cfg(test)]
#[test]
fn test_float() {
	assert_eq!(float()("3.14"), Ok(("", 3.14f32)));
	// TODO 他にも
}

pub_parser![identifier, &str, {
	re_find(re(r"[a-zA-Z0-9_][a-zA-Z0-9_]*"))
}];
