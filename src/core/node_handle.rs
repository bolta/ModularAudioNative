use super::node::*;
use super::node_host::*;
use std::rc::Weak;
use std::cell::RefCell;

/*
 * FIXME 寿命のチェックを回避するために unsafe を多用している。
 * まともなやり方があれば今後がんばりたい
 */

 #[derive(Clone)]
pub struct NodeHandle/* <'a> */ {
	// TODO 限定公開がうまくいかない
	pub/* (in crate::core::node_host) */ host: */* 'a */ mut NodeHost,
	pub/* (in crate::core::node_host) */ id: usize,
}

impl NodeHandle {
	pub fn host(&self) -> &NodeHost {
		unsafe { &* self.host }
	}
	pub fn host_mut(&self) -> &/* 'a */ mut NodeHost {
		unsafe { &mut * self.host }
	}

	pub fn node(&self) -> &Node {
		unsafe { &* self.host }.node(self.id)
	}

	pub fn node_mut(&self) -> &mut Node {
		unsafe { &mut * self.host }.node_mut(self.id)
	}
}

