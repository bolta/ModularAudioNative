pub type Sample = f32;
pub type SampleCount = i32;

#[derive(Clone, Copy)]
pub struct NodeIndex(pub usize);

pub const TWO_PI: f32 = 2_f32 * std::f32::consts::PI;

pub const CHANNELS: i32 = 1; // TODO どこに置こう？
pub const SAMPLE_RATE: i32 = 44100; // TODO どこに置こう？
pub const SAMPLE_RATE_F32: f32 = SAMPLE_RATE as f32; // TODO どこに置こう？

pub const NO_OUTPUT: Sample = f32::NAN;
