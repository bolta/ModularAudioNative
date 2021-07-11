use crate::core::{
	common::*,
	event::*,
	machine::*,
	node::*,
};

use std::cmp::min;
use std::collections::BTreeMap;

pub struct EventScheduler {
	events: BTreeMap<SampleCount, Vec<Box<dyn Event>>>,
	next_key: Option<SampleCount>,
}
impl EventScheduler {
	pub fn new() -> Self {
		Self {
			events: BTreeMap::new(),
			next_key: None,
		}
	}
	pub fn add_event(&mut self, elapsed_samples: SampleCount, event: Box<dyn Event>) {
		match self.events.get_mut(&elapsed_samples) {
			Some(es) => { es.push(event); }
			None => { self.events.insert(elapsed_samples, vec![event]); }
		}

		let next = match self.next_key {
			Some(n) => min(n, elapsed_samples),
			None => elapsed_samples,
		};
		self.next_key = Some(next);
	}
}
impl Node for EventScheduler {
	fn upstreams(&self) -> Vec<NodeIndex> { vec![] }
	fn execute(&mut self, _inputs: &Vec<Sample>, machine: &mut Machine) -> Sample {
		// TODO machine に対して次のことができる必要：
		// * 経過サンプル数をもらう
		// * post_event を呼ぶ
		NO_OUTPUT
	}
}
