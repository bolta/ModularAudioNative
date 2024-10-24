use parser::common::Location;

/// ビルトイン変数を提供する。今後プラグインの読み込みなどをここでやる想定
use super::{
	error::*, function::*, import::ImportCache, io::*, scope::*, value::*
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
		wave::*, prev::PrevIo,
	},
};

use std::{
	cell::RefCell, collections::hash_map::HashMap, path::Path, rc::Rc
};

pub fn builtin_vars(sample_rate: i32, imports: &mut ImportCache) -> ModdlResult<HashMap<String, Value>> {
	// 組み込み ModDL に失敗することはないはずなので、Location は全て dummy とする

	let builtin_vars = {
		let native_builtins = native_builtins(sample_rate);
		vec![("__Native".to_string(), (ValueBody::Assoc(native_builtins), Location::dummy()))]
				.into_iter().collect()
	};

	let root_moddl = {
		// TODO ちゃんとエラーチェック（失敗の可能性は低そうだが）
		let mut path = std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
		 path.push("builtins");
		 path.push("root.moddl");
		 path
	};

	let path = root_moddl.as_path();
	let root_scope = Scope::root(builtin_vars);
	let (imported, _) = imports.import(path, &root_moddl, root_scope, & Location::dummy()) ?;
	if let ValueBody::Assoc(builtin_vars) = imported {
		Ok(builtin_vars)
	} else {
		// ファイルは必ず存在するので（セットアップ失敗や IO エラーを除き）失敗しないはず
		Err(error(ErrorType::TypeMismatch { expected: ValueType::Assoc }, Location::dummy()))
	}
}

fn native_builtins(sample_rate: i32) -> HashMap<String, Value> {
	let mut result = HashMap::<String, Value>::new();
	// ビルトインは位置を持たない（dummy）
	macro_rules! add_number {
		($name: expr, $value: expr) => {
			result.insert($name.to_string(), (ValueBody::Float($value), Location::dummy()));
		}
	}
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
	macro_rules! add_io {
		($name: expr, $io: expr) => {
			result.insert($name.to_string(), (ValueBody::Io(Rc::new(RefCell::new($io))), Location::dummy()));
		}
	}

	result.insert("false".to_string(), false_value());
	result.insert("true".to_string(), true_value());

	// musical
	add_function!("phase", Phase { });
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

	// numerical
	add_number!("pi", std::f32::consts::PI);
	add_number!("tau", std::f32::consts::TAU);
	add_number!("e", std::f32::consts::E);
	add_function!(Log::name(), Log { });
	add_function!(Log10::name(), Log10 { });
	add_function!(Sin::name(), Sin { });
	add_function!(Cos::name(), Cos { });
	add_function!(Tan::name(), Tan { });
	add_function!(Abs::name(), Abs { });
	add_function!(Signum::name(), Signum { });
	add_function!(Floor::name(), Floor { });
	add_function!(Ceil::name(), Ceil { });
	add_function!(Round::name(), Round { });
	add_function!(Trunc::name(), Trunc { });

	// array
	add_function!("at", At { });
	// concat は flat を使って ModDL で実装する
	// add_function!("concat", Concat { });
	add_function!("flat", Flat { });

	// type
	add_function!("type", Type { });

	// functional
	add_function!("map", Map { });
	add_function!("filter", Filter { });
	add_function!("reduce", Reduce { });

	// io
	add_function!("then", Then { });

	// util
	add_function!("print", Print { });
	add_function!("toString", ToString { });

	// prev
	add_io!("prev", PrevIo::new());

	// import/export
	add_function!("import", Import { });

	add_io!("rand", Rand::new());

	result
}

// TODO 関数の置き場が必要
pub struct Phase { }
impl Function for Phase {
	fn signature(&self) -> FunctionSignature { vec!["initial".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>, call_loc: Location, _imports: &mut ImportCache) -> ModdlResult<Value> {
		let initial = match args.get(& "initial".to_string()) {
			None => 0f32,
			Some((initial_val, initial_loc)) => initial_val.as_float()
					.ok_or_else(|| error(ErrorType::TypeMismatch { expected: ValueType::Number }, initial_loc.clone())) ?,
		};
		let result = Rc::new(PhaseFactory::new(initial));

		Ok((ValueBody::NodeFactory(result), call_loc))
	}
}

pub struct WaveformPlayer { }
impl Function for WaveformPlayer {
	fn signature(&self) -> FunctionSignature { vec!["waveform".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>, call_loc: Location, _imports: &mut ImportCache) -> ModdlResult<Value> {
		let (wave_val, wave_loc) = args.get(& "waveform".to_string())
				.ok_or_else(|| error(ErrorType::ArgMissing { name: "waveform".to_string() }, call_loc.clone())) ?;
		let wave = wave_val.as_waveform_index()
				.ok_or_else(|| error(ErrorType::TypeMismatch { expected: ValueType::Waveform }, wave_loc.clone())) ?;
		let result = Rc::new(WaveformPlayerFactory::new(wave));

		Ok((ValueBody::NodeFactory(result), call_loc))
	}
}

pub struct NesFreq { }
impl Function for NesFreq {
	fn signature(&self) -> FunctionSignature { vec!["triangle".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>, call_loc: Location, _imports: &mut ImportCache) -> ModdlResult<Value> {
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
	fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>, call_loc: Location, _imports: &mut ImportCache) -> ModdlResult<Value> {
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
			fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>, call_loc: Location, _imports: &mut ImportCache) -> ModdlResult<Value> {
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
		impl $name {
			fn name() -> &'static str { <$calc_type>::operator() }
		}
	}
}

unary_math_func!(Log, LogCalc);
unary_math_func!(Log10, Log10Calc);
unary_math_func!(Sin, SinCalc);
unary_math_func!(Cos, CosCalc);
unary_math_func!(Tan, TanCalc);
unary_math_func!(Abs, AbsCalc);
unary_math_func!(Signum, SignumCalc);
unary_math_func!(Floor, FloorCalc);
unary_math_func!(Ceil, CeilCalc);
unary_math_func!(Round, RoundCalc);
unary_math_func!(Trunc, TruncCalc);

// 最低限の配列操作のため、とりあえず map と reduce を作っておく

// TODO [] 演算子にしたい
pub struct At { }
impl Function for At {
	fn signature(&self) -> FunctionSignature { vec!["source".to_string(), "index".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>, call_loc: Location, _imports: &mut ImportCache) -> ModdlResult<Value> {
		let (source, _) = get_required_arg(args, "source", &call_loc)?.as_array() ?;
		let (index, _) = get_required_arg(args, "index", &call_loc)?.as_float() ?;

		source.get(index as usize).map(|elem| elem.clone()).ok_or_else(|| error(ErrorType::IndexOutOfBounds, call_loc.clone()))
	}
}

pub struct Flat { }
impl Function for Flat {
	fn signature(&self) -> FunctionSignature { vec!["arrays".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>, call_loc: Location, _imports: &mut ImportCache) -> ModdlResult<Value> {
		let (arrays, _) = get_required_arg(args, "arrays", &call_loc)?.as_array() ?;

		let mut result = vec![];
		for array in arrays {
			let (array, _) = array.as_array() ?;
			for elem in array {
				result.push(elem.clone());
			}
		}

		Ok((ValueBody::Array(result), call_loc))
	}
}

pub struct Map { }
impl Function for Map {
	fn signature(&self) -> FunctionSignature { vec!["source".to_string(), "mapper".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, vars: &Rc<RefCell<Scope>>, call_loc: Location, imports: &mut ImportCache) -> ModdlResult<Value> {
		let (source, _) = get_required_arg(args, "source", &call_loc)?.as_array() ?;
		let (mapper, mapper_loc) = get_required_arg(args, "mapper", &call_loc)?.as_function() ?;

		let sig = mapper.signature();
		check_arity(&sig, 1, &mapper_loc) ?;

		let mut result = vec![];
		for elem in source {
			result.push(mapper.call(& HashMap::from([(sig[0].clone(), elem.clone())]), vars, mapper_loc.clone(), imports) ?);
		}
		Ok((ValueBody::Array(result), call_loc))
	}
}

pub struct Filter { }
impl Function for Filter {
	fn signature(&self) -> FunctionSignature { vec!["source".to_string(), "predicate".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, vars: &Rc<RefCell<Scope>>, call_loc: Location, imports: &mut ImportCache) -> ModdlResult<Value> {
		let (source, _) = get_required_arg(args, "source", &call_loc)?.as_array() ?;
		let (predicate, predicate_loc) = get_required_arg(args, "predicate", &call_loc)?.as_function() ?;

		let sig = predicate.signature();
		check_arity(&sig, 1, &predicate_loc) ?;

		let mut result = vec![];
		for elem in source {
			let satisfied = predicate.call(& HashMap::from([(sig[0].clone(), elem.clone())]), vars, predicate_loc.clone(), imports)?.as_boolean()?.0;
			if satisfied { result.push(elem.clone()); }
		}
		Ok((ValueBody::Array(result), call_loc))
	}
}

pub struct Reduce { }
impl Function for Reduce {
	fn signature(&self) -> FunctionSignature { vec!["source".to_string(), "initial".to_string(), "reducer".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, vars: &Rc<RefCell<Scope>>, call_loc: Location, imports: &mut ImportCache) -> ModdlResult<Value> {
		let (source, _) = get_required_arg(args, "source", &call_loc)?.as_array() ?;

		let (init, _) = get_required_arg(args, "initial", &call_loc) ?;
		let (reducer, reducer_loc) = get_required_arg(args, "reducer", &call_loc)?.as_function() ?;

		let sig = reducer.signature();
		check_arity(&sig, 2, &reducer_loc) ?;

		let mut result = init.clone();
		for (elem, elem_loc) in source {
			result = reducer.call(& HashMap::from([
				(sig[0].clone(), (result, reducer_loc.clone())), // 位置は便宜的なもの
				(sig[1].clone(), (elem.clone(), elem_loc.clone())),
			]), vars, reducer_loc.clone(), imports)?.0;
		}
		Ok((result, call_loc))
	}
}

pub struct Then { }
impl Function for Then {
	fn signature(&self) -> FunctionSignature { vec!["predecessor".to_string(), "successor".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, vars: &Rc<RefCell<Scope>>, call_loc: Location, _imports: &mut ImportCache) -> ModdlResult<Value> {
		let (predecessor, _) = get_required_arg(args, "predecessor", &call_loc)?.as_io() ?;
		let (successor, _) = get_required_arg(args, "successor", &call_loc)?.as_function() ?;

		Ok((ValueBody::Io(Rc::new(RefCell::new(ThenIo::new(predecessor.clone(), successor.clone(), vars.clone())))), call_loc))
	}
}

pub struct Import { }
impl Function for Import {
	fn signature(&self) -> FunctionSignature { vec!["path".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, vars: &Rc<RefCell<Scope>>, call_loc: Location, imports: &mut ImportCache) -> ModdlResult<Value> {
		let (path, _) = get_required_arg(args, "path", &call_loc)?.as_string() ?;

		let root_scope = vars.borrow().get_root()
				.ok_or_else(|| error(ErrorType::UnknownError { message: "cannot get root scope".to_string() }, call_loc.clone())) ?;

		imports.import(Path::new(&path), call_loc.path.as_path(), root_scope, &call_loc)
	}
}

pub struct Type { }
impl Function for Type {
	fn signature(&self) -> FunctionSignature { vec!["arg".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>, call_loc: Location, _imports: &mut ImportCache) -> ModdlResult<Value> {
		let (arg, _) = get_required_arg(args, "arg", &call_loc)?;

		let type_id = match arg {
			ValueBody::Float(_) => "Number",
			ValueBody::WaveformIndex(_) => "Waveform",
			ValueBody::TrackSet(_) => "TrackSet",
			ValueBody::IdentifierLiteral(_) => "QuotedIdentifier",
			ValueBody::String(_) => "String",
			ValueBody::Array(_) => "Array",
			ValueBody::Assoc(_) => "Assoc",
			ValueBody::NodeStructure(_) => "NodeStructure",
			ValueBody::NodeFactory(_) => "NodeFactory",
			ValueBody::Function(_) => "Function",
			ValueBody::Io(_) => "Io",
		};

		Ok((ValueBody::String(type_id.to_string()), call_loc))
	}
}

pub struct Print { }
impl Function for Print {
	fn signature(&self) -> FunctionSignature { vec!["value".to_string(), "text".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>, call_loc: Location, _imports: &mut ImportCache) -> ModdlResult<Value> {
		let (value, _) = get_required_arg(args, "value", &call_loc)?;
		let text = get_optional_arg(args, "text").map(|v| &v.0);
		
		let print_value = |v: &ValueBody| v.to_str(|s| println!("{}", s));

		match text {
			Some(text) => print_value(text),
			None => print_value(value),
		}

		Ok((value.clone(), call_loc))
	}
}

pub struct ToString { }
impl Function for ToString {
	fn signature(&self) -> FunctionSignature { vec!["value".to_string()] }
	fn call(&self, args: &HashMap<String, Value>, _vars: &Rc<RefCell<Scope>>, call_loc: Location, _imports: &mut ImportCache) -> ModdlResult<Value> {
		let (value, _) = get_required_arg(args, "value", &call_loc)?;

		let result = ValueBody::String(value.to_str(|s| s.to_string()));
		Ok((result, call_loc.clone()))
	}
}

// pub struct MapLabels { }
// impl Function for MapLabels {
// 	fn signature(&self) -> FunctionSignature { vec!["struct".to_string(), "mapper".to_string()] }
// 	fn call(&self, args: &HashMap<String, Value>, vars: &Rc<RefCell<Scope>>, call_loc: Location, _imports: &mut ImportCache) -> ModdlResult<Value> {
		
// 	}
// }

