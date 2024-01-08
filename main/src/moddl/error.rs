use std::{
	convert::From,
	io,
};
use std::fmt::Display;

use parser::common::Span;


type NomError = nom::Err<nom::error::VerboseError<String>>;

// それぞれのエラーに十分な付加情報を含めるべきだが、とりあえずはざっと分類まで
#[derive(Debug)]
pub enum Error {
	Syntax(NomError),
	// MmlSyntax(NomError),
	// TODO ↑テンポずれも同様のエラーで捕捉
	InstrumentNotFound { track: String },
	DirectiveArgNotFound,
	DirectiveArgTypeMismatch, // TODO 今後 TypeMismatch に統合
	DirectiveDuplicate { msg: String }, // TODO ここだけ msg を自前で持つのは変かも…全体でしくみを考える
	VarNotFound { var: String },
	NodeFactoryNotFound,
	NodeFactoryArgTypeMismatch, // TODO 今後 TypeMismatch に統合
	// TODO 「NodeStructure の解析中に、NodeStructure に変換できない値が出てきた」は何エラーにしよう…ここまでのどれかに含めれるか？
	// TODO 「piped_upstreams の個数（過）不足」は、内部エラーで panic でもいいか？
	ChannelMismatch,
	TypeMismatch,
	ArgMissing { name: String },
	SignatureMismatch, // map や filter に渡す関数の arity が 1 でないなど
	EntryDuplicate { name: String },
	EntryNotFound { name: String },
	TooManyUnnamedArgs,
	TooManyTracks,
	GrooveTargetDuplicate { track: String },

	// TODO イベントキューあふれとかテンポずれとか、演奏時のエラーをラップする
	Playing,
	File(io::Error),
}

impl Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}", self)
	}
}

pub type ModdlResult<T> = Result<T, Error>;

impl <'a> From<nom::Err<nom::error::VerboseError<Span<'a>>>> for Error {
	fn from(nom_err: nom::Err<nom::error::VerboseError<Span<'a>>>) -> Self {
		// エラーがソースコードの寿命に干渉されると不便なので、
		// VerboseError<&str> から VerboseError<String> に変換する。
		// FIXME e.to_owned() をかませばよいかと思いきや、それでは &str から変わってくれなかったので、
		// 中身を 1 つずつ変換したが、これでいいのか？
		let nom_err_by_string = nom_err.map(|e| {
			nom::error::VerboseError {
				errors: e.errors.into_iter().map(|(part, kind)| (part.to_string(), kind)).collect(),
			}
		});
		Self::Syntax(nom_err_by_string)
	}
}
impl From<io::Error> for Error {
	fn from(io_err: io::Error) -> Self {
		Self::File(io_err)
	}
}
