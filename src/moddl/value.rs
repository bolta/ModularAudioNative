use super::node_factory::*;

use crate::{
	core::{common::*, node::*, node_host::*},
	node::prim::*,
};

/// 生成すべき Node の構造を表現する型。
/// Value から直接 Node を生成すると問題が多いので、一旦この形式を挟む
#[derive(Clone)]
pub enum NodeStructure {
	Connect(Box<NodeStructure>, Box<NodeStructure>),
	Power(Box<NodeStructure>, Box<NodeStructure>),
	Multiply(Box<NodeStructure>, Box<NodeStructure>),
	Divide(Box<NodeStructure>, Box<NodeStructure>),
	Remainder(Box<NodeStructure>, Box<NodeStructure>),
	Add(Box<NodeStructure>, Box<NodeStructure>),
	Subtract(Box<NodeStructure>, Box<NodeStructure>),
	Identifier(String),
	// Lambda,
	NodeWithArgs {
		factory: Box<NodeStructure>,
		label: String,
		args: Vec<(String, Value)>,
	},
	Constant(f32),
}

#[derive(Clone)]
pub enum Value {
	Float(f32),
	TrackSet(Vec<String>),
	/// Identifier（foo）の評価結果としての値（IdentifierLiteral は `foo` の結果）
	Identifier(String),
	// Node(NodeIndex),
	NodeStructure(NodeStructure),
}

impl Value {
	pub fn as_float(&self) -> Option<f32> {
		match self {
			Self::Float(value) => Some(*value),
			_ => None,
		}
	}
	pub fn as_track_set(&self) -> Option<Vec<String>> {
		match self {
			Self::TrackSet(tracks) => Some(tracks.clone()),
			_ => None,
		}
	}
	pub fn as_identifier(&self) -> Option<String> {
		match self {
			Self::Identifier(id) => Some(id.clone()),
			_ => None,
		}
	}

	pub fn as_node_structure(&self) -> Option<NodeStructure> {
		// Value から直接 Node に変換しようとすると NodeHost が必要になったり、
		// Node をタグ付きで生成したいときに困ったりとよろしくないことが多いので、
		// Node への変換は提供しない。
		// 代わりに、Node の一歩手前というか、Node の生成における仕様となる NodeStructure を提供し、
		// そこから Node を生成するのは然るべき場所（Player）でいいようにやってもらうこととする。
		// 数値や変数参照から Node への暗黙の変換もここで提供する
		match self {
			Self::NodeStructure(str) => Some(str.clone()),
			Self::Float(value) => Some(NodeStructure::Constant(*value)),
			Self::Identifier(id) => Some(NodeStructure::Identifier(id.clone())),
			_ => None,
		}
	}
}
