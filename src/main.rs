mod core;
use crate::core::node::*;

#[derive(Clone, Copy)]
pub struct InputIndex(pub usize);

pub enum Instruction {
	Load { to: InputIndex, from: NodeIndex },
	Store { to: NodeIndex },
	Execute(NodeIndex),
	Update(NodeIndex),
}

fn main() {
	let mut nodes = vec![];
	{
		let mut add_node = |node: Box<dyn Node>| -> NodeIndex {
			nodes.push(node);
			NodeIndex(nodes.len() - 1)
		};
		let const_441 = add_node(Box::new(Constant::new(441f32)));
		let sin = add_node(Box::new(SineOsc::new(const_441)));
		let const_1 = add_node(Box::new(Constant::new(1f32)));
		let sum = add_node(Box::new(Add::new(vec![sin, const_1])));
		let _print = add_node(Box::new(Print::new(sum)));
	}

	// これ以降 nodes に要素を足してはいけない（nodes の参照を取ってあれこれするかもしれないため）

	let num_nodes = nodes.len();

	let vec_with_length = |len| {
		let mut result = Vec::with_capacity(len);
		for _ in 0 .. len { result.push(0f32); }
		result
	};

	let mut values = vec_with_length(num_nodes);
	let upstreams: Vec<Vec<NodeIndex>> = nodes.iter()
			.map(|node| node.upstreams())
			.collect();
	let mut inputs = vec_with_length(upstreams.iter().map(|u| u.len()).max().unwrap());

	let mut output = 0f32;
	// nodes が topologically sorted であることを期待している。
	// 普通に構築すればそうなるはず…
	let instructions: Vec<Instruction> = (0usize .. num_nodes).flat_map(|i| {
		let loads = upstreams[i].iter().enumerate().map(|(input_idx, upstream_idx)| {
			Instruction::Load { to: InputIndex(input_idx), from: *upstream_idx }
		});

		let node_idx = NodeIndex(i);
		loads.chain(vec![
			Instruction::Execute(node_idx),
			Instruction::Store { to: node_idx },
			Instruction::Update(node_idx),
		])
	}).collect();

	 ////
	//// PLAY

	for node in &mut nodes { node.initialize(); }
	/* loop */ for _ in 0 .. 101 {
		for instrc in &instructions {
			match instrc {
				Instruction::Load { to, from } => {
					inputs[to.0] = values[from.0];
				}
				Instruction::Store { to } => {
					values[to.0] = output;
				}
				Instruction::Execute(node_idx) => {
					output = nodes[node_idx.0].execute(&inputs);
				}
				Instruction::Update(node_idx) => {
					nodes[node_idx.0].update(&inputs);
				}
			}
		}
	}
	for node in nodes.iter_mut().rev() { node.finalize(); }
}
