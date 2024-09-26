use crate::core::{
	common::*,
	machine::*,
	node::*,
};

use graphviz_rust::{
	dot_generator::*,
	dot_structures::{
		Edge,
		EdgeTy,
		Graph,
		Id,
		Node as DotNode,
		NodeId as DotNodeId,
		Stmt,
		Subgraph,
		Vertex, Attribute,
	},
};

use std::collections::hash_map::HashMap;

use graphviz_rust::{
    cmd::Format,
    exec,
    printer::PrinterContext,
};

pub fn output_graph(graph: Graph) {
    let graph_out = exec(
        graph,
        &mut PrinterContext::default(),
        vec![Format::Svg.into()],
        // vec![Format::Dot.into()],
    ).unwrap();
	// print!("{}", graph_out);

	// TODO エラー処理
	let _ = std::fs::write("out.svg", &graph_out);
}

pub fn make_graph(all: &Vec<MachineSpec>, sends_to_receives: &HashMap<NodeId, NodeId>) -> Graph {
	let subgraphs = all.iter().enumerate().map(|(i, machine_spec)| machine_to_subgraph_stmt(i, machine_spec)).collect();
	let intermachine_edges = sends_to_receives.iter().map(
		|(send, receive)| stmt!(edge_dashed(make_edge(
			make_dot_node_id(send.machine.0, send.node_of_any_machine().unchanneled().0),
			make_dot_node_id(receive.machine.0, receive.node_of_any_machine().unchanneled().0),
		)))
	).collect();
	let stmts = [
		vec![
			stmt!(Attribute(id!("rankdir"), id!("LR"))),
		],
		subgraphs,
		intermachine_edges,
	].concat();
	Graph::DiGraph {
		id: id!("structure"),
		strict: false,
		stmts,
	}
}
fn machine_to_subgraph_stmt(machine_idx: usize, machine_spec: &MachineSpec) -> Stmt {
	let mut dot_nodes: Vec<_> = machine_spec.nodes.nodes().iter().enumerate().map(|(i, node)| node_to_dot_node_stmt(machine_idx, i, node)).collect();

	machine_spec.nodes.nodes().iter().enumerate().for_each(|(down_idx, node)| {
		let id_down = make_dot_node_id(machine_idx, down_idx);
		node.upstreams().iter().for_each(|up_idx| {
			let id_up = make_dot_node_id(machine_idx, up_idx.unchanneled().0);
			dot_nodes.push(stmt!(make_edge(id_up, id_down.clone())));
		});
	});
	stmt!(Subgraph {
		id: id!(format!("cluster_m{}", machine_idx)),
		stmts: vec![
			vec![
				stmt!(make_label_attr(machine_spec.name.as_str())),
			],
			dot_nodes,
		].concat(),
	})
}
fn make_edge(from: DotNodeId, to: DotNodeId) -> Edge {
	Edge {
		ty: EdgeTy::Pair(Vertex::N(from), Vertex::N(to)),
		attributes: vec![
			// make_label_attr("hoge"),
		],
	}
}
fn edge_dashed(mut edge: Edge) -> Edge {
	edge.attributes.push(Attribute(id!("style"), id!("dashed")));
	edge
}

fn node_to_dot_node_stmt(machine_idx: usize, node_idx: usize, node_: &Box<dyn Node>) -> Stmt {
	stmt!(DotNode {
		id: make_dot_node_id(machine_idx, node_idx),
		attributes: vec![
			make_label_attr(format!("[{}] {}", node_idx, node_.type_label()).as_str()),
		],
	})
}

fn make_dot_node_id(machine_idx: usize, node_idx: usize) -> DotNodeId {
	DotNodeId(id!(format!("m{}_n{}", machine_idx, node_idx)), None)
}

fn make_label_attr(text: &str) -> Attribute {
	Attribute(id!("label"), id!(esc text))
}

