use super::{
	error::*,
	evaluator::*,
	function::*,
	scope::*,
	value::*,
};

// extern crate parser;
use parser::{moddl::ast::*, common::Location};

use std::{
	cell::RefCell,
	collections::HashMap,
	rc::Rc,
};

pub struct Param {
	pub name: String,
	pub default: Option<Value>,
}

/// 式によって記述された関数
pub struct LambdaFunction {
	params: Vec<Param>,
	body: Expr,
	vars: Rc<RefCell<Scope>>,
}
impl LambdaFunction {
	pub fn new(params: Vec<Param>, body: Expr, vars: &Rc<RefCell<Scope>>) -> Self {
		Self { params, body, vars: vars.clone() }
	}
}
impl Function for LambdaFunction {
	fn signature(&self) -> FunctionSignature { self.params.iter().map(|param| param.name.clone()).collect() }
	fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>, call_loc: Location) -> ModdlResult<Value> {
		// 引数のスコープを追加
		let mut child_vars = Scope::child_of(self.vars.clone());
		self.params.iter().try_for_each(|param| {
			let value = args.get(&param.name).or(param.default.as_ref())
					.ok_or_else(|| error(ErrorType::ArgMissing { name: param.name.clone() }, call_loc.clone())) ?;
			child_vars.borrow_mut().set(&param.name, value.clone()) ?;
			ModdlResult::Ok(())
		}) ?;

		evaluate(&self.body, &mut child_vars)
	}
}
