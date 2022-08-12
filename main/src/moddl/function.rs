use super::{
	error::*,
	value::*,
};

use std::collections::HashMap;

pub type FunctionSignature = Vec<&'static str>;

pub trait Function {
	fn signature(&self) -> FunctionSignature; // 将来的に型情報も必要になるかもだが、とりあえず名前と数だけ
	fn call(&self, args: &HashMap<String, Value>) -> ModdlResult<Value>;
	// TODO 副作用が必要な場合もあるので引数はもっと増える
}

// for experiments
pub struct Twice { }
impl Function for Twice {
	fn signature(&self) -> FunctionSignature { vec!["arg0"] }
	fn call(&self, args: &HashMap<String, Value>) -> ModdlResult<Value> {
		const ARG_NAME: &str = "arg0"; // TODO こういうどうでもいい名前でもつけないとだめか？
		let arg_val = args.get(& ARG_NAME.to_string()).ok_or_else(|| Error::TypeMismatch) ?;
		let arg = arg_val.as_float().ok_or_else(|| Error::TypeMismatch) ?;
		let result = arg * 2f32;

		Ok(Value::Float(result))
	}
}
