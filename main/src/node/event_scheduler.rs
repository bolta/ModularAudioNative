use crate::common::util::ignore_errors;

use crate::core::{
	common::*,
	context::*,
	event::*,
	machine::*,
	node::*,
};
use node_macro::node_impl;

use std::cmp::min;
use std::collections::BTreeMap;

pub struct EventScheduler {
	base_: NodeBase,
	events: BTreeMap<SampleCount, Vec<Box<dyn Event>>>,
	next_key: Option<SampleCount>,
}
impl EventScheduler {
	pub fn new(base: NodeBase) -> Self {
		Self {
			base_: base,
			events: BTreeMap::new(),
			next_key: None, // TODO None の代わりにとても大きい値でもいいかも（Option が不要になる）
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
	fn process_events(&mut self, elapsed_samples: SampleCount, prod: &mut EventProducer) {
		if self.next_key.is_none() { return; }
		let next_key = self.next_key.unwrap();
		if elapsed_samples < next_key { return; }

		let keys_to_remove: Vec<SampleCount> = self.events.keys().take_while(|k| **k <= elapsed_samples)
				.map(|k| *k)
				.collect();
		for key in keys_to_remove {
			let events_at_key = self.events.remove(&key).unwrap();
			for e in events_at_key {
				// TODO キューが一杯だったときの処理
				ignore_errors(prod.push(e));
			}
		}
		self.next_key = self.events.keys().next().map(|k| *k);
	}
}
#[node_impl]
impl Node for EventScheduler {
	fn channels(&self) -> i32 { 0 }
	fn upstreams(&self) -> Upstreams { vec![] }
	fn activeness(&self) -> Activeness { Activeness::Active } // TODO でいいのかな？
	fn initialize(&mut self, _context: &Context, env: &mut Environment) {
		self.process_events(0, env.events_mut());
	}
	fn update(&mut self, _inputs: &Vec<Sample>, context: &Context, env: &mut Environment) {
		self.process_events(context.elapsed_samples() + 1, env.events_mut());
	}
}
