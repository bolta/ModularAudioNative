use crate::core::{
	event::*,
	machine::*,
	node::*,
	context::Context as CoreContext,
};
use crate::node::{
	env::NoteEvent,
	var::*,
};
use super::{
	common::*,
	context::*,
	instruction::*,
	tick::EVENT_TYPE_TICK,
	sequence::*,
};

pub struct Sequencer {
	sequences: Vec<Sequence>,
	// TODO 今後 context は任意個になる予定
	context: Context,
}
impl Sequencer {
	pub fn new(sequences: Vec<Sequence>) -> Self {
		Self {
			sequences,
			context: Context {
				// ↓上の sequence と同時に初期化する方法がわからないのでとりあえず index で持つ
				// sequence: self.sequence,
				seq_idx: SequenceIndex(0),
				instrc_idx: InstructionIndex(0),
				wait: 0,
			}
		}
	}
}

impl Node for Sequencer {
	fn process_event(&mut self, event: &dyn Event, context: &CoreContext, env: &mut Environment) {
		if event.event_type() != EVENT_TYPE_TICK { return; }

//		println!("tick at sample {}", context.elapsed_samples());
		self.context.tick(&self.sequences[self.context.seq_idx.0], context, env);
	}
}

struct Context {
//	sequence: &'a Sequence,
	seq_idx: SequenceIndex,
	instrc_idx: InstructionIndex,
	wait: i32,
}
impl Context {
	fn tick(&mut self, sequence: &Sequence, context: &CoreContext, env: &mut Environment) {
		if self.wait > 0 {
			self.wait -= 1;
			if self.wait > 0 { return; }
		}

		// ウェイトを挟まずに並んでいるインストラクションは全て実行する
		while self.wait == 0 && self.instrc_idx.0 < sequence.len() {
			match &sequence[self.instrc_idx.0] {
				Instruction::Note { tag, note_on } => {
					env.events_mut().push(Box::new(NoteEvent::new(EventTarget::Tag(tag.clone()), *note_on)));
				}
				Instruction::Value { tag, value } => {
					// TODO キューが一杯だったときの処理
					env.events_mut().push(Box::new(SetEvent::new(EventTarget::Tag(tag.clone()), *value)));
				}
				Instruction::Wait(wait) => {
					self.wait = *wait;
				}

			}
			self.instrc_idx.0 += 1;
		}
	}
}
