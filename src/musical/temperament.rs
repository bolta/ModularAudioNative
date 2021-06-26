use super::tone::*;

use std::collections::HashMap;

pub trait Temperament<T> {
	fn freq(&self, tone: T) -> f32;
}

static TONE_NAME_TO_SEMITONE: HashMap<BaseName, i32> = (|| {
	let mut m = HashMap::<_, _>::new();
	m.insert(BaseName::C, -9);
	m.insert(BaseName::D, -7);
	m.insert(BaseName::E, -5);
	m.insert(BaseName::F, -4);
	m.insert(BaseName::G, -2);
	m.insert(BaseName::A,  0);
	m.insert(BaseName::B,  2);

	m
})();

pub struct EqualTemperament {
	a4: f32,
}
impl EqualTemperament {
	pub fn new() -> Self { Self::tuned(440f32) }
	pub fn tuned(a4: f32) -> Self { Self { a4 } }
}
impl Temperament<Tone> for EqualTemperament {
	fn freq(&self, tone: Tone) -> f32 {
		let base_semitone = TONE_NAME_TO_SEMITONE.get(& tone.base_name);

		match base_semitone {
			None => 0f32,
			Some(s) => self.a4 * 2f32.powf((tone.octave - 4) as f32 + (s + tone.accidental) as f32 / 12f32),
		}
	}
}
