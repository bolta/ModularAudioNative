use crate::core::{
	common::*,
	context::*,
	event::*,
	machine::*,
	node::*,
};
use node_macro::node_impl;

/// システム上の「ジョブ」の稼働状況を監視し、全てのジョブが終了したら（TODO かつ、無音になったら）演奏を終了するノード。
/// ジョブとは今のところ SequenceThread のことだが、このしくみはそれ以外にも使えるので、一般的な概念として「ジョブ」と呼ぶようにする。
/// ジョブとして何かを稼働させたいときは、稼働開始時に JobStarting、終了時に JobEnded を投げることで、
/// 稼働終了を待って演奏が終了するようになる（他から演奏が終了されない限り）。
pub struct Terminator {
	input: ChanneledNodeIndex,
	thread_count: i32,
}
impl Terminator {
	pub fn new(input: ChanneledNodeIndex) -> Self {
		Self {
			input,
			thread_count: 0,
		}
	}
}
#[node_impl]
impl Node for Terminator {
	fn channels(&self) -> i32 { 0 }
	fn upstreams(&self) -> Upstreams { vec![self.input] }
	fn execute(&mut self, inputs: &Vec<Sample>, _output: &mut Vec<Sample>, _context: &Context, _env: &mut Environment) {
		// TODO 無音検知
	}
	fn process_event(&mut self, event: &dyn Event, _context: &Context, env: &mut Environment) {
		if event.event_type() == EVENT_TYPE_JOB_STARTING {
			println!("{} -> job starting", self.thread_count);
			self.thread_count += 1;
		}
		if event.event_type() == EVENT_TYPE_JOB_ENDED {
			self.thread_count -= 1;
			println!("job ended -> {}", self.thread_count);
		}
		// TODO 無音が続いていたら、も追加
		if self.thread_count <= 0 {
			env.events_mut().push(Box::new(TerminateEvent { }));
		}
}
}

pub struct JobEvent {
	target: EventTarget,
	event_type: &'static str,
}
impl JobEvent {
	pub fn starting() -> Self {
		JobEvent {
			// TODO グローバルな名前だが、こんなのでいいか？
			target: EventTarget::Tag("terminator".to_string()),
			event_type: EVENT_TYPE_JOB_STARTING,
		}
	}
	pub fn ended() -> Self {
		JobEvent {
			target: EventTarget::Tag("terminator".to_string()),
			event_type: EVENT_TYPE_JOB_ENDED,
		}
	}
}
impl Event for JobEvent {
	fn target(&self) -> &EventTarget { &self.target }
	fn event_type(&self) -> &str { self.event_type }
}

pub const EVENT_TYPE_JOB_STARTING: &str = "System::JobStarting";
pub const EVENT_TYPE_JOB_ENDED: &str = "System::JobEnded";
