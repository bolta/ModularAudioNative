use std::collections::HashMap;

use crate::{
	core::{
		common::*,
		context::*,
		event::*,
		machine::*,
		node::*,
		node_factory::*,
	},
	node::envelope::*,
	wave::{
		waveform::*,
		waveform_host::*,
	},
};
use node_macro::node_impl;

use super::var::{SetEvent, EVENT_TYPE_SET};

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
	fn activeness(&self) -> Activeness { Activeness::Active }
	fn execute(&mut self, _inputs: &Vec<Sample>, output: &mut [Sample], _context: &Context, env: &mut Environment) {
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
		// freq == waveform.original_freq() && waveform.sample_rate() == context.sample_rate_f32() のとき、等速（1 サンプル進む）
		// そこから freq と waveform.sample_rate() に比例して速くなる
		self.offset += 1f32 * freq * waveform.sample_rate() as f32 / waveform.original_freq() / context.sample_rate_f32();
		let end_offset = waveform.end_offset().unwrap_or_else(|| waveform.len() as f32);
		if self.offset >= end_offset {
			match waveform.loop_offset() {
				Some(loop_offset) => {
					self.offset -= end_offset - loop_offset;
				},
				None => {
					self.state = WaveformPlayerState::Idle;
				},
			}
		}
	}

	fn process_event(&mut self, event: &dyn Event, _context: &Context, env: &mut Environment) {
		match event.event_type() {
			EVENT_TYPE_NOTE => {
				let event = event.downcast_ref::<NoteEvent>().unwrap();
				if event.note_on() {
					self.state = WaveformPlayerState::Note;
					// TODO WaveformHost の範囲チェック、どこに入れるか
					self.offset = env.waveforms()[self.index].start_offset();
				} else {
					// リリースを発音する場合があるのでノートオフは無視する
					// self.state = WaveformPlayerState::Idle;
				}
			},

			EVENT_TYPE_SET => {
				let event = event.downcast_ref::<SetEvent>().unwrap();
				if event.key() == "waveform" {
					self.index = WaveformIndex(event.value() as usize);
				}
			}

			_ => { },
		}
	}
}

#[derive(Eq, PartialEq)] enum WaveformPlayerState { Idle, Note }

pub struct WaveformPlayerFactory {
	waveform_index: WaveformIndex,
}
impl WaveformPlayerFactory {
	pub fn new(waveform_index: WaveformIndex) -> Self {
		Self { waveform_index }
	}
}
impl NodeFactory for WaveformPlayerFactory {
	fn node_arg_specs(&self) -> Vec<NodeArgSpec> { vec![] }
	fn input_channels(&self) -> i32 { 1 }
	fn default_prop_key(&self) -> Option<String> { Some("waveform".to_string()) }
	fn initial_values(&self) -> HashMap<String, Sample> {
		vec![
			("waveform".to_string(), self.waveform_index.0 as Sample)
		].into_iter().collect()
	}
	fn create_node(&self, _node_args: &NodeArgs, piped_upstream: ChanneledNodeIndex) -> Box<dyn Node> {
		let freq = piped_upstream.as_mono();
		Box::new(WaveformPlayer::new(1, self.waveform_index, freq))
	}
}
