use std::{fmt::Display, write};

use itertools::Itertools;
use serde::Serialize;

use crate::common::Location;

pub fn to_json(ser: &impl Serialize) -> String {
	serde_json::to_string(ser).unwrap()
}

#[derive(Clone, Debug, Serialize)]
pub struct CompilationUnit {
	pub statements: Vec<(Statement, Location)>,
}

#[derive(Clone, Debug, Serialize)]
pub enum Statement {
	Construction { name: String, args: Vec<Expr> },
	Mml { tracks: Vec<String>, mml: String },
}

pub type Assoc = Vec<(String, Box<Expr>)>;

#[derive(Clone, Debug, Serialize)]
pub struct Args {
	pub unnamed: Vec<Box<Expr>>,
	pub named: Assoc,
}
impl Args {
	pub fn empty() -> Self {
		Self { unnamed: vec![], named: vec![] }
	}
}

#[derive(Clone, Debug, Serialize)]
pub struct FunctionParam {
	pub name: String,
	pub default: Option<Box<Expr>>,
}

/// foo.bar.baz みたいな . でつながった識別子（絶対パスとは限らない）
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct QualifiedLabel {
	/// foo.bar.baz の場合 ["foo", "bar"]
	qualifiers: Vec<String>,

	/// foo.bar.baz の場合 "baz"
	local: String,
}
impl QualifiedLabel {
	pub fn new(qualifiers: Vec<impl Into<String>>, local: impl Into<String>) -> Self {
		Self {
			qualifiers: qualifiers.into_iter().map(Into::into).collect(),
			local: local.into(),
		}
	}
	pub fn local(local: impl Into<String>) -> Self {
		Self { qualifiers: vec![], local: local.into() }
	}

	pub fn join(&self, descendant: &Self) -> Self {
		Self {
			qualifiers: self.elems().chain(descendant.qualifiers.iter().map(String::as_str)).map(&str::to_string).collect(),
			local: descendant.local.clone(),
		}
	}

	pub fn prepend(&self, parent: impl Into<String>) -> Self {
		let mut result = self.clone();
		result.qualifiers.insert(0, parent.into());
		result
	}

	pub fn append(&self, child: impl Into<String>) -> Self {
		Self::new(self.elems().map(&str::to_string).collect(), child)
	}

	pub fn elems(&self) -> impl Iterator<Item = &str> {
		self.qualifiers.iter()
				.chain(std::iter::once(&self.local))
				.map(String::as_str)
	} 
}
impl Display for QualifiedLabel {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.elems().join("."))
	}
}

#[derive(Clone, Debug, Serialize)]
pub enum LabelFilterSpec {
	AllowAll,
	Allow(QualifiedLabel),
	Deny(QualifiedLabel),
	Rename(QualifiedLabel, QualifiedLabel),
}

pub type Expr = (ExprBody, Location);

#[derive(Clone, Debug, Serialize)]
pub enum ExprBody {
	Connect { lhs: Box<Expr>, rhs: Box<Expr> },
	Power { lhs: Box<Expr>, rhs: Box<Expr> },
	Multiply { lhs: Box<Expr>, rhs: Box<Expr> },
	Divide { lhs: Box<Expr>, rhs: Box<Expr> },
	Remainder { lhs: Box<Expr>, rhs: Box<Expr> },
	Add { lhs: Box<Expr>, rhs: Box<Expr> },
	Subtract { lhs: Box<Expr>, rhs: Box<Expr> },
	Less { lhs: Box<Expr>, rhs: Box<Expr> },
	LessOrEqual { lhs: Box<Expr>, rhs: Box<Expr> },
	Equal { lhs: Box<Expr>, rhs: Box<Expr> },
	NotEqual { lhs: Box<Expr>, rhs: Box<Expr> },
	Greater { lhs: Box<Expr>, rhs: Box<Expr> },
	GreaterOrEqual { lhs: Box<Expr>, rhs: Box<Expr> },
	And { lhs: Box<Expr>, rhs: Box<Expr> },
	Or { lhs: Box<Expr>, rhs: Box<Expr> },
	Not { arg: Box<Expr> },
	Negate { arg: Box<Expr> },
	Plus { arg: Box<Expr> },
	Identifier(String),
	Condition { cond: Box<Expr>, then: Box<Expr>, els: Box<Expr> },
	FunctionCall { function: Box<Expr>, args: Args },
	PropertyAccess { assoc: Box<Expr>, assoc_loose: bool, prop_loose: bool, name: String },
	NodeWithArgs { node_def: Box<Expr>, args: Args },

	Number(f32),
	// 「全てのトラック」を表す * は、とりあえずそのまま文字列 "*" として持つことにする。
	// 構文上 "*" は必ず唯一の要素となる（^*ab のような記述は許されない）
	TrackSet(Vec<String>),
	QuotedIdentifier(String),
	String(String),
	// FIXME この Box は取り除ける？
	Array(Vec<Box<Expr>>),
	Assoc(Assoc),
	Function { params: Vec<FunctionParam>, body: Box<Expr> },
	InputRef { input_param: String, body: Box<Expr> },

	Labeled { label: QualifiedLabel, inner: Box<Expr> },
	LabelFilter { strukt: Box<Expr>, filter: Vec<LabelFilterSpec> },
	LabelPrefix { strukt: Box<Expr>, prefix: QualifiedLabel },
}
