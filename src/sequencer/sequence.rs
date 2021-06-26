use std::rc::Rc;

use super::instruction::*;

pub struct Sequence<'a> {
	instructions: Vec<Instruction<'a>>,
}

impl<'a> Sequence<'a> {
	pub fn new(instructions: Vec<Instruction<'a>>) -> Self {
		Self {
			instructions,
		}
	}

	pub fn count(&self) -> usize { self.instructions.len() }

	// TODO Rust で indexer は書けるのか、書けるならどう書くのか
	pub fn at(&self, index: usize) -> /*Rc<Instruction>*/ &'a Instruction {
	//	Rc::clone(& self.instructions[index])

	& self.instructions[index]
	}
}
