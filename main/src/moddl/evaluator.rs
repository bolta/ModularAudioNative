use super::{
	error::*,
	value::*,
};

extern crate parser;
use parser::moddl::ast::*;

use std::{
	collections::hash_map::HashMap,
};



pub fn evaluate(expr: &Expr, vars: &HashMap<String, Value>) -> ModdlResult<Value> {
	match expr {
		// TODO lhs, rhs 両方が数値の場合は計算を行う
		Expr::Connect { lhs, rhs } => evaluate_binary_structure(lhs, rhs, vars, NodeStructure::Connect),
		Expr::Power { lhs, rhs } => evaluate_binary_structure(lhs, rhs, vars, NodeStructure::Power),
		Expr::Multiply { lhs, rhs } => evaluate_binary_structure(lhs, rhs, vars, NodeStructure::Multiply),
		Expr::Divide { lhs, rhs } => evaluate_binary_structure(lhs, rhs, vars, NodeStructure::Divide),
		Expr::Remainder { lhs, rhs } => evaluate_binary_structure(lhs, rhs, vars, NodeStructure::Remainder),
		Expr::Add { lhs, rhs } => evaluate_binary_structure(lhs, rhs, vars, NodeStructure::Add),
		Expr::Subtract { lhs, rhs } => evaluate_binary_structure(lhs, rhs, vars, NodeStructure::Subtract),
		Expr::Identifier(id) => {
			let val = vars.get(id.as_str()).ok_or_else(|| Error::VarNotFound { var: id.clone() }) ?;
			Ok(val.clone())
		},
		Expr::IdentifierLiteral(id) => Ok(Value::IdentifierLiteral(id.clone())),
		Expr::StringLiteral(content) => Ok(Value::StringLiteral(content.clone())),
		Expr::Lambda { input_param: _, body: _ } => unimplemented!(),
		// Expr::ModuleParamExpr { module_def, label: String, ctor_params: AssocArray, signal_params: AssocArray } => {}
		Expr::FloatLiteral(value) => Ok(Value::Float(*value)),
		Expr::TrackSetLiteral(tracks) => Ok(Value::TrackSet(tracks.clone())),
		// Expr::MmlLiteral(String) => {}
		// Expr::AssocArrayLiteral(AssocArray) => {}
		Expr::FunctionCall { function, unnamed_args, named_args } => {
			let function = evaluate(function, vars)?.as_function().ok_or_else(|| Error::TypeMismatch) ?;

			// TODO unnamed_args も使う
			let mut value_args = HashMap::<String, Value>::new();
			for (name, expr) in named_args {
				value_args.insert(name.clone(), evaluate(expr, vars) ?);
			}

			function.call(&value_args)
		},
		Expr::NodeWithArgs { node_def, label, args } => {
			// TODO map() を使いたいがクロージャで ? を使っているとうまくいかず。いい書き方があれば修正
			let mut value_args = vec![];
			for (name, expr) in args {
				value_args.push((name.clone(), evaluate(expr, vars) ?));
			}

			Ok(Value::NodeStructure(NodeStructure::NodeWithArgs {
				factory: Box::new(evaluate_as_node_structure(node_def, vars) ?),
				label: label.clone(),
				args: value_args,
			}))
		},

		_ => unimplemented!(),
	}
}

fn evaluate_binary_structure(
	lhs: &Expr,
	rhs: &Expr,
	vars: &HashMap<String, Value>,
	make_structure: fn(Box<NodeStructure>, Box<NodeStructure>) -> NodeStructure,
) -> ModdlResult<Value> {
	// TODO 
	let l_str = evaluate_as_node_structure(lhs, vars) ?;
	let r_str = evaluate_as_node_structure(rhs, vars) ?;
	Ok(Value::NodeStructure(make_structure(Box::new(l_str), Box::new(r_str))))
}
fn evaluate_as_node_structure(expr: &Expr, vars: &HashMap<String, Value>) -> ModdlResult<NodeStructure> {
	// TODO 型エラーはこれでいいのか。汎用の TypeMismatch エラーにすべきか
	Ok(evaluate(expr, vars)?.as_node_structure().ok_or_else(|| Error::DirectiveArgTypeMismatch) ?)
}
