use crate::{
	core::{
		common::*,
		context::*,
		machine::*,
		node::*,
		node_factory::*,
	},
	node::common::delay_buffer::*,
};
use node_macro::node_impl;

// Audio EQ Cookbook によるフィルタの実装
// https://www.w3.org/TR/audio-eq-cookbook/

macro_rules! bi_quad_filter {
	($name: ident, $factory_name: ident, $make_coeffs: expr) => {
		pub struct $name {
			base_: NodeBase,
			signal: MonoNodeIndex,
			cutoff: MonoNodeIndex,
			q: MonoNodeIndex,
		
			filter: BiQuadFilterCore,
		}
		impl $name {
			pub fn new(base: NodeBase, signal: MonoNodeIndex, cutoff: MonoNodeIndex, q: MonoNodeIndex) -> Self {
				Self { base_: base, signal, cutoff, q, filter: BiQuadFilterCore::new() }
			}
		}
		#[node_impl]
		impl Node for $name {
			fn channels(&self) -> i32 { 1 }
			fn upstreams(&self) -> Upstreams { vec![
				self.signal.channeled(),
				self.cutoff.channeled(),
				self.q.channeled(),
			] }
			fn activeness(&self) -> Activeness { Activeness::Active }
			fn execute(&mut self, inputs: &Vec<Sample>, output: &mut [Sample], context: &Context, _env: &mut Environment) {
				let in_value = inputs[0];
				let cutoff = inputs[1];
				let q = inputs[2];
		
				let coeffs = $make_coeffs(cutoff, q, context.sample_rate_f32());
		
				// self.filter.sample() の中で状態の更新も行われるので self.update() は実装しない
				let out_value = self.filter.sample(in_value, &coeffs);
				output_mono(output, out_value);
			}
		}
		
		pub struct $factory_name { }
		impl NodeFactory for $factory_name {
			fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![
				spec("cutoff", 1),
				spec("q", 1),
			] }
			fn input_channels(&self) -> i32 { 1 }
			fn create_node(&self, base: NodeBase, node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
				let signal = piped_upstream.as_mono();
				let cutoff = node_args.get("cutoff").unwrap().as_mono(); 
				let q = node_args.get("q").unwrap().as_mono(); 
				Box::new($name::new(base, signal, cutoff, q))
			}
		}
	}
}

bi_quad_filter!(LowPassFilter, LowPassFilterFactory, (|cutoff, q, sample_rate| {
	let vars = intermediate_vars(cutoff, q, sample_rate);
	let b1 = 1f32 - vars.cos_w0;
	let b0 = b1 / 2f32;
	BiQuadFilterCoeffs {
		b0,
		b1,
		b2: b0,
		a0: 1f32 + vars.alpha,
		a1: -2f32 * vars.cos_w0,
		a2: 1f32 - vars.alpha,
	}
}));
bi_quad_filter!(HighPassFilter, HighPassFilterFactory, (|cutoff, q, sample_rate| {
	let vars = intermediate_vars(cutoff, q, sample_rate);
	let b = 1f32 + vars.cos_w0;
	let b0 = b / 2f32;
	BiQuadFilterCoeffs {
		b0,
		b1: -b,
		b2: b0,
		a0: 1f32 + vars.alpha,
		a1: -2f32 * vars.cos_w0,
		a2: 1f32 - vars.alpha,
	}
}));
bi_quad_filter!(BandPassFilter, BandPassFilterFactory, (|cutoff, q, sample_rate| {
	let vars = intermediate_vars(cutoff, q, sample_rate);
	let b0 = q * vars.alpha;
	BiQuadFilterCoeffs {
		b0,
		b1: 0f32,
		b2: -b0,
		a0: 1f32 + vars.alpha,
		a1: -2f32 * vars.cos_w0,
		a2: 1f32 - vars.alpha,
	}
}));

struct BiQuadFilterCore {
	in_delay: DelayBuffer<Sample>,
	out_delay: DelayBuffer<Sample>,
}
impl BiQuadFilterCore {
	fn new() -> Self {
		Self {
			in_delay: DelayBuffer::<Sample>::new(2),
			out_delay: DelayBuffer::<Sample>::new(2),
		}
	}
	fn sample(&mut self, in_value: Sample, coeffs: &BiQuadFilterCoeffs) -> Sample {
		let out_value = (coeffs.b0 * in_value + coeffs.b1 * self.in_delay[0] + coeffs.b2 * self.in_delay[-1]
				- coeffs.a1 * self.out_delay[0] - coeffs.a2 * self.out_delay[-1])
				/ coeffs.a0;
		self.in_delay.push(in_value);
		self.out_delay.push(out_value);

		return out_value;
	}
}

struct BiQuadFilterCoeffs {
	b0: Sample,
	b1: Sample,
	b2: Sample,
	a0: Sample,
	a1: Sample,
	a2: Sample,
}

struct BiQuadFilterIntermediateVars {
	w0: Sample,
	cos_w0: Sample,
	sin_w0: Sample,
	alpha: Sample,
}

fn intermediate_vars(cutoff: Sample, q: Sample, sample_rate: Sample) -> BiQuadFilterIntermediateVars {
	let w0 = TWO_PI * cutoff / sample_rate;
	let cos_w0 = w0.cos();
	let sin_w0 = w0.sin();
	let alpha = sin_w0 / (2f32 * q);
	BiQuadFilterIntermediateVars { w0, cos_w0, sin_w0, alpha }
}
