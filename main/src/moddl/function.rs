use super::{
	error::*,
	value::*,
};
use crate::{
	core::node_factory::*,
};

use std::collections::HashMap;
use std::rc::Rc;

pub trait Function {
	fn call(&self, args: &HashMap<String, Value>) -> ModdlResult<Value>;
	// TODO 副作用が必要な場合もあるので引数はもっと増える
	// TODO シグネチャの公開も必要？
}

// for experiments
pub struct Twice { }
impl Function for Twice {
	fn call(&self, args: &HashMap<String, Value>) -> ModdlResult<Value> {
		const ARG_NAME: &str = "arg0"; // TODO こういうどうでもいい名前でもつけないとだめか？
		let arg_val = args.get(& ARG_NAME.to_string()).ok_or_else(|| Error::TypeMismatch) ?;
		let arg = arg_val.as_float().ok_or_else(|| Error::TypeMismatch) ?;
		let result = arg * 2f32;

		Ok(Value::Float(result))
	}
}

pub struct WaveformPlayer { }
impl Function for WaveformPlayer {
	fn call(&self, args: &HashMap<String, Value>) -> ModdlResult<Value> {
		let wave_val = args.get(& "waveform".to_string()).ok_or_else(|| Error::TypeMismatch) ?;
		let wave = wave_val.as_waveform_index().ok_or_else(|| Error::TypeMismatch) ?;
		let result = Rc::new(WaveformPlayerFactory::new(wave));

		Ok(Value::NodeFactory(result))
	}
}
