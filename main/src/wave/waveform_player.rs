use super::{
	waveform::*,
	waveform_host::*,
};

use crate::{
	core::{
		common::*,
		context::*,
		event::*,
		machine::*,
		node::*,
	},
	node::env::*,
};
use node_macro::node_impl;

pub struct WaveformPlayer {
	// TODO 波形のチャンネル数と照合
	// ステレオの player でモノラルの波形を読む場合はステレオに拡張するとして、
	// 逆の場合はエラー？
	channels: i32,
	index: WaveformIndex,
	freq: MonoNodeIndex,
	state: WaveformPlayerState,
	offset: f32,
}
impl WaveformPlayer {
	pub fn new(channels: i32, index: WaveformIndex, freq: MonoNodeIndex) -> Self {
		Self {
			channels,
			index,
			freq,
			state: WaveformPlayerState::Idle,
			offset: 0f32,
		}
	}

	fn waveform<'a>(&self, env: &'a Environment) -> &'a Waveform { & env.waveforms()[self.index] }
}
#[node_impl]
impl Node for WaveformPlayer {
	fn channels(&self) -> i32 { self.channels }
	fn upstreams(&self) -> Upstreams { vec![self.freq.channeled()] }
	fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut Vec<Sample>, _context: &Context, env: &mut Environment) {
		match self.state {
			WaveformPlayerState::Note => {
				let waveform = self.waveform(env);
				for ch in 0usize .. self.channels as usize {
					// TODO 補間
					output[ch] = waveform.sample(ch as i32, self.offset as usize);
				}
			}
			WaveformPlayerState::Idle => {
				for ch in 0usize .. self.channels as usize {
					output[ch] = 0f32;
				}
			}
		}
	}
	fn update(&mut self, inputs: &Vec<Sample>, context: &Context, env: &mut Environment) {
		if self.state != WaveformPlayerState::Note { return; }

		let freq = inputs[0];
		let waveform = self.waveform(env);
		// freq == waveform.master_freq() && waveform.sample_rate() == context.sample_rate_f32() のとき、等速（1 サンプル進む）
		// そこから freq と waveform.sample_rate() に比例して速くなる
		// TODO ループ対応
		self.offset += 1f32 * freq * waveform.sample_rate() as f32 / waveform.master_freq() / context.sample_rate_f32();
		if self.offset >= waveform.len() as f32 {
			self.state = WaveformPlayerState::Idle;
		}
	}

	fn process_event(&mut self, event: &dyn Event, _context: &Context, env: &mut Environment) {
		// TODO 波形を切り替えるイベント

		if event.event_type() != EVENT_TYPE_NOTE { return; }

		let event = event.downcast_ref::<NoteEvent>().unwrap();
		if event.note_on() {
			self.state = WaveformPlayerState::Note;
			// TODO WaveformHost の範囲チェック、どこに入れるか
			self.offset = env.waveforms()[self.index].start_offset();
		} else {
			self.state = WaveformPlayerState::Idle;
		}
	}
}

#[derive(Eq, PartialEq)] enum WaveformPlayerState { Idle, Note }
