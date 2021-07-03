pub type Sample = f32;

#[derive(Clone, Copy)]
pub struct NodeIndex(pub usize);

const TWO_PI: f32 = 2_f32 * std::f32::consts::PI;

const SAMPLE_RATE: i32 = 44100; // TODO どこに置こう？
const SAMPLE_RATE_F32: f32 = SAMPLE_RATE as f32; // TODO どこに置こう？

const NO_OUTPUT: Sample = f32::NAN;

pub trait Node {
	fn initialize(&mut self) { }
	fn upstreams(&self) -> Vec<NodeIndex>; // { vec![] }
	fn execute(&self, inputs: &Vec<Sample>) -> Sample; // ここで状態を変えないといけない場合があるかも？
	fn update(&mut self, _inputs: &Vec<Sample>) { }
	fn finalize(&mut self) { }
}


pub struct Constant {
	value: Sample,
}
impl Constant {
	pub fn new(value: Sample) -> Self { Self { value } }
}
impl Node for Constant {
	fn upstreams(&self) -> Vec<NodeIndex> { vec![] }
	fn execute(&self, _inputs: &Vec<Sample>) -> Sample { self.value }
}

pub struct SineOsc {
	freq: NodeIndex,

	phase: f32,
}
impl SineOsc {
	pub fn new(freq: NodeIndex) -> Self { Self { freq, phase: 0f32 } }
}
impl Node for SineOsc {
	fn upstreams(&self) -> Vec<NodeIndex> { vec![self.freq] }
	fn execute(&self, _inputs: &Vec<Sample>) -> Sample { self.phase.sin() }
	fn update(&mut self, inputs: &Vec<Sample>) {
		let freq = inputs[0];
		self.phase = (self.phase + TWO_PI * freq / SAMPLE_RATE_F32) % TWO_PI;
	}
}

pub struct Add {
	args: Vec<NodeIndex>,
}
impl Add {
	pub fn new(args: Vec<NodeIndex>) -> Self { Self { args } }
}
impl Node for Add {
	fn upstreams(&self) -> Vec<NodeIndex> { self.args.clone() }
	fn execute(&self, inputs: &Vec<Sample>) -> Sample {
		inputs.iter().take(self.args.len()).sum()
	}
}

pub struct Print {
	input: NodeIndex,
}
impl Print {
	pub fn new(input: NodeIndex) -> Self { Self { input } }
}
impl Node for Print {
	// TODO ↓これ抽象クラス的なものに括り出したい
	fn upstreams(&self) -> Vec<NodeIndex> { vec![self.input] }
	fn execute(&self, inputs: &Vec<Sample>) -> Sample {
		println!("{}", inputs[0]);
		NO_OUTPUT
	}
}
