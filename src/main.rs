mod core;
use crate::core::node::*;

fn main() {
//    println!("Hello, world!");

	let mut nodes = vec![];
	{
		let mut add_node = |node: Box<dyn Node>| -> NodeIndex {
			nodes.push(node);
			nodes.len() - 1
		};
		// 普通に構築すれば topologically sorted なはず
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

	for node in &mut nodes { node.initialize(); }
	/* loop */ for _ in 0 .. 101 {
		for i in 0usize .. num_nodes {
			let node = &mut nodes[i];

			for (input_idx, upstream_idx) in upstreams[i].iter().enumerate() {
				inputs[input_idx] = values[*upstream_idx];
			}
			values[i] = node.execute(&inputs);
			node.update(&inputs);
		}
	}
	for node in nodes.iter_mut().rev() { node.finalize(); }
}

