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

use regex::Regex;

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

pub fn ok<T>(value: T) -> Result<T, ()> { Ok::<_, ()>(value) }

pub fn re(pattern: &str) -> Regex {
	// re_find() はマッチするところまで入力を読み飛ばしてしまう
	// （re_find(re(r"\d"))("abc1") → Ok(("", "1"))）
	// これは都合が悪いので、^ を補うようにする
	let head_match_pattern = format!(r"^(?:{})", pattern);

	Regex::new(head_match_pattern.as_str()).unwrap()
}


//type Parser<'a, Result> = FnMut (&'a str) -> IResult<&'a str, Result>;

#[macro_export]
macro_rules! parser {
	($name: ident, $result_type: ty, $impl: expr) => {
		fn $name<'a>() -> impl FnMut (&'a str) -> IResult<&'a str, $result_type> {
			$impl
		}
	}
}
#[macro_export]
macro_rules! pub_parser {
	($name: ident, $result_type: ty, $impl: expr) => {
		pub fn $name<'a>() -> impl FnMut (&'a str) -> IResult<&'a str, $result_type> {
			$impl
		}
	}
}

/// skips following inline spaces if any
// TODO 本当は関数の方がいいかも
#[macro_export]
macro_rules! si {
	($parser: expr) => { terminated($parser, space0) }
}
/// skips following spaces including newlines if any
// TODO 本当は関数の方がいいかも
#[macro_export]
macro_rules! ss {
	($parser: expr) => { terminated($parser, multispace0) }
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
