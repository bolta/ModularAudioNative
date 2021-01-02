use crate::core::{
	node::*,
	node_handle::*,
	node_host::*,
};
use super::util::*;

pub fn negate(arg: NodeHandle) -> NodeHandle {
	let host = arg.host_mut();
	let node = discard_lifetime(arg.node());

	host.add_node(Node::from_closure_named(String::from("negate"), Box::new(move || {
		Some(- node.current())
	})))
}

pub fn add(lhs: NodeHandle, rhs: NodeHandle) -> NodeHandle {
	binary("add", lhs, rhs, |l, r| l + r)
}

pub fn sub(lhs: NodeHandle, rhs: NodeHandle) -> NodeHandle {
	binary("sub", lhs, rhs, |l, r| l - r)
}

pub fn mul(lhs: NodeHandle, rhs: NodeHandle) -> NodeHandle {
	binary("mul", lhs, rhs, |l, r| l * r)
}

pub fn div(lhs: NodeHandle, rhs: NodeHandle) -> NodeHandle {
	binary("div", lhs, rhs, |l, r| l / r)
}



fn binary(name: &'static str, lhs: NodeHandle, rhs: NodeHandle, oper: fn (f32, f32) -> f32) -> NodeHandle {
	let host = lhs.host_mut();
	let l_node = discard_lifetime(lhs.node());
	let r_node = discard_lifetime(rhs.node());

	host.add_node(Node::from_closure_named(String::from(name), Box::new(move || {
		Some(oper(l_node.current(), r_node.current()))
	})))
}
