use super::node::*;
use super::node_host::*;
use std::rc::Weak;
use std::cell::RefCell;

/*
 * FIXME 寿命のチェックを回避するために unsafe を多用している。
 * まともなやり方があれば今後がんばりたい
 */

pub struct NodeHandle {
	// TODO 限定公開がうまくいかない
	pub/* (in crate::core::node_host) */ host: *mut NodeHost,
	pub/* (in crate::core::node_host) */ id: usize,
}

impl NodeHandle {
	fn host(&self) -> &NodeHost {
		unsafe { &* self.host }
	}
	fn host_mut(&self) -> &mut NodeHost {
		unsafe { &mut * self.host }
	}

	pub fn node(&self) -> &Node {
		unsafe { &* self.host }.node(self.id)
	}

	pub fn node_mut(&self) -> &mut Node {
		unsafe { &mut * self.host }.node_mut(self.id)
	}

	// exper

	pub fn constant(host: &mut NodeHost, value: f32) -> Self {
		host.add_node(Node::from_closure(Box::new(move || {
			Some(value)
		})))
	}

	pub fn negate(&self) -> Self {
		let host = self.host_mut();
		let node = discard_lifetime(self.node());

		host.add_node(Node::from_closure(Box::new(move || {
			Some(- node.current())
		})))
	}

	pub fn add<'a>(&self, other: &'a Self) -> Self {
		let host = self.host_mut();
		let lhs = discard_lifetime(self.node());
		let rhs = discard_lifetime(other.node());

		host.add_node(Node::from_closure(Box::new(move || {
			Some(lhs.current() + rhs.current())
		})))
	}
}

/// 安全なはずだが寿命チェックに引っかかってしまう場合、
/// 一旦ポインタにすることでチェックを逃れる（こんな方法しかないのか？？）
fn discard_lifetime<'a>(node: &'a Node) -> &'static Node {
	unsafe { &* (node as *const Node) }
}
