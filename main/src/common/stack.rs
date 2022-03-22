/// 言語処理系で使うためのスタック。
/// 初期状態が必ずある
#[derive(Debug)]
pub struct Stack<T: Clone> {
	stack: Vec<T>,
}
impl <T: Clone> Stack<T> {
	pub fn init(init: T) -> Self { Self { stack: vec![init] }}
	pub fn push_clone(&mut self) {
		let new_frame = self.top().clone();
		self.stack.push(new_frame);
	}
	pub fn pop(&mut self) {
		if self.is_bottom() {
			debug_assert!(false);
			return;
		}
		self.stack.pop();
	}
	pub fn top(&self) -> &T {
		let len = self.stack.len();
		&self.stack[len - 1]
	}
	pub fn top_mut(&mut self) -> &mut T {
		let len = self.stack.len();
		&mut self.stack[len - 1]
	}
	pub fn is_bottom(&self) -> bool { self.stack.len() == 1 }
}
