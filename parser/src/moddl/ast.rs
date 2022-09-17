#[derive(Debug)]
pub struct CompilationUnit {
	pub statements: Vec<Statement>,
}

#[derive(Debug)]
pub enum Statement {
	Directive { name: String, args: Vec<Expr> },
	Mml { tracks: Vec<String>, mml: String },
}

pub type AssocArray = Vec<(String, Box<Expr>)>;

#[derive(Debug)]
pub struct Args {
	pub unnamed: Vec<Box<Expr>>,
	pub named: AssocArray,
}

#[derive(Debug)]
pub  struct FuncParam {
	pub name: String,
	pub default: Option<Box<Expr>>,
}

#[derive(Debug)]
pub enum Expr {
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
	Identifier(String),
	Condition { cond: Box<Expr>, then: Box<Expr>, els: Box<Expr> },
	LambdaFunc { params: Vec<FuncParam>, body: Box<Expr> },
	LambdaNode { input_param: String, body: Box<Expr> },
	FunctionCall { function: Box<Expr>, args: Args },
	NodeWithArgs { node_def: Box<Expr>, label: String, args: Args },

	FloatLiteral(f32),
	TrackSetLiteral(Vec<String>),
	IdentifierLiteral(String),
	StringLiteral(String),
	MmlLiteral(String),
	AssocArrayLiteral(AssocArray),

	Labeled { label: String, inner: Box<Expr> },
}
