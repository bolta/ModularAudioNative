use super::{
	error::*,
	lambda_function::*,
	scope::*,
	value::*,
};

extern crate parser;
use parser::{moddl::ast::*, common::Location};

use crate::{
	calc::*,
};

use std::{
	cell::RefCell,
	collections::hash_map::HashMap,
	rc::Rc,
};

pub fn evaluate(expr: &Expr, vars: &Rc<RefCell<Scope>>) -> ModdlResult<Value> {
	let body = match &expr.body {
		ExprBody::Connect { lhs, rhs } => {
			let (l_str, _) = evaluate(lhs, vars)?.as_node_structure() ?;
			let (r_str, _) = evaluate(rhs, vars)?.as_node_structure() ?;
			Ok(ValueBody::NodeStructure(NodeStructure::Connect(Box::new(l_str), Box::new(r_str))))
		},

		ExprBody::Power { lhs, rhs } => evaluate_binary_structure::<PowCalc>(lhs, rhs, vars),
		ExprBody::Multiply { lhs, rhs } => evaluate_binary_structure::<MulCalc>(lhs, rhs, vars),
		ExprBody::Divide { lhs, rhs } => evaluate_binary_structure::<DivCalc>(lhs, rhs, vars),
		ExprBody::Remainder { lhs, rhs } => evaluate_binary_structure::<RemCalc>(lhs, rhs, vars),
		ExprBody::Add { lhs, rhs } => evaluate_binary_structure::<AddCalc>(lhs, rhs, vars),
		ExprBody::Subtract { lhs, rhs } => evaluate_binary_structure::<SubCalc>(lhs, rhs, vars),
		ExprBody::Less { lhs, rhs } => evaluate_binary_structure::<LeCalc>(lhs, rhs, vars),
		ExprBody::LessOrEqual { lhs, rhs } => evaluate_binary_structure::<LeCalc>(lhs, rhs, vars),
		ExprBody::Equal { lhs, rhs } => evaluate_binary_structure::<EqCalc>(lhs, rhs, vars),
		ExprBody::NotEqual { lhs, rhs } => evaluate_binary_structure::<NeCalc>(lhs, rhs, vars),
		ExprBody::Greater { lhs, rhs } => evaluate_binary_structure::<GtCalc>(lhs, rhs, vars),
		ExprBody::GreaterOrEqual { lhs, rhs } => evaluate_binary_structure::<GeCalc>(lhs, rhs, vars),
		ExprBody::And { lhs, rhs } => evaluate_binary_structure::<AndCalc>(lhs, rhs, vars),
		ExprBody::Or { lhs, rhs } => evaluate_binary_structure::<OrCalc>(lhs, rhs, vars),

		ExprBody::Negate { arg } => evaluate_unary_structure::<NegCalc>(arg, vars),

		ExprBody::Identifier(id) => {
			let (val, _) = vars.borrow().lookup(id).ok_or_else(|| { error(ErrorType::VarNotFound { var: id.clone() }, expr.loc.clone()) }) ?;
			Ok(val.clone())
		},
		ExprBody::IdentifierLiteral(id) => Ok(ValueBody::IdentifierLiteral(id.clone())),
		ExprBody::StringLiteral(content) => Ok(ValueBody::String(content.clone())),
		ExprBody::ArrayLiteral(content) => {
			// TODO map() を使いたいがクロージャで ? を使っているとうまくいかず。いい書き方があれば修正
			let mut result = vec![];
			for elem in content {
				result.push(evaluate(&*elem, vars) ?);
			}
			Ok(ValueBody::Array(result))
		},
		ExprBody::AssocLiteral(content) => {
			// TODO map() を使いたいがクロージャで ? を使っているとうまくいかず。いい書き方があれば修正
			let mut result = HashMap::<String, Value>::with_capacity(content.len());
			for (key, value_expr) in content {
				result.insert(key.clone(), evaluate(&*value_expr, vars) ?);
			}
			Ok(ValueBody::Assoc(result))
		},
		ExprBody::Condition { cond, then, els } => evaluate_conditional_expr(cond, then, els, vars),
		ExprBody::LambdaFunction { params, body } => {
			let mut param_values: Vec<Param> = vec![];
			params.iter().try_for_each(|param| {
				param_values.push(Param {
					name: param.name.clone(),
					default: match &param.default { // param.default.map で書きたいがうまくいかず
						None => None,
						Some(default) => Some(evaluate(&*default, vars) ?),
					},
				});
				ModdlResult::Ok(())
			}) ?;
			// Value 側で式を使う必要があるので、単純に式を clone して持たせておく。
			// 何とかして参照した方が効率的だが
			Ok(ValueBody::Function(Rc::new(LambdaFunction::new(param_values, *body.clone(), vars))))
		}
		ExprBody::LambdaNode { input_param, body } => {
			let vars = Scope::child_of(vars.clone());
			vars.borrow_mut().set(input_param,
					(ValueBody::NodeStructure(NodeStructure::Placeholder { name: input_param.clone() }), expr.loc.clone())) ?;
			let result = Ok(ValueBody::NodeStructure(NodeStructure::Lambda {
				input_param: input_param.clone(),
				body: Box::new(evaluate(body, &vars)?.as_node_structure()?.0), // TODO loc も引き渡す
			}));
			result
		},
		// Expr::ModuleParamExpr { module_def, label: String, ctor_params: AssocArray, signal_params: AssocArray } => {}
		ExprBody::FloatLiteral(value) => Ok(ValueBody::Float(*value)),
		ExprBody::TrackSetLiteral(tracks) => Ok(ValueBody::TrackSet(tracks.clone())),
		// Expr::MmlLiteral(String) => {}
		// Expr::AssocArrayLiteral(AssocArray) => {}
		ExprBody::FunctionCall { function, args } => {
			let (function, _) = evaluate(function, vars)?.as_function() ?;

			let arg_names = function.signature().iter().map(|name| name.to_string()).collect();
			let resolved_args = resolve_args(&arg_names, args, &expr.loc) ?;
			let mut value_args = HashMap::new();
			// TODO map() を使いたいがクロージャで ? を使っているとうまくいかず。いい書き方があれば修正
			for (name, expr) in &resolved_args {
				value_args.insert(name.clone(), evaluate(expr, vars) ?);
			}

			function.call(&value_args, &vars, expr.loc.clone()).map(|(v, _)| v)
		},
		ExprBody::PropertyAccess { assoc, name } => {
			let assoc_val = evaluate(assoc, vars) ?;
			let (assoc, _) = assoc_val.as_assoc() ?;
			let val = assoc.get(name);
			Ok(val.map(|(v, _)| v).ok_or_else(|| error(ErrorType::EntryNotFound { name: name.clone() }, expr.loc.clone()))?.clone())
		},
		ExprBody::NodeWithArgs { node_def, label, args } => {
			let (factory, _) = evaluate(node_def, vars)?.as_node_structure() ?;
			let arg_names = match &factory {
				NodeStructure::NodeFactory(factory) => factory.node_arg_specs(),
				// TODO これで適切なエラーになるかどうか
				_ => return Err(error(ErrorType::TypeMismatch { expected: ValueType::NodeFactory }, expr.loc.clone())),
			}.iter().map(|spec| spec.name.clone()).collect();
			let resolved_args = resolve_args(&arg_names, args, &expr.loc) ?;

			// TODO map() を使いたいがクロージャで ? を使っているとうまくいかず。いい書き方があれば修正
			let mut value_args = HashMap::new();
			for (name, expr) in &resolved_args {
				value_args.insert(name.clone(), evaluate(expr, vars) ?);
			}

			Ok(ValueBody::NodeStructure(NodeStructure::NodeWithArgs {
				factory: Box::new(factory),
				label: label.clone(),
				args: value_args,
			}))
		},

		ExprBody::MmlLiteral(_) => unimplemented!(),

		ExprBody::Labeled { label, inner } => {
			let inner_val = evaluate(inner, vars) ?;

			Ok(ValueBody::Labeled { label: label.clone(), inner: Box::new(inner_val) })
		}
	} ?;
	Ok((body, expr.loc.clone()))
}

fn evaluate_unary_structure<C: Calc + 'static>(
	arg: &Expr,
	vars: &Rc<RefCell<Scope>>,
) -> ModdlResult<ValueBody> {
	let arg_val = evaluate(arg, vars) ?;

	// 定数はコンパイル時に計算する。
	// ただしラベルがついているときは演奏中の設定の対象になるため計算しない
	if arg_val.0.label().is_none() {
		match arg_val.0.as_float() {
			Some(arg_float) => {
				return Ok(ValueBody::Float(C::calc(&vec![arg_float])));
			}
			_ => { } // 下へ
		}
	}

	let (arg_str, _) = arg_val.as_node_structure() ?;
	Ok(ValueBody::NodeStructure(NodeStructure::Calc {
		node_factory: Rc::new(CalcNodeFactory::<C>::new()),
		args: vec![Box::new(arg_str)],
	}))
}

fn evaluate_binary_structure<C: Calc + 'static>(
	lhs: &Expr,
	rhs: &Expr,
	vars: &Rc<RefCell<Scope>>,
) -> ModdlResult<ValueBody> {
	let ref l_val @ (ref l_body, _) = evaluate(lhs, vars) ?;
	let ref r_val @ (ref r_body, _) = evaluate(rhs, vars) ?;

	// 定数はコンパイル時に計算する。
	// ただしラベルがついているときは演奏中の設定の対象になるため計算しない
	if l_body.label().is_none() && r_body.label().is_none() {
		match (l_body.as_float(), r_body.as_float()) {
			(Some(l_float), Some(r_float)) => {
				return Ok(ValueBody::Float(C::calc(&vec![l_float, r_float])));
			}
			_ => { } // 下へ
		}
	}

	let (l_str, _) = l_val.as_node_structure() ?;
	let (r_str, _) = r_val.as_node_structure() ?;
	Ok(ValueBody::NodeStructure(NodeStructure::Calc {
		node_factory: Rc::new(CalcNodeFactory::<C>::new()),
		args: vec![Box::new(l_str), Box::new(r_str)],
	}))
}

fn evaluate_conditional_expr(cond: &Expr, then: &Expr, els: &Expr, vars: &Rc<RefCell<Scope>>) -> ModdlResult<ValueBody> {
	// cond が定数式の場合は短絡評価する。
	// 式全体が定数式になるかどうかは、評価する方の枝の評価結果が定数式になるかどうかに拠る
	let ref cond_val @ (ref cond_body, _) = evaluate(cond, vars) ?;
	if cond_body.label().is_none() {
		if let Some(cond_bool) = cond_body.as_boolean() {
			return if cond_bool {
				evaluate(then, vars).map(|(v, _)| v)
			} else {
				evaluate(els, vars).map(|(v, _)| v)
			};
		}
	}

	// cond が定数式でない場合は NodeStructure として演奏時に評価する。
	// then と else も NodeStructure でなければならないので、定数式にはならない
	let then_val = evaluate(then, vars) ?;
	let else_val = evaluate(els, vars) ?;
	let (cond_str, _) = cond_val.as_node_structure() ?;
	let (then_str, _) = then_val.as_node_structure() ?;
	let (else_str, _) = else_val.as_node_structure() ?;
	Ok(ValueBody::NodeStructure(NodeStructure::Condition {
		cond: Box::new(cond_str),
		then: Box::new(then_str),
		els: Box::new(else_str),
	}))
}

/**
 * 引数名を省略した実引数を、要求された引数リストと照合して解決することで、全ての引数を「引数名」と「式」の対応関係にする。
 * 引数の重複もチェックする（引数名が省略されていても解決してからチェックする）
 */
fn resolve_args<'a>(arg_names: &'a Vec<String>, args: &'a Args, expr_loc: &Location) -> ModdlResult<HashMap<String, &'a Box<Expr>>> {
	let mut result = HashMap::<String, &'a Box<Expr>>::new();
	let mut add = |name: &String, expr: &'a Box<Expr>| -> ModdlResult<()> {
		if result.contains_key(name) { return Err(error(ErrorType::EntryDuplicate { name: name.clone() }, expr_loc.clone())); }

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
