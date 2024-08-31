use parser::common::Location;

use crate::wave::waveform_host::WaveformIndex;

use super::{
	error::*, import::ImportCache, scope::*, value::*
};

use std::{
	cell::RefCell,
	collections::HashMap,
	rc::Rc,
};

pub type FunctionSignature = Vec<String>;

pub trait Function {
	fn signature(&self) -> FunctionSignature; // 将来的に型情報も必要になるかもだが、とりあえず名前と数だけ
	fn call(&self, args: &HashMap<String, Value>, vars: &Rc<RefCell<Scope>>, call_loc: Location, _imports: &mut ImportCache) -> ModdlResult<Value>;
	// TODO 副作用が必要な場合もあるので引数はもっと増える
}

pub fn check_arity(sig: &FunctionSignature, expected: usize, loc: &Location) -> ModdlResult<()> {
	let actual = sig.len();
	if actual == expected {
		Ok(())
	} else {
		Err(error(ErrorType::ArityMismatch { expected, actual }, loc.clone()))
	}
}

// for experiments
pub struct Twice { }
impl Function for Twice {
	fn signature(&self) -> FunctionSignature { vec!["arg0".to_string()] } // TODO こういうどうでもいい名前でもつけないとだめか？
	fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>, call_loc: Location, _imports: &mut ImportCache) -> ModdlResult<Value> {
		let (arg, _) = get_required_arg(args, "arg0", &call_loc)?.as_float() ?;
		let result = arg * 2f32;

		Ok((ValueBody::Float(result), call_loc))
	}
}

pub fn get_required_arg<'a>(args: &'a HashMap<String, Value>, name: &str, call_loc: &Location) -> ModdlResult<&'a Value> {
	get_optional_arg(args, name)
			.ok_or_else(|| error(ErrorType::ArgMissing { name: name.to_string() }, call_loc.clone()))
}
pub fn get_optional_arg<'a>(args: &'a HashMap<String, Value>, name: &str) -> Option<&'a Value> {
	args.get(& name.to_string())
}
