use std::{
	convert::From,
	io,
};
use std::fmt::Display;

use itertools::Itertools;
use parser::common::{Span, Located, Location};
use parser::mml::ast::Length;

use super::value::ValueType;

type NomError = nom::Err<nom::error::VerboseError<String>>;

pub type Error = Located<ErrorType>;

pub fn error(tipe: ErrorType, loc: Location) -> Error {
	Located::new(tipe, loc)
}

// それぞれのエラーに十分な付加情報を含めるべきだが、とりあえずはざっと分類まで
#[derive(Debug)]
pub enum ErrorType {
	Syntax(NomError),
	MmlSyntax(NomError),
	// TODO ↑テンポずれも同様のエラーで捕捉
	DirectiveArgNotFound,
	TrackDefNotFound { track: String },
	TrackDefDuplicate { track: String, existing_def_loc: Location }, // TODO ここだけ msg を自前で持つのは変かも…全体でしくみを考える
	VarNotFound { var: String },
	NodeFactoryNotFound, // TODO 発生条件確認
	// TODO 「NodeStructure の解析中に、NodeStructure に変換できない値が出てきた」は何エラーにしよう…ここまでのどれかに含めれるか？
	// TODO 「piped_upstreams の個数（過）不足」は、内部エラーで panic でもいいか？
	ChannelMismatch,
	// TypeMismatchAny だけでもいい気はする
	TypeMismatch { expected: ValueType },
	TypeMismatchAny { expected: Vec<ValueType> },
	ArgMissing { name: String },
	ArityMismatch { expected: usize, actual: usize }, // map や filter に渡す関数の arity が 1 でないなど
	EntryDuplicate { name: String },
	EntryNotFound { name: String },
	TooManyUnnamedArgs,
	GrooveControllerTrackMustBeSingle,
	GrooveTargetDuplicate { track: String, existing_assign_loc: Location },
	OptionNotAllowedHere,
	IndexOutOfBounds,

	TickUnderflow { length: Length },
	// TODO イベントキューあふれとか、演奏時のエラーをラップする
	Playing,
	File(io::Error),
}
impl Display for ErrorType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Syntax(nom_error) => write!(f, "ModDL syntax error: {}", nom_error),
			Self::MmlSyntax(nom_error) => write!(f, "MML syntax error (sorry, error location is wrong for some reason): {}", nom_error),
			Self::DirectiveArgNotFound => write!(f, "Not enough arguments are given for directive statement."),
			Self::TrackDefNotFound { track } => write!(f, "MML is given for track ^{} but track definition is missing.", track),
			Self::TrackDefDuplicate { track, existing_def_loc }
					=> write!(f, "Definition for track ^{} is duplicate: definition already exists at {}.", track, existing_def_loc),
			Self::VarNotFound { var } => write!(f, "Variable `{}` not found.", var),
			// NodeFactoryNotFound,
			// ChannelMismatch,
			Self::TypeMismatch { expected }=> write!(f, "Type mismatch: expected: {}", expected),
			Self::TypeMismatchAny { expected }=> write!(f, "Type mismatch: expected one of: {}", expected.iter().join(", ")),
			Self::ArgMissing { name } => write!(f, "Function argument `{}` missing.", name),
			Self::ArityMismatch { expected, actual }
					=> write!(f, "Arity mismatch: Given function is expected to take {} argument{}, but actually takes {}.",
							expected, if *expected == 1 { "" } else { "s" }, actual),
			// EntryDuplicate { name: String },
			// EntryNotFound { name: String },
			// TooManyUnnamedArgs,
			Self::GrooveControllerTrackMustBeSingle => write!(f, "Groove controller track must be single."),
			Self::GrooveTargetDuplicate { track, existing_assign_loc }
					=> write!(f, "Groove controller track for track {} is duplicate: already assigned at {}.", track, existing_assign_loc),
			Self::OptionNotAllowedHere => write!(f, "Options must be placed at the head of a source file."),
			// Playing,
			// File(io::Error),
			// TODO 全種類ちゃんと作る
			_ => write!(f, "{:?}", self),
		}
	}
}

pub type ModdlResult<T> = Result<T, Error>;

// エラーがソースコードの寿命に干渉されると不便なので、
// VerboseError<&str> から VerboseError<String> に変換する。
// FIXME e.to_owned() をかませばよいかと思いきや、それでは &str から変わってくれなかったので、
// 中身を 1 つずつ変換したが、これでいいのか？
pub fn nom_error_to_owned<'a>(nom_err: nom::Err<nom::error::VerboseError<Span<'a>>>) -> nom::Err<nom::error::VerboseError<String>> {
	nom_err.map(|e| {
		nom::error::VerboseError {
			errors: e.errors.into_iter().map(|(part, kind)| (part.to_string(), kind)).collect(),
		}
	})
}

impl From<io::Error> for ErrorType {
	fn from(io_err: io::Error) -> Self {
		Self::File(io_err)
	}
}
