use super::{
	error::*,
	value::*,
};

extern crate parser;
use parser::moddl::ast::*;

use crate::{
	common::stack::*,
	core::common::*,
	operator::*,
};

use std::{
	any::TypeId,
	collections::hash_map::HashMap,
	rc::Rc,
};

pub type VarStack = Stack<HashMap<String, Value>>;

pub fn evaluate(expr: &Expr, vars: &mut VarStack) -> ModdlResult<Value> {
	match expr {
		Expr::Connect { lhs, rhs } => {
			let l_str = evaluate_as_node_structure(lhs, vars) ?;
			let r_str = evaluate_as_node_structure(rhs, vars) ?;
			Ok(Value::NodeStructure(NodeStructure::Connect(Box::new(l_str), Box::new(r_str))))
		},

		Expr::Power { lhs, rhs } => evaluate_binary_structure::<PowCalc>(lhs, rhs, vars),
		Expr::Multiply { lhs, rhs } => evaluate_binary_structure::<MulCalc>(lhs, rhs, vars),
		Expr::Divide { lhs, rhs } => evaluate_binary_structure::<DivCalc>(lhs, rhs, vars),
		Expr::Remainder { lhs, rhs } => evaluate_binary_structure::<RemCalc>(lhs, rhs, vars),
		Expr::Add { lhs, rhs } => evaluate_binary_structure::<AddCalc>(lhs, rhs, vars),
		Expr::Subtract { lhs, rhs } => evaluate_binary_structure::<SubCalc>(lhs, rhs, vars),
		Expr::Less { lhs, rhs } => evaluate_binary_structure::<LeCalc>(lhs, rhs, vars),
		Expr::LessOrEqual { lhs, rhs } => evaluate_binary_structure::<LeCalc>(lhs, rhs, vars),
		Expr::Equal { lhs, rhs } => evaluate_binary_structure::<EqCalc>(lhs, rhs, vars),
		Expr::NotEqual { lhs, rhs } => evaluate_binary_structure::<NeCalc>(lhs, rhs, vars),
		Expr::Greater { lhs, rhs } => evaluate_binary_structure::<GtCalc>(lhs, rhs, vars),
		Expr::GreaterOrEqual { lhs, rhs } => evaluate_binary_structure::<GeCalc>(lhs, rhs, vars),
		Expr::And { lhs, rhs } => evaluate_binary_structure::<AndCalc>(lhs, rhs, vars),
		Expr::Or { lhs, rhs } => evaluate_binary_structure::<OrCalc>(lhs, rhs, vars),

		Expr::Identifier(id) => {
			let val = vars.top().get(id.as_str()).ok_or_else(|| Error::VarNotFound { var: id.clone() }) ?;
			Ok(val.clone())
		},
		Expr::IdentifierLiteral(id) => Ok(Value::IdentifierLiteral(id.clone())),
		Expr::StringLiteral(content) => Ok(Value::StringLiteral(content.clone())),
		Expr::Lambda { input_param, body } => {
			vars.push_clone();
			vars.top_mut().insert(input_param.clone(), Value::NodeStructure(NodeStructure::Placeholder { name: input_param.clone() }));
			let result = Ok(Value::NodeStructure(NodeStructure::Lambda {
				input_param: input_param.clone(),
				body: Box::new(evaluate(body, vars)?.as_node_structure().ok_or_else(|| Error::TypeMismatch) ?),
			}));
			vars.pop();
			result
		},
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

		Expr::AssocArrayLiteral(_) => unimplemented!(),
		Expr::MmlLiteral(_) => unimplemented!(),

		Expr::Labeled { label, inner } => {
			let inner_val = evaluate(inner, vars) ?;

			Ok(Value::Labeled { label: label.clone(), inner: Box::new(inner_val) })
		}
	}
}

fn evaluate_binary_structure<C: Calc + 'static>(
	lhs: &Expr,
	rhs: &Expr,
	vars: &mut VarStack,
) -> ModdlResult<Value> {
	let l_val = evaluate(lhs, vars) ?;
	let r_val = evaluate(rhs, vars) ?;

	// 定数はコンパイル時に計算する。
	// ただしラベルがついているときは演奏中の設定の対象になるため計算しない
	if l_val.label().is_none() && r_val.label().is_none() {
		match (l_val.as_float(), r_val.as_float()) {
			(Some(l_float), Some(r_float)) => {
				return Ok(Value::Float(C::calc(&vec![l_float, r_float])));
			}
			_ => { }
		}
	}

	let l_str = as_node_structure(&l_val) ?;
	let r_str = as_node_structure(&r_val) ?;
	Ok(Value::NodeStructure(NodeStructure::Calc {
		node_factory: Rc::new(CalcNodeFactory::<C>::new()),
		args: vec![Box::new(l_str), Box::new(r_str)],
	}))
}

fn as_node_structure(val: &Value) -> ModdlResult<NodeStructure> {
	// TODO 型エラーはこれでいいのか。汎用の TypeMismatch エラーにすべきか
	Ok(val.as_node_structure().ok_or_else(|| Error::DirectiveArgTypeMismatch) ?)
}
fn evaluate_as_node_structure(expr: &Expr, vars: &mut VarStack) -> ModdlResult<NodeStructure> {
	Ok(as_node_structure(& evaluate(expr, vars) ?) ?)
}
