use crate::common::{Location, Located};

#[derive(Debug)]
pub struct CompilationUnit {
	pub statements: Vec<(Statement, Location)>,
}

#[derive(Debug)]
pub enum Statement {
	Directive { name: String, args: Vec<Expr> },
	Mml { tracks: Vec<String>, mml: String },
}

pub type Assoc = Vec<(String, Box<Expr>)>;

#[derive(Clone, Debug)]
pub struct Args {
	pub unnamed: Vec<Box<Expr>>,
	pub named: Assoc,
}
impl Args {
	pub fn empty() -> Self {
		Self { unnamed: vec![], named: vec![] }
	}
}

#[derive(Clone, Debug)]
pub struct FunctionParam {
	pub name: String,
	pub default: Option<Box<Expr>>,
}

/// foo.bar.baz みたいな . でつながった識別子
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct QualifiedLabel(pub String);

#[derive(Clone, Debug)]
pub enum LabelFilterSpec {
	AllowAll,
	Allow(QualifiedLabel),
	Deny(QualifiedLabel),
	Rename(QualifiedLabel, QualifiedLabel),
}

pub type Expr = Located<ExprBody>;

#[derive(Clone, Debug)]
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
	Negate { arg: Box<Expr> },
	Identifier(String),
	Condition { cond: Box<Expr>, then: Box<Expr>, els: Box<Expr> },
	LambdaFunction { params: Vec<FunctionParam>, body: Box<Expr> },
	LambdaNode { input_param: String, body: Box<Expr> },
	FunctionCall { function: Box<Expr>, args: Args },
	PropertyAccess { assoc: Box<Expr>, name: String },
	NodeWithArgs { node_def: Box<Expr>, args: Args },

	FloatLiteral(f32),
	TrackSetLiteral(Vec<String>),
	IdentifierLiteral(String),
	StringLiteral(String),
	// FIXME この Box は取り除ける？
	ArrayLiteral(Vec<Box<Expr>>),
	AssocLiteral(Assoc),
	MmlLiteral(String),

	Labeled { label: QualifiedLabel, inner: Box<Expr> },
	LabelFilter { strukt: Box<Expr>, filter: Vec<LabelFilterSpec> },
}
