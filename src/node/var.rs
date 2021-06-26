use super::super::core::{
	node::*,
	node_handle::*,
	node_host::*,
};

use ::std::cell::Cell;
use ::std::rc::Rc;

pub struct VarController {
	handle: NodeHandle,
	value: Rc<Cell<f32>>,
}

impl VarController {

	pub fn new(host: &mut NodeHost, init_value: f32) -> Self {
		let value = Rc::new(Cell::new(init_value));
		let value_for_node = Rc::clone(&value);
		Self {
			handle: host.add_node(Node::from_closure_named(String::from("var"), Box::new(move || {
				Some(value_for_node.get())
			}))),
			value,
		}
	}

	pub fn handle(&self) -> NodeHandle { self.handle.clone() }

	pub fn set(&mut self, value: f32) {
		self.value.set(value);
	}
}
