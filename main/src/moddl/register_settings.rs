// レジスタの初期設定を収集するためのデータ構造を提供するモジュール

use crate::moddl::domain::DomainHint;

use super::value::*;
extern crate parser;
use parser::moddl::ast::QualifiedLabel;
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

// type RegisterInits = Vec<(String, f32)>;
pub type RegisterSettingsSubtree = BTreeMap<String, RegisterSettingsTreeNode>;
#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterSettingsTree(pub RegisterSettingsSubtree);
#[derive(Debug, Serialize, Deserialize)]
pub enum RegisterSettingsTreeNode {
	Settings {
		// 今のところ複数のキーの初期値を持つことはない。
		// 複数組持つ必要がある場合は key と init を HashMap<String, f32> などで持つようにすれば問題ないはず
		reg_key: String,
		init: f32,
		domain: Option<DomainHint>,
	},
	Group(RegisterSettingsSubtree),
}
impl RegisterSettingsTree {
	pub fn new() -> Self { Self(BTreeMap::new()) }
	pub fn add_settings(&mut self, path: &QualifiedLabel, reg_key: impl Into<String>, init: f32, domain: Option<DomainHint>) {
		let path_elems: Vec<&str> = path.elems().collect();
		Self::add_settings_iter(&mut self.0, &path_elems, reg_key.into(), init, domain);
	}
	fn add_settings_iter(tree: &mut RegisterSettingsSubtree, path: &[&str], reg_key: String, init: f32, domain: Option<DomainHint>) {
		match path.len() {
			0 => {
				unreachable!();
			},
			1 => {
				let key = path[0].to_string();
				tree.insert(key, RegisterSettingsTreeNode::Settings { reg_key, init, domain });
			}, 
			_ => {
				let key = path[0].to_string();
				match tree.get_mut(&key) {
					None => {
						let mut subtree = RegisterSettingsSubtree::new();
						Self::add_settings_iter(&mut subtree, &path[1 ..], reg_key, init, domain);
						tree.insert(key, RegisterSettingsTreeNode::Group(subtree));
					},
					Some(RegisterSettingsTreeNode::Settings { .. }) => {
						// TODO エラーにする
						println!("(x_x)");
					},
					Some(RegisterSettingsTreeNode::Group(subtree)) => {
						Self::add_settings_iter(subtree, &path[1 ..], reg_key, init, domain);
					},
				}
			}
		}
	}
}

