use parser::common::Location;

use super::error::{ModdlResult, error, ErrorType};
use super::io::Io;
use super::{
	function::*,
};
use crate::{
	core::node::*,
	core::node_factory::*,
	wave::waveform_host::WaveformIndex,
};
use enum_display::EnumDisplay;
use std::cell::RefCell;
use std::{
	collections::HashMap,
	rc::Rc,
};

// TODO 仮置き
use crate::calc::*;
use crate::core::common::*;
use crate::core::node::Node;
use crate::node::arith::*;
use std::marker::PhantomData;
pub trait CalcNodeFactoryTrait {
	fn create_mono(&self, base: NodeBase, args: Vec<MonoNodeIndex>) -> Box<dyn Node>;
	fn create_stereo(&self, base: NodeBase, args: Vec<StereoNodeIndex>) -> Box<dyn Node>;
}
// #[derive(Clone)]
pub struct CalcNodeFactory<C: 'static + Calc> {
	_c: PhantomData<fn () -> C>,
}
impl <C: 'static + Calc> CalcNodeFactory<C> {
	pub fn new() -> Self { Self { _c: PhantomData } }
}
impl <C: 'static + Calc> CalcNodeFactoryTrait for CalcNodeFactory<C> {
	fn create_mono(&self, base: NodeBase, args: Vec<MonoNodeIndex>) -> Box<dyn Node> {
		Box::new(MonoCalc::<C>::new(base, args))
	}
	fn create_stereo(&self, base: NodeBase, args: Vec<StereoNodeIndex>) -> Box<dyn Node> {
		Box::new(StereoCalc::<C>::new(base, args))
	}
}

/// 生成すべき Node の構造を表現する型。
/// Value から直接 Node を生成すると問題が多いので、一旦この形式を挟む
#[derive(Clone)]
pub enum NodeStructure {
	Calc{ node_factory: Rc<dyn CalcNodeFactoryTrait>, args: Vec<Box<NodeStructure>> },
	Connect(Box<NodeStructure>, Box<NodeStructure>),
	Condition { cond: Box<NodeStructure>, then: Box<NodeStructure>, els: Box<NodeStructure> },
	Lambda { input_param: String, body: Box<NodeStructure> },
	NodeCreation {
		factory: Rc<dyn NodeFactory>,
		args: HashMap<String, Value>,
		label: Option<String>,
	},
	Constant {
		value: f32,
		label: Option<String>,
	},
	Placeholder { name: String },
}
impl NodeStructure {
	pub fn label(&self) -> Option<String> {
		match self {
			NodeStructure::NodeCreation { label, .. } | NodeStructure::Constant { label, .. } => label.clone(),
			_ => None,
		}
	}
}

pub type Value = (ValueBody, Location);

pub trait ValueExtraction {
	fn as_float(&self) -> ModdlResult<(f32, Location)>;
	fn as_boolean(&self) -> ModdlResult<(bool, Location)>;
	fn as_waveform_index(&self) -> ModdlResult<(WaveformIndex, Location)>;
	fn as_track_set(&self) -> ModdlResult<(Vec<String>, Location)>;
	fn as_identifier_literal(&self) -> ModdlResult<(String, Location)>;
	fn as_string(&self) -> ModdlResult<(String, Location)>;
	fn as_array(&self) -> ModdlResult<(&Vec<Value>, Location)>;
	fn as_assoc(&self) -> ModdlResult<(&HashMap<String, Value>, Location)>;
	fn as_node_structure(&self) -> ModdlResult<(NodeStructure, Location)>;
	fn as_node_factory(&self) -> ModdlResult<(Rc<dyn NodeFactory>, Location)>;
	fn as_function(&self) -> ModdlResult<(Rc<dyn Function>, Location)>;
	fn as_io(&self) -> ModdlResult<(Rc<RefCell<dyn Io>>, Location)>;
}
fn extract<T>(val: Option<T>, loc: &Location, expected: ValueType) -> ModdlResult<(T, Location)> {
	match val {
		Some(val) => Ok((val, loc.clone())),
		None => Err(error(ErrorType::TypeMismatch { expected }, loc.clone())),
	}
}
fn extract_any<T>(val: Option<T>, loc: &Location, expected: Vec<ValueType>) -> ModdlResult<(T, Location)> {
	match val {
		Some(val) => Ok((val, loc.clone())),
		None => Err(error(ErrorType::TypeMismatchAny { expected }, loc.clone())),
	}
}
impl ValueExtraction for Value {
	fn as_float(&self) -> ModdlResult<(f32, Location)> { extract(self.0.as_float(), &self.1, ValueType::Number) }
	fn as_boolean(&self) -> ModdlResult<(bool, Location)> { extract(self.0.as_boolean() , &self.1, ValueType::Number) }
	fn as_waveform_index(&self) -> ModdlResult<(WaveformIndex, Location)> { extract(self.0.as_waveform_index() , &self.1, ValueType::Waveform) }
	fn as_track_set(&self) -> ModdlResult<(Vec<String>, Location)> { extract(self.0.as_track_set() , &self.1, ValueType::TrackSet) }
	fn as_identifier_literal(&self) -> ModdlResult<(String, Location)> { extract(self.0.as_identifier_literal() , &self.1, ValueType::QuotedIdentifier) }
	fn as_string(&self) -> ModdlResult<(String, Location)> { extract(self.0.as_string() , &self.1, ValueType::String) }
	fn as_array(&self) -> ModdlResult<(&Vec<Value>, Location)> { extract(self.0.as_array() , &self.1, ValueType::Array) }
	fn as_assoc(&self) -> ModdlResult<(&HashMap<String, Value>, Location)> { extract(self.0.as_assoc() , &self.1, ValueType::Assoc) }
	fn as_node_structure(&self) -> ModdlResult<(NodeStructure, Location)> { extract_any(self.0.as_node_structure() , &self.1,
			vec![ValueType::NodeStructure, ValueType::Number, ValueType::NodeFactory]) }
	fn as_node_factory(&self) -> ModdlResult<(Rc<dyn NodeFactory>, Location)> { extract(self.0.as_node_factory() , &self.1, ValueType::NodeFactory) }
	fn as_function(&self) -> ModdlResult<(Rc<dyn Function>, Location)> { extract(self.0.as_function() , &self.1, ValueType::Function) }
	fn as_io(&self) -> ModdlResult<(Rc<RefCell<dyn Io>>, Location)> { extract(self.0.as_io() , &self.1, ValueType::Io) }
}

#[derive(Clone)]
pub enum ValueBody {
	Float(f32),
	WaveformIndex(WaveformIndex),
	TrackSet(Vec<String>),
	IdentifierLiteral(String),
	String(String),
	Array(Vec<Value>),
	Assoc(HashMap<String, Value>),
	/// ノードの構造に関するツリー表現
	NodeStructure(NodeStructure),
	/// 引数を受け取ってノードを生成する関数
	NodeFactory(Rc<dyn NodeFactory>),
	Function(Rc<dyn Function>),
	Io(Rc<RefCell<dyn Io>>),
}

impl ValueBody {
	pub fn as_float(&self) -> Option<f32> {
		match self {
			Self::Float(value) => Some(*value),
			_ => None,
		}
	}
	pub fn as_boolean(&self) -> Option<bool> {
		self.as_float().map(|v| v > 0f32)
	}
	pub fn as_waveform_index(&self) -> Option<WaveformIndex> {
		match self {
			Self::WaveformIndex(value) => Some(*value),
			_ => None,
		}
	}
	pub fn as_track_set(&self) -> Option<Vec<String>> {
		match self {
			Self::TrackSet(tracks) => Some(tracks.clone()),
			_ => None,
		}
	}
	pub fn as_identifier_literal(&self) -> Option<String> {
		match self {
			Self::IdentifierLiteral(id) => Some(id.clone()),
			_ => None,
		}
	}

	pub fn as_string(&self) -> Option<String> {
		match self {
			Self::String(content) => Some(content.clone()),
			_ => None,
		}
	}

	pub fn as_array(&self) -> Option<&Vec<Value>> {
		match self {
			Self::Array(content) => Some(content),
			_ => None,
		}
	}

	pub fn as_assoc(&self) -> Option<&HashMap<String, Value>> {
		match self {
			Self::Assoc(content) => Some(content),
			_ => None,
		}
	}

	pub fn as_node_structure(&self) -> Option<NodeStructure> {
		// Value から直接 Node に変換しようとすると NodeHost が必要になったり、
		// Node をタグ付きで生成したいときに困ったりとよろしくないことが多いので、
		// Node への変換は提供しない。
		// 代わりに、Node の一歩手前というか、ノードグラフの設計図となる NodeStructure を提供し、
		// そこから Node を生成するのは然るべき場所（Player）でいいようにやってもらうこととする。
		// 数値や変数参照から Node への暗黙の変換もここで提供する
		match self {
			Self::NodeStructure(str) => Some(str.clone()),
			Self::Float(value) => Some(NodeStructure::Constant { value: *value, label: None }),
			Self::NodeFactory(fact) => Some(NodeStructure::NodeCreation {
				factory: fact.clone(),
				args: HashMap::new(),
				label: None,
			}),
			_ => None,
		}
	}
	pub fn as_node_factory(&self) -> Option<Rc<dyn NodeFactory>> {
		match self {
			Self::NodeFactory(fact) => Some(fact.clone()),
			_ => None,
		}
	}

	pub fn as_function(&self) -> Option<Rc<dyn Function>> {
		match self {
			Self::Function(func) => Some(func.clone()),
			_ => None,
		}
	}

	pub fn as_io(&self) -> Option<Rc<RefCell<dyn Io>>> {
		match self {
			Self::Io(io) => Some(io.clone()),
			_ => None,
		}
	}

	pub fn label(&self) -> Option<String> {
		match self {
			Self::NodeStructure(strukt) => strukt.label(),
			_ => None,
		}
	}
}

#[derive(Copy, Clone, Debug, EnumDisplay)]
pub enum ValueType {
	Number,
	Waveform,
	TrackSet,
	QuotedIdentifier,
	String,
	Array,
	Assoc,
	NodeStructure,
	NodeFactory,
	Function,
	Io,
}

// 当面 boolean 型は設けず、正を truthy、0 と負を falsy として扱う。
// 代表の値として true = 1、false = -1 とする
pub fn false_value() -> Value { (ValueBody::Float(-1f32), Location::dummy()) }
pub fn true_value() -> Value { (ValueBody::Float(1f32), Location::dummy()) }
