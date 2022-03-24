use super::{
	error::*,
	value::*,
};

extern crate parser;
use parser::moddl::ast::*;

use crate::{
	core::common::*,
	operator::*,
};

use std::{
	any::TypeId,
	collections::hash_map::HashMap,
};



pub fn evaluate(expr: &Expr, vars: &HashMap<String, Value>) -> ModdlResult<Value> {
	match expr {
		Expr::Connect { lhs, rhs } => evaluate_binary_structure::<NoneOp>(lhs, rhs, vars, NodeStructure::Connect),
		Expr::Power { lhs, rhs } => evaluate_binary_structure::<PowOp>(lhs, rhs, vars, NodeStructure::Power),
		Expr::Multiply { lhs, rhs } => evaluate_binary_structure::<MulOp>(lhs, rhs, vars, NodeStructure::Multiply),
		Expr::Divide { lhs, rhs } => evaluate_binary_structure::<DivOp>(lhs, rhs, vars, NodeStructure::Divide),
		Expr::Remainder { lhs, rhs } => evaluate_binary_structure::<RemOp>(lhs, rhs, vars, NodeStructure::Remainder),
		Expr::Add { lhs, rhs } => evaluate_binary_structure::<AddOp>(lhs, rhs, vars, NodeStructure::Add),
		Expr::Subtract { lhs, rhs } => evaluate_binary_structure::<SubOp>(lhs, rhs, vars, NodeStructure::Subtract),
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

// 定数畳み込みに対応しない演算子に与えるダミーの演算
struct NoneOp { }
impl BinaryOp for NoneOp { fn oper(_lhs: Sample, _rhs: Sample) -> Sample { unreachable!() } }

fn evaluate_binary_structure<Op: BinaryOp + 'static>(
	lhs: &Expr,
	rhs: &Expr,
	vars: &HashMap<String, Value>,
	make_structure: fn(Box<NodeStructure>, Box<NodeStructure>) -> NodeStructure,
) -> ModdlResult<Value> {
	let l_val = evaluate(lhs, vars) ?;
	let r_val = evaluate(rhs, vars) ?;

	// 定数はコンパイル時に計算
	if TypeId::of::<Op>() != TypeId::of::<NoneOp>() {
		match (l_val.as_float(), r_val.as_float()) {
			(Some(l_float), Some(r_float)) => {
				return Ok(Value::Float(Op::oper(l_float, r_float)));
			}
			_ => { }
		}
	}

	let l_str = as_node_structure(&l_val) ?;
	let r_str = as_node_structure(&r_val) ?;
	Ok(Value::NodeStructure(make_structure(Box::new(l_str), Box::new(r_str))))
}

fn as_node_structure(val: &Value) -> ModdlResult<NodeStructure> {
	// TODO 型エラーはこれでいいのか。汎用の TypeMismatch エラーにすべきか
	Ok(val.as_node_structure().ok_or_else(|| Error::DirectiveArgTypeMismatch) ?)
}
fn evaluate_as_node_structure(expr: &Expr, vars: &HashMap<String, Value>) -> ModdlResult<NodeStructure> {
	Ok(as_node_structure(& evaluate(expr, vars) ?) ?)
}
