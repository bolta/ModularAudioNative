use super::{
	ast::*,
	value::*,
};

pub fn evaluate(expr: &Expr /* context */) -> Value {
	match expr {
		// Expr::Connect { lhs: Box<Expr>, rhs: Box<Expr> } => {}
		// Expr::Power { lhs: Box<Expr>, rhs: Box<Expr> } => {}
		// Expr::Multiply { lhs: Box<Expr>, rhs: Box<Expr> } => {}
		// Expr::Divide { lhs: Box<Expr>, rhs: Box<Expr> } => {}
		// Expr::Remainder { lhs: Box<Expr>, rhs: Box<Expr> } => {}
		// Expr::Add { lhs: Box<Expr>, rhs: Box<Expr> } => {}
		// Expr::Subtract { lhs: Box<Expr>, rhs: Box<Expr> } => {}
		// Expr::Identifier(String) => {}
		// Expr::Lambda { input_param: String, body: Box<Expr> } => {}
		// Expr::ModuleParamExpr { module_def: Box<Expr>, label: String, ctor_params: AssocArray, signal_params: AssocArray } => {}
		Expr::FloatLiteral(value) => Value::Float(*value),
		Expr::TrackSetLiteral(tracks) => Value::TrackSet(tracks.clone()),
		Expr::IdentifierLiteral(id) => Value::Identifier(id.clone()),
		// Expr::MmlLiteral(String) => {}
		// Expr::AssocArrayLiteral(AssocArray) => {}

		_ => unimplemented!()
	}
}
