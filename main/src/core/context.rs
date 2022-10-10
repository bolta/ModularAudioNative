use super::{
	common::*,
};

pub struct Context {
	sample_rate: i32,
	elapsed_samples: SampleCount,
	sample_rate_f32: f32,
}
impl Context {
	pub fn new(sample_rate: i32) -> Self {
		Self {
			sample_rate,
			elapsed_samples: 0,
			sample_rate_f32: sample_rate as f32,
		}
	} 
	pub fn sample_rate(&self) -> i32 { self.sample_rate }
	pub fn elapsed_samples(&self) -> SampleCount { self.elapsed_samples }
	pub fn sample_rate_f32(&self) -> f32 { self.sample_rate_f32 }

	pub fn sample_elapsed(&mut self) { self.elapsed_samples += 1; }
}
