use super::{
	error::*,
	value::*,
};

extern crate parser;
use parser::moddl::ast::*;

use crate::{
	calc::*,
	common::stack::*,
};

use std::{
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
		Expr::Condition { cond, then, els } => evaluate_conditional_expr(cond, then, els, vars),
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
		Expr::FunctionCall { function, args } => {
			let function = evaluate(function, vars)?.as_function().ok_or_else(|| Error::TypeMismatch) ?;

			let arg_names = function.signature().iter().map(|name| name.to_string()).collect();
			let resolved_args = resolve_args(&arg_names, args) ?;
			let mut value_args = HashMap::new();
			// TODO map() を使いたいがクロージャで ? を使っているとうまくいかず。いい書き方があれば修正
			for (name, expr) in &resolved_args {
				value_args.insert(name.clone(), evaluate(expr, vars) ?);
			}

			function.call(&value_args)
		},
		Expr::NodeWithArgs { node_def, label, args } => {
			let factory = evaluate_as_node_structure(node_def, vars) ?;
			let arg_names = match &factory {
				NodeStructure::NodeFactory(factory) => factory.node_arg_specs(),
				_ => return Err(Error::TypeMismatch),
			}.iter().map(|spec| spec.name.clone()).collect();
			let resolved_args = resolve_args(&arg_names, args) ?;

			// TODO map() を使いたいがクロージャで ? を使っているとうまくいかず。いい書き方があれば修正
			let mut value_args = HashMap::new();
			for (name, expr) in &resolved_args {
				value_args.insert(name.clone(), evaluate(expr, vars) ?);
			}

			Ok(Value::NodeStructure(NodeStructure::NodeWithArgs {
				factory: Box::new(factory),
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
			_ => { } // 下へ
		}
	}

	let l_str = as_node_structure(&l_val) ?;
	let r_str = as_node_structure(&r_val) ?;
	Ok(Value::NodeStructure(NodeStructure::Calc {
		node_factory: Rc::new(CalcNodeFactory::<C>::new()),
		args: vec![Box::new(l_str), Box::new(r_str)],
	}))
}

fn evaluate_conditional_expr(cond: &Expr, then: &Expr, els: &Expr, vars: &mut VarStack) -> ModdlResult<Value> {
	let cond_val = evaluate(cond, vars) ?;
	let then_val = evaluate(then, vars) ?;
	let else_val = evaluate(els, vars) ?;

	// 定数はコンパイル時に計算する。
	// ただしラベルがついているときは演奏中の設定の対象になるため計算しない
	// TODO まだ最適化の余地あり：3 つとも定数でなくても、cond さえ定数であれば（かつラベルがなければ）
	// TODO   どちらになるかはコンパイル時に決めれるはず。追々作り込む
	if cond_val.label().is_none() && then_val.label().is_none() && else_val.label().is_none() {
		match (cond_val.as_boolean(), then_val.as_float(), else_val.as_float()) {
			(Some(cond_bool), Some(then_float), Some(else_float)) => {
				return Ok(Value::Float(if cond_bool { then_float } else { else_float }));
			}
			_ => { } // 下へ
		}
	}

	let cond_str = as_node_structure(&cond_val) ?;
	let then_str = as_node_structure(&then_val) ?;
	let else_str = as_node_structure(&else_val) ?;
	Ok(Value::NodeStructure(NodeStructure::Condition {
		cond: Box::new(cond_str),
		then: Box::new(then_str),
		els: Box::new(else_str),
	}))
}

fn as_node_structure(val: &Value) -> ModdlResult<NodeStructure> {
	// TODO 型エラーはこれでいいのか。汎用の TypeMismatch エラーにすべきか
	Ok(val.as_node_structure().ok_or_else(|| Error::DirectiveArgTypeMismatch) ?)
}
fn evaluate_as_node_structure(expr: &Expr, vars: &mut VarStack) -> ModdlResult<NodeStructure> {
	Ok(as_node_structure(& evaluate(expr, vars) ?) ?)
}

/**
 * 引数名を省略した実引数を、要求された引数リストと照合して解決することで、全ての引数を「引数名」と「式」の対応関係にする。
 * 引数の重複もチェックする（引数名が省略されていても解決してからチェックする）
 */
fn resolve_args<'a>(arg_names: &'a Vec<String>, args: &'a Args) -> ModdlResult<HashMap<String, &'a Box<Expr>>> {
	let mut result = HashMap::<String, &'a Box<Expr>>::new();
	let mut add = |name: &String, expr: &'a Box<Expr>| -> ModdlResult<()> {
		if result.contains_key(name) { return Err(Error::EntryDuplicate { name: name.clone() }); }

		result.insert(name.clone(), expr);
		Ok(())
	};

	// TODO 無名引数の過剰チェックをするなら未知の引数もエラーにする。しないならしない。で統一する
	// if args.unnamed.len() > arg_names.len() { return Err(Error::TooManyUnnamedArgs) };

	let mut unnamed_args = arg_names.iter().zip(& args.unnamed);
	unnamed_args.try_for_each(|(name, arg)| add(name, arg)) ?;
	args.named.iter().try_for_each(|(name, arg)| add(name, arg)) ?;

	Ok(result)
}
