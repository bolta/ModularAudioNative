use std::{
	default::Default,
	ops::Index,
};

pub struct DelayBuffer<T: Clone + Default> {
	buffer: Vec<T>,
	head: usize,
}
impl <T: Clone + Default> DelayBuffer<T> {
	pub fn new(size: usize) -> Self {
		Self {
			buffer: vec![Default::default(); size],
			head: 0usize,
		}
	}
	pub fn len(&self) -> usize { self.buffer.len() }
	pub fn push(&mut self, value: T) {
		self.head = (self.head + 1) % self.buffer.len();
		self.buffer[self.head] = value;
	}
}
impl <T: Clone + Default> Index<i32> for DelayBuffer<T> {
	type Output = T;
	fn index(&self, offset: i32) -> &Self::Output {
		if offset <= -(self.buffer.len() as i32) || 0 < offset {
			panic!("offset must satisfy -size < offset <= 0");
		}

		return & self.buffer[(self.head + self.buffer.len() - (-offset as usize)) % self.buffer.len()];
	}
}
