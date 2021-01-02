use super::util::*;
use crate::core::{node::*, node_handle::*, node_host::*};

pub fn constant(host: &mut NodeHost, value: f32) -> NodeHandle {
	host.add_node(Node::from_closure_named(String::from("constant"), Box::new(move || {
		// println!("{}", value);
		Some(value)
	})))
}
