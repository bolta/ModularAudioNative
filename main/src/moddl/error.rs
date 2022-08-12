use std::{
	convert::From,
	io,
};

type NomError<'a> = nom::Err<nom::error::VerboseError<&'a str>>;

// それぞれのエラーに十分な付加情報を含めるべきだが、とりあえずはざっと分類まで
#[derive(Debug)]
pub enum Error/* <'a> */ {
	Syntax(String/* NomError<'a> */),
	MmlSyntax, // TODO MML パーザから返されたエラーをラップする
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
	EntryDuplicate { name: String },
	TooManyUnnamedArgs,

	// TODO イベントキューあふれとかテンポずれとか、演奏時のエラーをラップする
	Playing,
	File(io::Error),
}

pub type ModdlResult</* 'a, */ T> = Result<T, Error/* <'a> */>;

// TODO Error 全体が MML の寿命に影響されるのがまずいので format! をかましてしまっているが、どうするのがいいのか？
// MML のエラー表示をやる際に再検討
impl <'a> From<NomError<'a>> for Error/* <'a> */ {
	fn from(nom_err: NomError<'a>) -> Self {
		Self::Syntax(format!("{}", nom_err))
	}
}
impl <'a> From<io::Error> for Error {
	fn from(io_err: io::Error) -> Self {
		Self::File(io_err)
	}
}
