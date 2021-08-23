use super::{common::*, node::*};

use std::{
	collections::hash_map::HashMap,
	ops::{Index, IndexMut},
};

pub struct NodeHost {
	nodes: Vec<Box<dyn Node>>,
	tags: HashMap<String, Vec<NodeIndex>>,
}
impl NodeHost {
	pub fn new() -> Self {
		Self {
			nodes: vec![],
			tags: HashMap::new(),
		}
	}
	pub fn add(&mut self, node: Box<dyn Node>) -> ChanneledNodeIndex {
		let chs = node.channels();
		let idx = self.count();
		self.nodes.push(node);
		match chs {
			0 => ChanneledNodeIndex::no_output(idx),
			1 => ChanneledNodeIndex::mono(idx),
			2 => ChanneledNodeIndex::stereo(idx),
			_ => panic!("unsupported channel count: {}", chs),
		}
	}

	pub fn add_with_tags(&mut self, tags: Vec<String>, node: Box<dyn Node>) -> ChanneledNodeIndex {
		let idx = self.add(node);

		for tag in tags {
			if self.tags.contains_key(&tag) {
				self.tags.get_mut(&tag).unwrap().push(idx.unchanneled());
			} else {
				self.tags.insert(tag, vec![idx.unchanneled()]);
			}
		}

		idx
	}

	pub fn add_with_tag(&mut self, tag: String, node: Box<dyn Node>) -> ChanneledNodeIndex {
		self.add_with_tags(vec![tag], node)
	}

	pub fn count(&self) -> usize {
		self.nodes.len()
	}
	pub fn nodes(&self) -> &Vec<Box<dyn Node>> {
		&self.nodes
	}
	pub fn nodes_mut(&mut self) -> &mut Vec<Box<dyn Node>> {
		&mut self.nodes
	}

	// TODO Vec を作らずに参照で返した方がよさそう
	pub fn resolve_tag(&self, tag: &String) -> Vec<NodeIndex> {
		self.tags
			.get(tag)
			.map_or_else(|| vec![], |idxs| idxs.clone())
	}
}
impl Index<NodeIndex> for NodeHost {
	type Output = Box<dyn Node>;
	fn index(&self, idx: NodeIndex) -> &Self::Output {
		&self.nodes[idx.0]
	}
}
impl IndexMut<NodeIndex> for NodeHost {
	fn index_mut(&mut self, idx: NodeIndex) -> &mut Self::Output {
		&mut self.nodes[idx.0]
	}
}
// impl Index<&str> for NodeHost {
// 	type Output = Option<&self Box<dyn Node>>;
// 	fn index(&self, idx: &str) -> Self::Output { &self.ids[idx] }
// }
