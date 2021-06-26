use std::collections::HashMap;

use super::sequence::*;

pub trait SequenceGenerator {
	fn generate_sequences(&self, ticks_per_beat: i32/*, temper: Temperament*/) -> HashMap<String, Sequence>;
}
