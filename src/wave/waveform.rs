use crate::{
	core::common::*,
};

pub struct Waveform {
	channels: i32,
	sample_rate: i32,
	data: Vec<Sample>,
	master_freq: f32,
	start_offset: f32,
	end_offset: Option<f32>,
	loop_offset: Option<f32>,
}
impl Waveform {
	pub fn new(channels: i32, sample_rate: i32, data: Vec<Sample>) -> Self {
		Self::new_with_details(channels, sample_rate, data, None, None, None, None)
	}
	pub fn new_with_details(
		channels: i32,
		sample_rate: i32,
		data: Vec<Sample>,
		// TODO これ以下の情報はここに持つべきなのか？
		// （波形の扱い方の規定であって、波形そのものではないので）
		// （ただし、WaveformPlayer で波形を切り替える際に波形の index の指定だけで実現できると都合がいいので
		// 便宜上ここに置いている）
		master_freq: Option<f32>,
		start_offset: Option<f32>,
		end_offset: Option<f32>,
		loop_offset: Option<f32>,
	) -> Self {
		Self {
			channels,
			sample_rate,
			data,
			master_freq: master_freq.unwrap_or(261.6255653005986f32), // o4c
			start_offset: start_offset.unwrap_or(0f32),
			end_offset,
			loop_offset,
		}
	}

	pub fn channels(&self) -> i32 { self.channels }
	pub fn sample_rate(&self) -> i32 { self.sample_rate }
	pub fn master_freq(&self) -> f32 { self.master_freq }
	pub fn start_offset(&self) -> f32 { self.start_offset }
	pub fn end_offset(&self) -> Option<f32> { self.end_offset }
	pub fn loop_offset(&self) -> Option<f32> { self.loop_offset }

	pub fn len(&self) -> usize { self.data.len() / self.channels as usize }
	pub fn sample(&self, channel: i32, offset: usize) -> Sample {
		self.data[self.channels as usize * offset + channel as usize]
	}

}
