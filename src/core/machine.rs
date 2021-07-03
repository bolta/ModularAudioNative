use super::node::*;

pub struct Machine {
	nodes: Vec<Box<dyn Node>>,
}
impl Machine {
	pub fn new() -> Self { Self { nodes: vec![] } }

	pub fn add_node(&mut self, node: Box<dyn Node>) -> NodeIndex {
		self.nodes.push(node);
		NodeIndex(self.nodes.len() - 1)
	}

	// TODO Node から状態を切り離すことができれば mut は不要になるのだが
	pub fn play(&mut self) {
		let num_nodes = self.nodes.len();
		let upstreams: Vec<Vec<NodeIndex>> = self.nodes.iter()
				.map(|node| node.upstreams())
				.collect();
		let instructions = self.compile(&upstreams);

		let mut state = State {
			values: vec_with_length(num_nodes),
			inputs: vec_with_length(upstreams.iter().map(|u| u.len()).max().unwrap()),
			output: 0f32,
		};
	
		for node in &mut self.nodes { node.initialize(); }
		/* loop */ for _ in 0 .. 101 {
			for instrc in &instructions {
				self.do_instruction(&instrc, &mut state);
			}
			// TODO ここでイベント処理などを行う
		}
		for node in self.nodes.iter_mut().rev() { node.finalize(); }
	}

	fn compile(&self, upstreams: &Vec<Vec<NodeIndex>>) -> Vec<Instruction> {
		// nodes が topologically sorted であることを期待している。
		// 普通に構築すればそうなるはず…
		(0usize .. self.nodes.len()).flat_map(|i| {
			let loads = upstreams[i].iter().enumerate().map(|(input_idx, upstream_idx)| {
				Instruction::Load { to: InputIndex(input_idx), from: *upstream_idx }
			});

			let node_idx = NodeIndex(i);
			loads.chain(vec![
				Instruction::Execute(node_idx),
				Instruction::Store { to: node_idx },
				Instruction::Update(node_idx),
			])
		}).collect()
	}

	fn do_instruction(&mut self, instrc: &Instruction, state: &mut State) {
		match instrc {
			Instruction::Load { to, from } => {
				state.inputs[to.0] = state.values[from.0];
			}
			Instruction::Store { to } => {
				state.values[to.0] = state.output;
			}
			Instruction::Execute(node_idx) => {
				state.output = self.nodes[node_idx.0].execute(&state.inputs);
			}
			Instruction::Update(node_idx) => {
				self.nodes[node_idx.0].update(&state.inputs);
			}
		}
	}
}

#[derive(Clone, Copy)]
struct InputIndex(pub usize);

enum Instruction {
	Load { to: InputIndex, from: NodeIndex },
	Store { to: NodeIndex },
	Execute(NodeIndex),
	Update(NodeIndex),
}

struct State {
	values: Vec<Sample>,
	inputs: Vec<Sample>,
	output: Sample,
}

fn vec_with_length(len: usize) -> Vec<Sample> {
	let mut result = Vec::with_capacity(len);
	for _ in 0 .. len { result.push(0f32); }
	result
}
