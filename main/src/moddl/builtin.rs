/// ビルトイン変数を提供する。今後プラグインの読み込みなどをここでやる想定
use super::{
	function::*,
	value::*,
};
use crate::{
	core::{
		node_factory::*,
	},
	moddl::{
		error::*,
	},
	node::{
		osc::*,
		arith::*,
		wave::*,
	},
};

use std::{
	collections::hash_map::HashMap,
	rc::Rc,
};

pub fn builtin_vars() -> HashMap<String, Value> {
	let mut result = HashMap::<String, Value>::new();
	macro_rules! add_node_factory {
		($name: expr, $fact: expr) => {
			result.insert($name.to_string(), Value::NodeFactory(Rc::new($fact)));
		}
	}
	macro_rules! add_function {
		($name: expr, $fact: expr) => {
			result.insert($name.to_string(), Value::Function(Rc::new($fact)));
		}
	}

	add_node_factory!("sineOsc", SineOscFactory { });
	add_node_factory!("pulseOsc", PulseOscFactory { });
	add_node_factory!("limit", LimitFactory { });
	add_node_factory!("pan", PanFactory { });
	add_function!("waveformPlayer", WaveformPlayer { });

	// for experiments
	add_node_factory!("env1", Env1Factory { });
	add_node_factory!("stereoTestOsc", StereoTestOscFactory { });
	add_function!("twice", Twice { });

	result
}

// TODO 関数の置き場が必要
pub struct WaveformPlayer { }
impl Function for WaveformPlayer {
	fn call(&self, args: &HashMap<String, Value>) -> ModdlResult<Value> {
		let wave_val = args.get(& "waveform".to_string()).ok_or_else(|| Error::TypeMismatch) ?;
		let wave = wave_val.as_waveform_index().ok_or_else(|| Error::TypeMismatch) ?;
		let result = Rc::new(WaveformPlayerFactory::new(wave));

		Ok(Value::NodeFactory(result))
	}
}
