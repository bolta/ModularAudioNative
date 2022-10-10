use super::{
	waveform::*,
};
use std::{
	ops::{
		Index,
		IndexMut,
	},
};

/// WaveformHost における Waveform の添字。
/// 単なる添字なので出力チャンネル数の情報は持たない
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WaveformIndex(pub usize);

pub struct WaveformHost {
	waveforms: Vec<Waveform>
}
impl WaveformHost {
	pub fn new() -> Self { Self { waveforms: vec![] } }
	pub fn add(&mut self, waveform: Waveform) -> WaveformIndex {
		self.waveforms.push(waveform);
		WaveformIndex(self.waveforms.len() - 1)
	}
}
impl Index<WaveformIndex> for WaveformHost {
	type Output = Waveform;
	fn index(&self, idx: WaveformIndex) -> &Self::Output {
		&self.waveforms[idx.0]
	}
}
impl IndexMut<WaveformIndex> for WaveformHost {
	fn index_mut(&mut self, idx: WaveformIndex) -> &mut Self::Output {
		&mut self.waveforms[idx.0]
	}
}
