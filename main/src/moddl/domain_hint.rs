#[derive(Clone)]
pub enum DomainHint {
	Range {
		min: f32,
		includes_min: bool,
		max: f32,
		includes_max: bool,
	},
	Enum {
		items: Vec<EnumItem>,
	},
}
impl DomainHint {
	pub fn range_including_no_ends(min: f32, max: f32) -> Self {
		Self::Range { min, max, includes_min: false, includes_max: false }
	}
	pub fn range_including_min_end(min: f32, max: f32) -> Self {
		Self::Range { min, max, includes_min: true, includes_max: false }
	}
	pub fn range_including_max_end(min: f32, max: f32) -> Self {
		Self::Range { min, max, includes_min: false, includes_max: true }
	}
	pub fn range_including_both_ends(min: f32, max: f32) -> Self {
		Self::Range { min, max, includes_min: true, includes_max: true }
	}
}

#[derive(Clone)]
pub struct EnumItem { value: f32, name: String }
