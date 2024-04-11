use std::collections::HashMap;

use crate::wave::waveform_host::WaveformHost;

use super::value::Value;

pub struct ImportCache<'a> {
	// TODO String じゃなくて Path とか他の型になるのかも？
	pub imports: HashMap<String, Value>,
	pub waveforms: &'a mut WaveformHost,
}
impl <'a> ImportCache<'a> {
	pub fn new(waveforms: &'a mut WaveformHost) -> Self {
		Self {
			imports: HashMap::new(),
			waveforms,
		}
	}
}
