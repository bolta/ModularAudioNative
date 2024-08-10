use super::{
	waveform::*,
};
use crate::{
	core::common::*,
};

extern crate wav;
use wav::bit_depth::BitDepth;

use std::{
	fs::File,
	io,
};

pub fn read_wav_file(
	wav_path: &str,
	sample_rate: Option<i32>,
	master_freq: Option<f32>,
	start_offset: Option<f32>,
	end_offset: Option<f32>,
	loop_offset: Option<f32>,
) -> io::Result<Waveform> {
	let mut file = File::open(wav_path) ?;
	let (header, wav_data) = wav::read(&mut file) ?;

	let data = convert_data(&wav_data);

	Ok(Waveform::new_with_details(
		header.channel_count as i32,
		sample_rate.unwrap_or_else(|| header.sampling_rate as i32),
		data,
		master_freq,
		start_offset,
		end_offset,
		loop_offset,
	))
}

fn convert_data(wav_data: &BitDepth) -> Vec<Sample> {
	let dest_min = -1f32;
	let dest_max =  1f32;
	let dest_range = dest_max - dest_min;
	match wav_data {
		BitDepth::Eight(wav_data) => {
			let src_min = u8::MIN as f32 + 1f32; // TODO MIN と MIN + 1 は同一視、でいい？
			let src_range = u8::MAX as f32 + 1f32 - src_min; // TODO 上端の処理これでいいか？

			wav_data.iter().map(|w| ((*w as f32 - src_min) as f32 * dest_range / src_range + dest_min).max(dest_min).min(dest_max)).collect()
		}
		BitDepth::Sixteen(wav_data) => {
			let src_min = i16::MIN as f32 + 1f32; // TODO MIN と MIN + 1 は同一視、でいい？
			let src_range = i16::MAX as f32 + 1f32 - src_min; // TODO 上端の処理これでいいか？

			wav_data.iter().map(|w| (((*w as f32) - src_min) as f32 * dest_range / src_range + dest_min).max(dest_min).min(dest_max)).collect()
		}
		BitDepth::TwentyFour(wav_data) => {
			let src_min = i32::MIN as f32 + 1f32; // TODO MIN と MIN + 1 は同一視、でいい？
			let src_range = i32::MAX as f32 + 1f32 - src_min; // TODO 上端の処理これでいいか？

			wav_data.iter().map(|w| ((*w as f32 - src_min) as f32 * dest_range / src_range + dest_min).max(dest_min).min(dest_max)).collect()
		}
		BitDepth::ThirtyTwoFloat(wav_data) => {
			// > 浮動小数点数で格納される場合、慣習からデータ値の範囲は-1.0から+1.0に限られる
			// https://ja.wikipedia.org/wiki/WAV
			// とのことだが、保証しておく
			wav_data.iter().map(|w| w.max(dest_min).min(dest_max)).collect()
		}
		// なんだこれ？
		BitDepth::Empty => {
			vec![]
		}
	}
}
