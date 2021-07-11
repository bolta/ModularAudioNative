use super::{
	common::*,
	node::*,
};

use std::{
	collections::hash_map::HashMap,
	ops::{
		Index,
		IndexMut,
	}
};

pub struct NodeHost {
	nodes: Vec<Box<dyn Node>>,
	ids: HashMap<String, NodeIndex>,
}
impl NodeHost {
	pub fn new() -> Self {
		Self {
			nodes: vec![],
			ids: HashMap::new(),
		}
	}
	pub fn add(&mut self, node: Box<dyn Node>) -> NodeIndex {
		self.nodes.push(node);
		NodeIndex(self.count() - 1)
	}
	pub fn add_with_id(&mut self, id: &str, node: Box<dyn Node>) -> NodeIndex {
		let key = String::from(id);
		if self.ids.contains_key(&key) {
			println!("NodeHost::add_with_id: id {} already exists.", id);
		}

		let idx = self.add(node);
		self.ids.insert(key, idx);

		idx
	}

	pub fn count(&self) -> usize { self.nodes.len() }
	pub fn nodes(&self) -> &Vec<Box<dyn Node>> { &self.nodes }
	pub fn nodes_mut(&mut self) -> &mut Vec<Box<dyn Node>> { &mut self.nodes }

	pub fn resolve_id(&self, id: &String) -> Option<NodeIndex> { self.ids.get(id).map(|id| *id) }
}
impl Index<NodeIndex> for NodeHost {
	type Output = Box<dyn Node>;
	fn index(&self, idx: NodeIndex) -> &Self::Output { &self.nodes[idx.0] }
}
impl IndexMut<NodeIndex> for NodeHost {
	fn index_mut(&mut self, idx: NodeIndex) -> &mut Self::Output { &mut self.nodes[idx.0] }
}
// impl Index<&str> for NodeHost {
// 	type Output = Option<&self Box<dyn Node>>;
// 	fn index(&self, idx: &str) -> Self::Output { &self.ids[idx] }
// }
