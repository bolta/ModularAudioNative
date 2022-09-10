use crate::common;
use crate::core::{
	event::*,
	machine::*,
	node::*,
	context::Context as CoreContext,
};
use crate::node::{
	envelope::NoteEvent,
	system::*,
	var::*,
};
use super::{
	common::*,
	instruction::*,
	tick::EVENT_TYPE_TICK,
	sequence::*,
};
use node_macro::node_impl;

use std::{
	collections::hash_map::HashMap,
};

pub struct Sequencer {
	sequences: HashMap<String, Sequence>,
	// TODO 今後 context は任意個になる予定
	context: Context,
}
impl Sequencer {
	pub fn new(sequences: HashMap<String, Sequence>) -> Self {
		Self {
			sequences,
			context: Context {
				stack: Stack::init(StackFrame {
					seq_idx: SequenceName(SEQUENCE_NAME_MAIN.to_string()),
					instrc_idx: 0,
					vars: Vars::new(),
				}),
				wait: 0,
			},
		}
	}
}

#[node_impl]
impl Node for Sequencer {
	fn channels(&self) -> i32 { 0 }
	fn upstreams(&self) -> Upstreams { vec![] }
	fn activeness(&self) -> Activeness { Activeness::Static } // TODO execute と update はないので Static、でいいのかな？
	fn process_event(&mut self, event: &dyn Event, context: &CoreContext, env: &mut Environment) {
		if event.event_type() != EVENT_TYPE_TICK { return; }

//		println!("tick at sample {}", context.elapsed_samples());
		self.context.tick(&mut self.sequences, context, env);
	}
}

// TODO 他の型の変数もほしいかも…
type Vars = HashMap<String, i32>;
type Stack = common::stack::Stack<StackFrame>;
#[derive(Clone, Debug)]
struct StackFrame {
	seq_idx: SequenceName,
	instrc_idx: i32, // 一時的に -1 にする必要があるので符号つきで持つ
	vars: Vars,
}

struct Context {
	stack: Stack,
	wait: i32,
}
impl Context {
	fn tick(&mut self, sequences: &mut HashMap<String, Sequence>, _context: &CoreContext, env: &mut Environment) {
		if self.wait > 0 {
			self.wait -= 1;
			if self.wait > 0 { return; }
		}

		// ウェイトを挟まずに並んでいるインストラクションは全て実行する
		while self.wait == 0 {
			let instrc_idx = self.stack.top_mut().instrc_idx as usize;
			// TODO 無限ループで先頭に戻ったときも開始扱いになってしまう
			if instrc_idx == 0usize && self.stack.is_bottom() {
				env.post_event(Box::new(JobEvent::starting()));
			}
			// TODO 毎回ハッシュテーブルを引くと遅いか？
			let mut sequence = sequences.get(& self.stack.top().seq_idx.0).unwrap();
			if instrc_idx >= sequence.len() { return; }

			self.process_instruction(&sequence[instrc_idx], env);

			// 次に実行するインストラクションを求める
			loop {
				self.stack.top_mut().instrc_idx += 1;
				debug_assert!(self.stack.top_mut().instrc_idx >= 0);
				if (self.stack.top().instrc_idx as usize) < sequence.len() { break; } // 今のシーケンスに続きがある

				// シーケンスの終わりに達した
				if self.stack.is_bottom() {
					env.post_event(Box::new(JobEvent::ended()));
					break; // 曲が終わった。次回の tick からは何もしない
				} else {
					self.stack.pop(); // 呼び出し元の続きに復帰
					sequence = sequences.get(& self.stack.top().seq_idx.0).unwrap();
				}
			}

		}
	}

	fn process_instruction(&mut self, instrc: &Instruction, env: &mut Environment) {
		match instrc {
			Instruction::Nop => {
				// nop
			}
			Instruction::Note { tag, note_on } => {
				env.post_event(Box::new(NoteEvent::new(EventTarget::Tag(tag.clone()), *note_on)));
			}
			Instruction::Value { tag, value } => {
				env.post_event(Box::new(SetEvent::new(EventTarget::Tag(tag.clone()), *value)));
			}
			Instruction::Wait(wait) => {
				self.wait = *wait;
			}
			Instruction::NewVar { name, value } => {
				// TODO 重複はエラー
				self.stack.top_mut().vars.insert(name.clone(), *value);
			}
			Instruction::DecrVar { name } => {
				if let Some(value) = self.stack.top_mut().vars.get_mut(name) {
					*value -= 1;
				} else {
					// TODO エラー
				};
			}
			Instruction::DeleteVar { name } => {
				self.stack.top_mut().vars.remove(name);
			}
			Instruction::Call { seq_name } => {
				self.stack.push_clone();
				let mut new_top = self.stack.top_mut();
				new_top.seq_idx = SequenceName(seq_name.clone());
				new_top.instrc_idx = -1; // この後インクリメントされるので 1 引いておく
			}
			Instruction::JumpAbs { seq_name, pos } => {
				let mut top = self.stack.top_mut();
				if let Some(seq_name) = seq_name { top.seq_idx = SequenceName(seq_name.clone()); }
				top.instrc_idx = pos.0 as i32 - 1; // この後インクリメントされるので 1 引いておく
			}
			Instruction::JumpRel { offset } => {
				let mut top = self.stack.top_mut();
				top.instrc_idx = top.instrc_idx + offset - 1; // この後インクリメントされるので 1 引いておく
			}
			Instruction::If0 { var, then } => {
				if let Some(0) = self.stack.top().vars.get(var.as_str()) {
					self.process_instruction(then, env);
				}
			}
			Instruction::EnterSkipMode => {
				env.post_event(Box::new(EnterSkipModeEvent { }));
			}
			Instruction::ExitSkipMode => {
				env.post_event(Box::new(ExitSkipModeEvent { }));
			}
		}
	}
}

