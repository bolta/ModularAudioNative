use itertools::Itertools;
use parser::common::Location;
use parser::moddl::ast::QualifiedLabel;

use super::error::{ModdlResult, error, ErrorType};
use super::io::Io;
use super::function::*;
use crate::moddl::module_def::{ModuleDef, NodeDef};
use crate::wave::waveform_host::WaveformIndex;
use enum_display::EnumDisplay;
use std::cell::RefCell;
use std::{
	collections::HashMap,
	rc::Rc,
};

pub type Value = (ValueBody, Location);

pub trait ValueExtraction {
	fn as_number(&self) -> ModdlResult<(f32, Location)>;
	fn as_boolean(&self) -> ModdlResult<(bool, Location)>;
	fn as_waveform_index(&self) -> ModdlResult<(WaveformIndex, Location)>;
	fn as_track_set(&self) -> ModdlResult<(Vec<String>, Location)>;
	fn as_quoted_identifier(&self) -> ModdlResult<(String, Location)>;
	fn as_string(&self) -> ModdlResult<(String, Location)>;
	fn as_array(&self) -> ModdlResult<(&Vec<Value>, Location)>;
	fn as_assoc(&self) -> ModdlResult<(&HashMap<String, Value>, Location)>;
	fn as_module_def(&self) -> ModdlResult<(ModuleDef, Location)>;
	fn as_node_def(&self) -> ModdlResult<(NodeDef, Location)>;
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
	fn as_number(&self) -> ModdlResult<(f32, Location)> { extract(self.0.as_number(), &self.1, ValueType::Number) }
	fn as_boolean(&self) -> ModdlResult<(bool, Location)> { extract(self.0.as_boolean() , &self.1, ValueType::Number) }
	fn as_waveform_index(&self) -> ModdlResult<(WaveformIndex, Location)> { extract(self.0.as_waveform_index() , &self.1, ValueType::Waveform) }
	fn as_track_set(&self) -> ModdlResult<(Vec<String>, Location)> { extract(self.0.as_track_set() , &self.1, ValueType::TrackSet) }
	fn as_quoted_identifier(&self) -> ModdlResult<(String, Location)> { extract(self.0.as_quoted_identifier() , &self.1, ValueType::QuotedIdentifier) }
	fn as_string(&self) -> ModdlResult<(String, Location)> { extract(self.0.as_string() , &self.1, ValueType::String) }
	fn as_array(&self) -> ModdlResult<(&Vec<Value>, Location)> { extract(self.0.as_array() , &self.1, ValueType::Array) }
	fn as_assoc(&self) -> ModdlResult<(&HashMap<String, Value>, Location)> { extract(self.0.as_assoc() , &self.1, ValueType::Assoc) }
	fn as_module_def(&self) -> ModdlResult<(ModuleDef, Location)> { extract_any(self.0.as_module_def() , &self.1,
			vec![ValueType::ModuleDef, ValueType::Number, ValueType::NodeDef]) }
	fn as_node_def(&self) -> ModdlResult<(NodeDef, Location)> { extract(self.0.as_node_factory() , &self.1, ValueType::NodeDef) }
	fn as_function(&self) -> ModdlResult<(Rc<dyn Function>, Location)> { extract(self.0.as_function() , &self.1, ValueType::Function) }
	fn as_io(&self) -> ModdlResult<(Rc<RefCell<dyn Io>>, Location)> { extract(self.0.as_io() , &self.1, ValueType::Io) }
}

#[derive(Clone)]
pub enum ValueBody {
	Number(f32),
	WaveformIndex(WaveformIndex),
	TrackSet(Vec<String>),
	QuotedIdentifier(String),
	String(String),
	Array(Vec<Value>),
	Assoc(HashMap<String, Value>),
	/// ノードの構造に関するツリー表現
	ModuleDef(ModuleDef),
	/// 引数を受け取ってノードを生成する関数
	NodeDef(NodeDef),
	Function(Rc<dyn Function>),
	Io(Rc<RefCell<dyn Io>>),
}

impl ValueBody {
	pub fn as_number(&self) -> Option<f32> {
		match self {
			Self::Number(value) => Some(*value),
			_ => None,
		}
	}
	pub fn as_boolean(&self) -> Option<bool> {
		self.as_number().map(|v| v > 0f32)
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
	pub fn as_quoted_identifier(&self) -> Option<String> {
		match self {
			Self::QuotedIdentifier(id) => Some(id.clone()),
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

	pub fn as_module_def(&self) -> Option<ModuleDef> {
		// Value から直接 Node に変換しようとすると NodeHost が必要になったり、
		// Node をタグ付きで生成したいときに困ったりとよろしくないことが多いので、
		// Node への変換は提供しない。
		// 代わりに、Node の一歩手前というか、ノードグラフの設計図となる ModuleDef を提供し、
		// そこから Node を生成するのは然るべき場所（Player）でいいようにやってもらうこととする。
		// 数値や変数参照から Node への暗黙の変換もここで提供する
		match self {
			Self::ModuleDef(str) => Some(str.clone()),
			Self::Number(value) => Some(ModuleDef::Constant { value: *value, label: None }),
			Self::NodeDef(fact) => Some(ModuleDef::NodeCreation {
				factory: fact.clone(),
				args: HashMap::new(),
				label: None,
			}),
			_ => None,
		}
	}
	pub fn as_node_factory(&self) -> Option<NodeDef> {
		match self {
			Self::NodeDef(fact) => Some(fact.clone()),
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

	pub fn label(&self) -> Option<QualifiedLabel> {
		match self {
			Self::ModuleDef(strukt) => strukt.label(),
			_ => None,
		}
	}

	/// 値が文字列の場合に無駄なコピーを発生させないよう、参照をコールバック内で使ってもらう
	pub fn to_str<T>(&self, use_str: impl Fn (&str) -> T) -> T {
		match self {
			Self::String(s) => use_str(s),
			_ => use_str(& self.force_to_string()),
		}
	}

	// 文字列の場合も含めて必ず値を返す版
	pub fn force_to_string(&self) -> String {
		match self {
			Self::Number(value) => value.to_string(),
			Self::WaveformIndex(index) => format!("Waveform({})", index.0),
			Self::TrackSet(tracks) => if tracks.iter().all(|t| t.len() == 1) {
				format!("^{}", tracks.join(""))
			} else {
				// 複数文字のトラック名は現状ないが、仮の記法で出力しておく
				format!("^({})", tracks.join(", "))
			},
			Self::QuotedIdentifier(id) => format!(":{}", id),
			// 文字列だけは「式っぽい」整形を受けず中身そのままなので、少し毛色が違う
			Self::String(value) => value.clone(),
			Self::Array(elems) => {
				let content = elems.iter().map(|(e, _)| e.force_to_string()).join(", ");
				format!("[{}]", content)
			},
			Self::Assoc(entries) => {
				let content = entries.iter().map(|(k, (v, _))| format!("{}: {}", k, v.force_to_string())).join(", ");
				format!("{{ {} }}", content)
			},
			Self::ModuleDef(strukt) => format!("ModuleDef({})", strukt.to_string()),

			// TODO 以下、もうちょっと何か出せるか？
			Self::NodeDef(_fact) => "(NodeDef)".to_string(),
			Self::Function(_func) => "(Function)".to_string(),
			Self::Io(_io) => "(Io)".to_string(),
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
	ModuleDef,
	NodeDef,
	Function,
	Io,
}

// 当面 boolean 型は設けず、正を truthy、0 と負を falsy として扱う。
// 代表の値として true = 1、false = -1 とする
pub fn false_value() -> Value { (ValueBody::Number(-1f32), Location::dummy()) }
pub fn true_value() -> Value { (ValueBody::Number(1f32), Location::dummy()) }
