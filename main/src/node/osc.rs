use crate::core::{
	common::*,
	context::*,
	machine::*,
	node::*,
	node_factory::*,
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
			pub fn new(base: NodeBase, freq: MonoNodeIndex) -> Self { Self { base_: base,  freq, phase: 0f32 } }
		}
		
		#[node_impl]
		impl Node for $name {
			fn channels(&self) -> i32 { 1 }
			fn initialize(&mut self, _context: &Context, _env: &mut Environment) { self.phase = 0f32; }
			fn upstreams(&self) -> Upstreams { vec![self.freq.channeled()] }
			fn activeness(&self) -> Activeness { Activeness::Active }
			fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut [Sample], _context: &Context, _env: &mut Environment) {
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
	fn execute(&mut self, inputs: &Vec<Sample>, output: &mut [Sample], _context: &Context, _env: &mut Environment) {
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

 ////
//// StereoTestOsc

pub struct StereoTestOsc {
	base_: NodeBase,
	freq: MonoNodeIndex,

	phase_l: f32,
	phase_r: f32,
}
impl StereoTestOsc {
	pub fn new(base: NodeBase, freq: MonoNodeIndex) -> Self { Self { base_: base, freq, phase_l: 0f32, phase_r: 0f32 } }
}
#[node_impl]
impl Node for StereoTestOsc {
	fn channels(&self) -> i32 { 2 }
	fn initialize(&mut self, _context: &Context, _env: &mut Environment) { }
	fn upstreams(&self) -> Upstreams { vec![self.freq.channeled()] }
	fn activeness(&self) -> Activeness { Activeness::Active }
	fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut [Sample], _context: &Context, _env: &mut Environment) {
		output_stereo(output, self.phase_l.sin(), self.phase_r.sin());
	}
	fn update(&mut self, inputs: &Vec<Sample>, context: &Context, _env: &mut Environment) {
		let freq = inputs[0];
		self.phase_l = (self.phase_l + TWO_PI * freq         / context.sample_rate_f32()) % TWO_PI;
		self.phase_r = (self.phase_r + TWO_PI * freq / 2_f32 / context.sample_rate_f32()) % TWO_PI;
	}
}

pub struct StereoTestOscFactory { }
impl NodeFactory for StereoTestOscFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![] }
	fn input_channels(&self) -> i32 { 1 }
	fn create_node(&self, base: NodeBase, _node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		let freq = piped_upstream.as_mono();
		Box::new(StereoTestOsc::new(base, freq))
	}
}
