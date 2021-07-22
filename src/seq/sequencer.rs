use crate::core::{
	event::*,
	machine::*,
	node::*,
	context::Context as CoreContext,
};
use super::{
	common::*,
	context::*,
	instruction::*,
	tick::EVENT_TYPE_TICK,
	sequence::*,
};

pub struct Sequencer/* <'a> */ {
	// TODO 今後 sequence と context はともに任意個になる
	sequence: Sequence,
	context: Context/* <'a> */,
}
impl /* <'a> */ Sequencer/* <'a> */ {
	pub fn new(sequence: Sequence) -> Self {
		Self {
			sequence,
			context: Context {
				// ↓上の sequence と同時に初期化する方法がわからないのでとりあえず index で持つ
				// sequence: self.sequence,
				sequence: SequenceIndex(0),
				instruction: InstructionIndex(0),
			}
		}
	}
}

impl /* <'a> */ Node for Sequencer/* <'a> */ {
	// fn upstreams(&self) -> 
	fn process_event(&mut self, event: &dyn Event, context: &CoreContext, env: &mut Environment) {
		if event.event_type() != EVENT_TYPE_TICK { return; }

		println!("tick at sample {}", context.elapsed_samples());
	}
}

struct Context/* <'a> */ {
//	sequence: &'a Sequence,
	sequence: SequenceIndex,
	instruction: InstructionIndex,
}
// impl <'a> Context<'a> {
// 	pub fn new(sequence: &'a Sequence) -> Self {
// 		Self {
// 			sequence,
// 			index: InstructionIndex(0),
// 		}
// 	}
// }
