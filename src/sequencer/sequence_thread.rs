use std::rc::Rc;
use super::sequence::*;
use super::sequencer::*;

pub struct SequenceThread<'a> {
	//sequencer: Rc<Sequencer>,

	/** 現在のシーケンス */
	sequence: &'a Sequence<'a>, //Rc<Sequence<'a>>,

	/** これから実行するインストラクションの添字 */
	pointer: usize,

	/**
	 * 待機中の場合、正の値（tick 単位）。
	 * 0 のときは直ちに次のインストラクションを実行できる
	 */
	wait: i32,
}

impl<'a> SequenceThread<'a> {
	pub fn new(sequence: &'a Sequence<'a>/*Rc<Sequence>*/) -> Self {
		Self {
			sequence,
			pointer: 0,
			wait: 0,
		}
	}

	pub fn tick(&mut self) {
		if self.wait > 0 {
			self.wait -= 1;
			// TODO これ >= 0 ではないだろうか？
			if self.wait > 0 { return; }
		}

		while self.wait == 0 && self.pointer < self.sequence.count() {
			self.sequence.at(self.pointer).execute(self);
			self.pointer += 1;
		}
	}

	pub fn wait(&self) -> i32 { self.wait }
	pub fn set_wait(&mut self, wait: i32) { self.wait = wait; }

	// pub fn notable_by_name(&self, name: &str) -> &dyn Notable {}
}
