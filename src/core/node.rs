use super::signal::*;

pub struct Node {
	current: f32,
	iterator: Box<dyn Iterator<Item = f32>>,
	name: Option<String>,
}

impl Node {
	pub fn from_closure(closure: Box<dyn FnMut () -> Option<f32>>) -> Self {
		Self::from_closure_named_or_not(None, closure)
	}

	pub fn from_closure_named(name: String, closure: Box<dyn FnMut () -> Option<f32>>) -> Self {
		Self::from_closure_named_or_not(Some(name), closure)

	}
	fn from_closure_named_or_not(name: Option<String>, closure: Box<dyn FnMut () -> Option<f32>>) -> Self {
		Self {
			current: 0f32,
			iterator: Box::new(ClosureIterator { closure }),
			name,
		}
	}

	// TODO str にすべき？（よくわかっていない）
	pub fn name(&self) -> Option<&String> { (& self.name).as_ref() }
	pub fn id(&self) -> String { format!("{:p}", self ) }
	pub fn id_name(&self) -> String { format!("{}:{}", self.id(), self.name().unwrap_or(&String::from(""))) }

	pub fn update(&mut self) {
		self.current = match self.iterator.next() {
			Some(v) => v,
			None => 0f32,
		};
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
