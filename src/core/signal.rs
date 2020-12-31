pub struct Signal {
	// TODO Option<f32> のまま持った方がいいのかな？　現状は C# 版に近づけているが
	current: f32,
	iterator: Box<dyn Iterator<Item = f32>>,
}

impl Signal {
	pub fn new(iterator: Box<dyn Iterator<Item = f32>>) -> Self {
		Self {
			current: 0f32,
			iterator
		}
	}

	pub fn from_closure(closure: Box<dyn FnMut () -> Option<f32>>) -> Self {
		Self::new(Box::new(ClosureIterator { closure }))
	}

	pub fn update(&mut self) {
		self.current = match self.iterator.next() {
			Some(v) => v,
			None => 0f32,
		}
	}

	pub fn value(&self) -> f32 { self.current }
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
