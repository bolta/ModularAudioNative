use parser::common::Location;

use super::{
	error::*,
	scope::*,
	value::*,
};

use std::{
	cell::RefCell,
	collections::HashMap,
	rc::Rc,
};

pub type FunctionSignature = Vec<String>;

pub trait Function {
	fn signature(&self) -> FunctionSignature; // 将来的に型情報も必要になるかもだが、とりあえず名前と数だけ
	fn call(&self, args: &HashMap<String, Value>, vars: &Rc<RefCell<Scope>>, loc: Location) -> ModdlResult<Value>;
	// TODO 副作用が必要な場合もあるので引数はもっと増える
}

// for experiments
pub struct Twice { }
impl Function for Twice {
	fn signature(&self) -> FunctionSignature { vec!["arg0".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>, call_loc: Location) -> ModdlResult<Value> {
		const ARG_NAME: &str = "arg0"; // TODO こういうどうでもいい名前でもつけないとだめか？
		let (arg_val, arg_loc) = args.get(& ARG_NAME.to_string())
				.ok_or_else(|| error(ErrorType::ArgMissing { name: ARG_NAME.to_string() }, call_loc.clone())) ?;
		let arg = arg_val.as_float().ok_or_else(|| error(ErrorType::TypeMismatch, arg_loc.clone())) ?;
		let result = arg * 2f32;

		Ok((ValueBody::Float(result), call_loc))
	}
}
