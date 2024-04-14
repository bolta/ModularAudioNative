use parser::common::Location;

use crate::{
	moddl::{
		error::*,
		value::*,
	},
};

use std::{
	cell::RefCell,
	collections::hash_map::HashMap,
	rc::Rc,
};

// TODO entries をジェネリックにする
pub struct Scope {
	entries: HashMap<String, Value>,
	parent: Option<Rc<RefCell<Self>>>,
}
impl Scope {
	pub fn root(entries: HashMap<String, Value>) -> Rc<RefCell<Self>> {
		Rc::new(RefCell::new(Self {
			entries,
			parent: None,
		}))
	}
	pub fn child_of(parent: Rc<RefCell<Self>>) -> Rc<RefCell<Self>> {
		Rc::new(RefCell::new(Self {
			entries: HashMap::new(),
			parent: Some(parent),
		}))
	}

	pub fn lookup(&self, name: &String) -> Option<Value> {
		match self.entries.get(name) {
			Some(value) => Some(value.clone()),
			None => {
				match &self.parent {
					Some(parent) => parent.borrow().lookup(name),
					None => None,
				}
			}
		}
	}

	pub fn parent(&self) -> Option<&Rc<RefCell<Self>>> {
		match &self.parent {
			Some(parent) => Some(&parent),
			None => None,
		}
	}

	/// ルートのスコープを Rc で返す
	/// self 自体がルートである場合は失敗し、None を返す（self から「self を包む Rc」を返せないため）
	pub fn get_root(&self) -> Option<Rc<RefCell<Self>>> {
		match &self.parent {
			Some(parent) => {
				// parent のさらに親がない → parent がルート
				match parent.borrow().parent {
					Some(_) => parent.borrow().get_root(),
					None => Some(parent.clone()),
				}
			}
			None => None,
		}
	}

	pub fn set(&mut self, name: &String, value: Value) -> ModdlResult<()> {
		if let Some((_, existing_loc)) = self.entries.get(name) {
			Err(error(ErrorType::EntryDuplicate { name: name.clone() }, existing_loc.clone()))
		} else {
			self.entries.insert(name.clone(), value.clone());
			Ok(())
		}
	}

	pub fn entries(&self) -> &HashMap<String, Value> { &self.entries }
}
