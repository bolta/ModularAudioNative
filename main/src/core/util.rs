use crate::core::common::*;

pub fn vec_with_length(len: usize) -> Vec<Sample> {
	let mut result = Vec::with_capacity(len);
	for _ in 0 .. len { result.push(0f32); }
	result
}
