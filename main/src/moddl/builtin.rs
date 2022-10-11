/// ビルトイン変数を提供する。今後プラグインの読み込みなどをここでやる想定
use super::{
	function::*,
	scope::*,
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
		arith::*,
		envelope::*,
		delay::*,
		filter::*,
		freq::*,
		lofi::*,
		noise::*,
		osc::*,
		transit::*,
		wave::*,
	},
};

use std::{
	cell::RefCell,
	collections::hash_map::HashMap,
	rc::Rc,
};

pub fn builtin_vars(sample_rate: i32) -> HashMap<String, Value> {
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
	add_node_factory!("uniformNoise", UniformNoiseFactory { });
	add_node_factory!("expEnv", ExpEnvFactory { });
	add_node_factory!("adsrEnv", AdsrEnvFactory { });
	add_node_factory!("limit", LimitFactory { });
	add_node_factory!("lpf", LowPassFilterFactory { });
	add_node_factory!("hpf", HighPassFilterFactory { });
	add_node_factory!("bpf", BandPassFilterFactory { });
	add_node_factory!("quantCrush", QuantCrushFactory { });
	add_node_factory!("sampleCrush", SampleCrushFactory::new(sample_rate));
	add_node_factory!("pan", PanFactory { });
	add_node_factory!("glide", GlideFactory { });
	add_function!("waveformPlayer", WaveformPlayer { });
	add_function!("nesFreq", NesFreq { });
	add_function!("delay", Delay::new(sample_rate));

	add_function!("log", Log { });
	add_function!("log10", Log10 { });
	add_function!("sin", Sin { });
	add_function!("cos", Cos { });
	add_function!("tan", Tan { });

	// for experiments
	add_node_factory!("stereoTestOsc", StereoTestOscFactory { });
	add_function!("twice", Twice { });

	result
}

// TODO 関数の置き場が必要

pub struct WaveformPlayer { }
impl Function for WaveformPlayer {
	fn signature(&self) -> FunctionSignature { vec!["waveform".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>) -> ModdlResult<Value> {
		let wave_val = args.get(& "waveform".to_string()).ok_or_else(|| Error::TypeMismatch) ?;
		let wave = wave_val.as_waveform_index().ok_or_else(|| Error::TypeMismatch) ?;
		let result = Rc::new(WaveformPlayerFactory::new(wave));

		Ok(Value::NodeFactory(result))
	}
}

pub struct NesFreq { }
impl Function for NesFreq {
	fn signature(&self) -> FunctionSignature { vec!["triangle".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>) -> ModdlResult<Value> {
		let triangle_val = args.get(& "triangle".to_string()).unwrap_or(&VALUE_FALSE);
		let triangle = triangle_val.as_boolean().ok_or_else(|| Error::TypeMismatch) ?;
		let result = Rc::new(NesFreqFactory::new(triangle));

		Ok(Value::NodeFactory(result))
	}
}

pub struct Delay {
	sample_rate: i32,
}
impl Delay {
	pub fn new(sample_rate: i32) -> Self { Self { sample_rate } }
}
impl Function for Delay {
	fn signature(&self) -> FunctionSignature { vec!["max_time".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>) -> ModdlResult<Value> {
		let max_time_val = args.get(& "max_time".to_string()).ok_or_else(|| Error::ArgMissing { name: "max_time".to_string() }) ?;
		let max_time = max_time_val.as_float().ok_or_else(|| Error::TypeMismatch) ?;
		let result = Rc::new(DelayFactory::new(max_time, self.sample_rate));

		Ok(Value::NodeFactory(result))
	}
}


use crate::calc::*;
macro_rules! unary_math_func {
	($name: ident, $calc_type: ty) => {
		pub struct $name { }
		impl Function for $name {
			fn signature(&self) -> FunctionSignature { vec!["arg".to_string()] }
			fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>) -> ModdlResult<Value> {
				let arg = args.get(& "arg".to_string()).ok_or_else(|| Error::TypeMismatch) ?;
				if let Some(val) = arg.as_float() {
					Ok(Value::Float(<$calc_type>::calc(&vec![val])))
		
				} else if let Some(val) = arg.as_node_structure() {
					Ok(Value::NodeStructure(NodeStructure::Calc {
						node_factory: Rc::new(CalcNodeFactory::<$calc_type>::new()),
						args: vec![Box::new(val)],
					}))
		
				} else {
					Err(Error::TypeMismatch)
				}
			}
		}
	}
}

unary_math_func!(Log, LogCalc);
unary_math_func!(Log10, Log10Calc);
unary_math_func!(Sin, SinCalc);
unary_math_func!(Cos, CosCalc);
unary_math_func!(Tan, TanCalc);
