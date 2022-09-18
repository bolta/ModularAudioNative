use super::{
	error::*,
	evaluator::*,
	function::*,
	value::*,
};

// extern crate parser;
use parser::moddl::ast::*;

use std::collections::HashMap;

pub struct Param {
	pub name: String,
	pub default: Option<Value>,
}

pub struct LambdaFunction {
	params: Vec<Param>,
	body: Expr,
	// vars: VarStack,
}
impl LambdaFunction {
	pub fn new(params: Vec<Param>, body: Expr/* , vars: VarStack */) -> Self {
		Self { params, body/* , vars */ }
	}
}
impl Function for LambdaFunction {
	fn signature(&self) -> FunctionSignature { self.params.iter().map(|param| param.name.clone()).collect() }
	fn call(&self, args: &HashMap<String, Value>, vars: &VarStack) -> ModdlResult<Value> {
		let mut vars = vars.clone();
		vars.push_clone();
		self.params.iter().try_for_each(|param| {
dbg!(&param.name);
			let value = args.get(&param.name).or(param.default.as_ref()).ok_or_else(|| Error::ArgMissing { name: param.name.clone() }) ?;
			vars.top_mut().insert(param.name.clone(), value.clone());
			ModdlResult::Ok(())
		}) ?;
dbg!(vars.top().keys());
		evaluate(&self.body, &mut vars)
	}
}
