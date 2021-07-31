pub enum Value {
	Float(f32),
	TrackSet(Vec<String>),
	/// IdentifierLiteral（`foo`）の評価結果としての値
	Identifier(String),
}

impl Value {
	pub fn as_float(&self) -> Option<f32> {
		match self {
			Self::Float(value) => Some(*value),
			_ => None,
		}
	}
	pub fn as_track_set(&self) -> Option<Vec<String>> {
		match self {
			Self::TrackSet(tracks) => Some(tracks.clone()),
			_ => None,
		}
	}
	pub fn as_identifier(&self) -> Option<String> {
		match self {
			Self::Identifier(id) => Some(id.clone()),
			_ => None,
		}
	}
}
