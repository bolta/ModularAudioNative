use super::{
	function::*,
};
use crate::{
	core::node::*,
	core::node_factory::*,
	wave::waveform_host::WaveformIndex,
};

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
	NodeWithArgs {
		factory: Box<NodeStructure>,
		label: String,
		args: HashMap<String, Value>,
	},
	NodeFactory(Rc<dyn NodeFactory>),
	// Constant(f32),
	Constant {
		value: f32,
		label: Option<String>,
	},
	Placeholder { name: String },
}

#[derive(Clone)]
pub enum Value {
	Float(f32),
	WaveformIndex(WaveformIndex),
	TrackSet(Vec<String>),
	IdentifierLiteral(String),
	String(String),
	Array(Vec<Value>),
	Assoc(HashMap<String, Value>),
	// Node(NodeIndex),
	/// ノードの構造に関するツリー表現
	NodeStructure(NodeStructure),
	/// 引数を受け取ってノードを生成する関数
	NodeFactory(Rc<dyn NodeFactory>),
	Function(Rc<dyn Function>),
	Labeled {
		label: String,
		inner: Box<Value>,
	},
}

impl Value {
	pub fn as_float(&self) -> Option<f32> {
		match self.value() {
			Self::Float(value) => Some(*value),
			_ => None,
		}
	}
	pub fn as_boolean(&self) -> Option<bool> {
		self.as_float().map(|v| v > 0f32)
	}
	pub fn as_waveform_index(&self) -> Option<WaveformIndex> {
		match self.value() {
			Self::WaveformIndex(value) => Some(*value),
			_ => None,
		}
	}
	pub fn as_track_set(&self) -> Option<Vec<String>> {
		match self.value() {
			Self::TrackSet(tracks) => Some(tracks.clone()),
			_ => None,
		}
	}
	pub fn as_identifier_literal(&self) -> Option<String> {
		match self.value() {
			Self::IdentifierLiteral(id) => Some(id.clone()),
			_ => None,
		}
	}

	pub fn as_string(&self) -> Option<String> {
		match self.value() {
			Self::String(content) => Some(content.clone()),
			_ => None,
		}
	}

	pub fn as_array(&self) -> Option<&Vec<Value>> {
		match self.value() {
			Self::Array(content) => Some(content),
			_ => None,
		}
	}

	pub fn as_assoc(&self) -> Option<&HashMap<String, Value>> {
		match self.value() {
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
		match self.value() {
			Self::NodeStructure(str) => Some(str.clone()),
			Self::Float(value) => Some(NodeStructure::Constant { value: *value, label: self.label() }),
			Self::NodeFactory(fact) => Some(NodeStructure::NodeFactory(fact.clone())),
			_ => None,
		}
	}
	pub fn as_node_factory(&self) -> Option<Rc<dyn NodeFactory>> {
		match self.value() {
			Self::NodeFactory(fact) => Some(fact.clone()),
			_ => None,
		}
	}

	pub fn as_function(&self) -> Option<Rc<dyn Function>> {
		match self.value() {
			Self::Function(func) => Some(func.clone()),
			_ => None,
		}
	}

	fn value(&self) -> &Value {
		let result = match self {
			Self::Labeled { inner, .. } => inner.value(),
			_ => self,
		};

		match result {
			Self::Labeled { .. } => assert!(false),
			_ => { },
		}

		result
	}

	pub fn label(&self) -> Option<String> {
		match self {
			Self::Labeled { label, .. } => Some(label.clone()),
			_ => None,
		}
	}
}

// 当面 boolean 型は設けず、正を truthy、0 と負を falsy として扱う。
// 代表の値として true = 1、false = -1 とする
pub const VALUE_FALSE: Value = Value::Float(-1f32);
pub const VALUE_TRUE: Value = Value::Float(1f32);

