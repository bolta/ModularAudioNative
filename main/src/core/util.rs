use crate::core::common::*;

pub fn sample_vec_with_length(len: usize) -> Vec<Sample> {
	vec_with_length_and_init_value(len, |_| 0f32)
}

pub fn vec_with_length_and_init_value<T, F>(len: usize, make_elem: F) -> Vec<T>
where F: Fn (usize) -> T {
	let mut result = Vec::with_capacity(len);
	for i in 0 .. len { result.push(make_elem(i)); }
	result
}

// TODO 共通化する
pub fn is_true(value: f32) -> bool { value > 0f32 }
