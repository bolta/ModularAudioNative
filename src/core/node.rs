use super::signal::*;

pub struct Node {
	current: f32,
	iterator: Box<dyn Iterator<Item = f32>>,
	// closure: 
}

impl Node {
	pub fn from_closure(closure: Box<dyn FnMut () -> Option<f32>>) -> Self {
		Self {
			current: 0f32,
			iterator: Box::new(ClosureIterator { closure }),
		}
	}

	pub fn update(&mut self) {
		self.current = match self.iterator.next() {
			Some(v) => v,
			None => 0f32,
		}
	}

	pub fn current(&self) -> f32 { self.current }
}

struct ClosureIterator {
	closure: Box<dyn FnMut () -> Option<f32>>,
}
impl Iterator for ClosureIterator {
	type Item = f32;
	fn next(&mut self) -> Option<Self::Item> {
		(self.closure)()
	}
}
