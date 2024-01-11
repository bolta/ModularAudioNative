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
	// pub fn make_child(&self) -> Rc<Self> {
	// 	Rc::new(Self {
	// 		entries: HashMap::new(),
	// 		parent: Some(Rc::new(self)),
	// 	})
	// }

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

	pub fn set(&mut self, name: &String, value: Value) -> ModdlResult<()> {
println!("set: {}", name);
		if self.entries.contains_key(name) {
			Err(error(ErrorType::EntryDuplicate { name: name.clone() }, Location::dummy()))
		} else {
			self.entries.insert(name.clone(), value.clone());
			Ok(())
		}
	}

	pub fn entries(&self) -> &HashMap<String, Value> { &self.entries }
}
