use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::sequence::*;
use super::sequence_thread::*;
use super::tick::*;

pub const MAIN_SEQUENCE_KEY: &str = "#main";

pub struct Sequencer<'a> {
	// TODO paramerets

	sequences: HashMap<String, Sequence<'a>>,
	threads: Vec<SequenceThread<'a>>,
}

impl<'a> Sequencer<'a> {
	fn new(tick: &mut Tick, /* TODO parameters */ sequences: HashMap<String, Sequence<'a>>) -> Self/*Rc<RefCell<Self>>*/ {
		let result = Rc::new(RefCell::new(Self {
			sequences,
			threads: vec![],
		}));
		let clone = Rc::clone(&result);
		tick.add_user(clone);

		result
	}
}

impl<'a> TickUser for Sequencer<'a> {
	fn tick(&mut self) {
		for t in self.threads.iter_mut() { t.tick(); }
	}
}
