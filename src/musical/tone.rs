pub struct Tone {
	pub octave: i32,
	pub base_name: BaseName,
	pub accidental: i32,
}

#[derive(PartialEq, Eq, Hash)]
pub enum BaseName {
	None = 0,
	C = 1,
	D = 2,
	E = 3,
	F = 4,
	G = 5,
	A = 6,
	B = 7,
}
