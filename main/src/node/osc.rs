use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
	node_factory::*,
	util::is_true,
};
use node_macro::node_impl;

macro_rules! simple_osc {
	($name: ident, $factory_name: ident, $expr: expr) => {
		pub struct $name {
			base_: NodeBase,
			freq: MonoNodeIndex,
		
			phase: f32,
		}
		impl $name {
			pub fn new(base: NodeBase, freq: MonoNodeIndex) -> Self { Self { base_: base, freq, phase: 0f32 } }
		}
		
		#[node_impl]
		impl Node for $name {
			fn channels(&self) -> i32 { 1 }
			fn initialize(&mut self, _context: &Context, _env: &mut Environment) { self.phase = 0f32; }
			fn upstreams(&self) -> Upstreams { vec![self.freq.channeled()] }
			fn activeness(&self) -> Activeness { Activeness::Active }
			fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut [OutputBuffer], _context: &Context, _env: &mut Environment) {
				// TODO クロージャを呼び出すことでオーバーヘッドが乗ったりしないか？　検証
				output_mono(output, $expr(self.phase));
			}
			fn update(&mut self, inputs: &Vec<Sample>, context: &Context, _env: &mut Environment) {
				let freq = inputs[0];
				self.phase = (self.phase + TWO_PI * freq / context.sample_rate_f32()) % TWO_PI;
			}
		}

		pub struct $factory_name { }
		impl NodeFactory for $factory_name {
			fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![] }
			fn input_channels(&self) -> i32 { 1 }
			fn create_node(&self, base: NodeBase, _node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
				let freq = piped_upstream.as_mono();
				Box::new($name::new(base, freq))
			}
		}
	}
}

 ////
//// Phase

// Phase の実装は各種オシレータとほぼ同じであり、位相をリセットする機構のために別に設けているだけ。
// 今後各種オシレータのネイティブ実装は不要になるかもしれない（パフォーマンス面で問題がなければ）
pub struct Phase {
	base_: NodeBase,
	freq: MonoNodeIndex,
	reset: MonoNodeIndex,

	phase: f32,
}
impl Phase {
	pub fn new(base: NodeBase, freq: MonoNodeIndex, reset: MonoNodeIndex, initial: f32) -> Self {
		Self { base_: base, freq, reset, phase: initial }
	}
}

#[node_impl]
impl Node for Phase {
	fn channels(&self) -> i32 { 1 }
	fn upstreams(&self) -> Upstreams { vec![self.freq.channeled(), self.reset.channeled()] }
	fn activeness(&self) -> Activeness { Activeness::Active }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut [OutputBuffer], _context: &Context, _env: &mut Environment) {
		let reset = is_true(inputs[1]);
		if reset { self.phase = 0f32; }
		output_mono(output, self.phase);
	}
	fn update(&mut self, inputs: &Vec<Sample>, context: &Context, _env: &mut Environment) {
		let freq = inputs[0];
		self.phase = (self.phase + TWO_PI * freq / context.sample_rate_f32()) % TWO_PI;
	}
}

pub struct PhaseFactory {
	initial: f32,
}
impl PhaseFactory {
	pub fn new(initial: f32) -> Self { Self { initial } }
}
impl NodeFactory for PhaseFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![spec_with_default("reset", 1, 0f32)] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, base: NodeBase, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		let freq = piped_upstream.as_mono();
		let reset = node_args.get("reset").unwrap().as_mono(); 
		Box::new(Phase::new(base, freq, reset, self.initial))
	}
}

 ////
//// Simple Oscillators

simple_osc!(SineOsc, SineOscFactory, (|phase: f32| phase.sin()));
simple_osc!(TriangleOsc, TriangleOscFactory, (|phase: f32|
		if phase < 0.25 * TWO_PI {
			(2f32 / PI) * phase
		} else if phase < 0.75 * TWO_PI {
			- (2f32 / PI) * phase + 2f32
		} else {
			(2f32 / PI) * phase - 4f32
		}));
simple_osc!(SawOsc, SawOscFactory, (|phase: f32| phase / PI - 1f32));

 ////
//// Pulse Oscillator

pub struct PulseOsc {
	base_: NodeBase,
	freq: MonoNodeIndex,
	duty: MonoNodeIndex,

	phase: f32,
}
impl PulseOsc {
	pub fn new(base: NodeBase, freq: MonoNodeIndex, duty: MonoNodeIndex) -> Self { Self { base_: base,  freq, duty, phase: 0f32 } }
}

#[node_impl]
impl Node for PulseOsc {
	fn channels(&self) -> i32 { 1 }
	fn initialize(&mut self, _context: &Context, _env: &mut Environment) { self.phase = 0f32; }
	fn upstreams(&self) -> Upstreams { vec![self.freq.channeled(), self.duty.channeled()] }
	fn activeness(&self) -> Activeness { Activeness::Active }
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut [OutputBuffer], _context: &Context, _env: &mut Environment) {
		let duty = inputs[1];
		output_mono(output, if self.phase % TWO_PI < TWO_PI * duty {
			1f32 
		} else {
			-1f32
		});
	}
	fn update(&mut self, inputs: &Vec<Sample>, context: &Context, _env: &mut Environment) {
		let freq = inputs[0];
		self.phase = (self.phase + TWO_PI * freq / context.sample_rate_f32()) % TWO_PI;
	}
}

pub struct PulseOscFactory { }
impl NodeFactory for PulseOscFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![spec_with_default("duty", 1, 0.5f32)] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, base: NodeBase, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		let freq = piped_upstream.as_mono();
		let duty = node_args.get("duty").unwrap().as_mono(); 
		Box::new(PulseOsc::new(base, freq, duty))
	}
}

