pub type Sample = f32;
pub type SampleCount = i32;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeIndex(pub usize);

pub const TWO_PI: f32 = 2_f32 * std::f32::consts::PI;

pub const NO_OUTPUT: Sample = f32::NAN;
