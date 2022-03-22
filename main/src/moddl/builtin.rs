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
		envelope::*,
		filter::*,
		freq::*,
		lofi::*,
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

	result.insert("false".to_string(), VALUE_FALSE);
	result.insert("true".to_string(), VALUE_TRUE);

	add_node_factory!("sineOsc", SineOscFactory { });
	add_node_factory!("triangleOsc", TriangleOscFactory { });
	add_node_factory!("sawOsc", SawOscFactory { });
	add_node_factory!("pulseOsc", PulseOscFactory { });
	add_node_factory!("expEnv", ExpEnvFactory { });
	add_node_factory!("adsrEnv", AdsrEnvFactory { });
	add_node_factory!("limit", LimitFactory { });
	add_node_factory!("lpf", LowPassFilterFactory { });
	add_node_factory!("hpf", HighPassFilterFactory { });
	add_node_factory!("bpf", BandPassFilterFactory { });
	add_node_factory!("quantCrush", QuantCrushFactory { });
	add_node_factory!("pan", PanFactory { });
	add_function!("waveformPlayer", WaveformPlayer { });
	add_function!("nesFreq", NesFreq { });

	// for experiments
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

pub struct NesFreq { }
impl Function for NesFreq {
	fn call(&self, args: &HashMap<String, Value>) -> ModdlResult<Value> {
		let triangle_val = args.get(& "triangle".to_string()).unwrap_or(&VALUE_FALSE);
		let triangle = triangle_val.as_boolean().ok_or_else(|| Error::TypeMismatch) ?;
		let result = Rc::new(NesFreqFactory::new(triangle));

		Ok(Value::NodeFactory(result))
	}
}
