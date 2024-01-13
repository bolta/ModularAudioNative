use parser::common::Location;

/// ビルトイン変数を提供する。今後プラグインの読み込みなどをここでやる想定
use super::{
	function::*,
	scope::*,
	value::*, error::{error, ErrorType, ModdlResult},
};
use crate::{
	core::{
		node_factory::*,
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
	// ビルトインは位置を持たない（dummy）
	macro_rules! add_node_factory {
		($name: expr, $fact: expr) => {
			result.insert($name.to_string(), (ValueBody::NodeFactory(Rc::new($fact)), Location::dummy()));
		}
	}
	macro_rules! add_function {
		($name: expr, $fact: expr) => {
			result.insert($name.to_string(), (ValueBody::Function(Rc::new($fact)), Location::dummy()));
		}
	}

	result.insert("false".to_string(), false_value());
	result.insert("true".to_string(), true_value());

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

	add_function!("map", Map { });
	add_function!("reduce", Reduce { });

	// for experiments
	add_node_factory!("stereoTestOsc", StereoTestOscFactory { });
	add_function!("twice", Twice { });

	result
}

// TODO 関数の置き場が必要

pub struct WaveformPlayer { }
impl Function for WaveformPlayer {
	fn signature(&self) -> FunctionSignature { vec!["waveform".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>, call_loc: Location) -> ModdlResult<Value> {
		let (wave_val, wave_loc) = args.get(& "waveform".to_string())
				.ok_or_else(|| error(ErrorType::ArgMissing { name: "waveform".to_string() }, Location::dummy())) ?;
		let wave = wave_val.as_waveform_index()
				.ok_or_else(|| error(ErrorType::TypeMismatch { expected: ValueType::Waveform }, wave_loc.clone())) ?;
		let result = Rc::new(WaveformPlayerFactory::new(wave));

		Ok((ValueBody::NodeFactory(result), call_loc))
	}
}

pub struct NesFreq { }
impl Function for NesFreq {
	fn signature(&self) -> FunctionSignature { vec!["triangle".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>, call_loc: Location) -> ModdlResult<Value> {
		let triangle = match args.get(& "triangle".to_string()) {
			Some(v) => v.as_boolean()?.0,
			None => false,
		};
		let result = Rc::new(NesFreqFactory::new(triangle));

		Ok((ValueBody::NodeFactory(result), call_loc))
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
	fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>, call_loc: Location) -> ModdlResult<Value> {
		let (max_time_val, max_time_loc) = args.get(& "max_time".to_string()).ok_or_else(|| error(ErrorType::ArgMissing { name: "max_time".to_string() }, Location::dummy())) ?;
		let max_time = max_time_val.as_float()
				.ok_or_else(|| error(ErrorType::TypeMismatch { expected: ValueType::Number }, max_time_loc.clone())) ?;
		let result = Rc::new(DelayFactory::new(max_time, self.sample_rate));

		Ok((ValueBody::NodeFactory(result), call_loc))
	}
}


use crate::calc::*;
macro_rules! unary_math_func {
	($name: ident, $calc_type: ty) => {
		pub struct $name { }
		impl Function for $name {
			fn signature(&self) -> FunctionSignature { vec!["arg".to_string()] }
			fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>, call_loc: Location) -> ModdlResult<Value> {
				let (arg, arg_loc) = args.get(& "arg".to_string())
						.ok_or_else(|| error(ErrorType::ArgMissing { name: "arg".to_string() }, call_loc.clone())) ?;
				if let Some(val) = arg.as_float() {
					Ok((ValueBody::Float(<$calc_type>::calc(&vec![val])), call_loc))
		
				} else if let Some(val) = arg.as_node_structure() {
					Ok((ValueBody::NodeStructure(NodeStructure::Calc {
						node_factory: Rc::new(CalcNodeFactory::<$calc_type>::new()),
						args: vec![Box::new(val)],
					}), call_loc))
		
				} else {
					Err(error(ErrorType::TypeMismatchAny {
						expected: vec![ValueType::Number, ValueType::NodeStructure],
					}, arg_loc.clone()))
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

// 最低限の配列操作のため、とりあえず map と reduce を作っておく

pub struct Map { }
impl Function for Map {
	fn signature(&self) -> FunctionSignature { vec!["source".to_string(), "mapper".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, vars: &Rc<RefCell<Scope>>, call_loc: Location) -> ModdlResult<Value> {
		let (source, _) = get_required_arg(args, "source", &call_loc)?.as_array() ?;
		let (mapper, mapper_loc) = get_required_arg(args, "mapper", &call_loc)?.as_function() ?;

		let sig = mapper.signature();
		if sig.len() != 1 { return Err(error(ErrorType::SignatureMismatch, Location::dummy())); }

		let mut result = vec![];
		for elem in source {
			result.push(mapper.call(& HashMap::from([(sig[0].clone(), elem.clone())]), vars, mapper_loc.clone()) ?);
		}
		Ok((ValueBody::Array(result), call_loc))
	}
}

pub struct Reduce { }
impl Function for Reduce {
	fn signature(&self) -> FunctionSignature { vec!["source".to_string(), "initial".to_string(), "reducer".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, vars: &Rc<RefCell<Scope>>, call_loc: Location) -> ModdlResult<Value> {
		let (source, _) = get_required_arg(args, "source", &call_loc)?.as_array() ?;

		let (init, _) = get_required_arg(args, "initial", &call_loc) ?;
		let (reducer, reducer_loc) = get_required_arg(args, "reducer", &call_loc)?.as_function() ?;

		let sig = reducer.signature();
		if sig.len() != 2 { return Err(error(ErrorType::SignatureMismatch, call_loc)); }

		let mut result = init.clone();
		for (elem, elem_loc) in source {
			result = reducer.call(& HashMap::from([
				(sig[0].clone(), (result, reducer_loc.clone())), // 位置は便宜的なもの
				(sig[1].clone(), (elem.clone(), elem_loc.clone())),
			]), vars, reducer_loc.clone())?.0;
		}
		Ok((result, call_loc))
	}
}
