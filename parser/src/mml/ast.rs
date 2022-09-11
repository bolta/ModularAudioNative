#[derive(Debug, PartialEq)]
pub struct CompilationUnit {
	pub commands: Vec<Command>,
}

#[derive(Debug, PartialEq)]
pub enum Command {
	// コマンドの名前が値の名前そのものである場合はパラメータ名を省略
	Octave(i32),
	OctaveIncr,
	OctaveDecr,
	Length(i32),
	GateRate(f32),
	Volume(f32),
	Velocity(f32),
	Detune(f32),
	Tone { tone_name: ToneName, length: Length, slur: bool },
	Rest(Length),
	Parameter { name: String, value: f32 },
	Tempo(f32),
	/// times は Some(n) で有限、None で無限、
	/// content1 は : より前（: がない場合は全部）、content 2 は : より後（: がない場合は None）
	Loop { times: Option<i32>, content1: Vec<Command>, content2: Option<Vec<Command>> },
	// LoopBreak,
	Stack { content: Vec<Command> },
	MacroDef { name: String, content: Vec<Command> },
	Skip,
	ExpandMacro { name: String },
}

#[derive(Debug, PartialEq)]
pub struct Length {
	pub elements: Vec<LengthElement>,
}

#[derive(Debug, PartialEq)]
pub struct LengthElement {
	/// 音長を示す数値。省略の場合は None。音長 4. に対して 4、.. に対して None となる
	pub number: Option<i32>,

	/// 付点の数
	pub dots: i32,
}

#[derive(Debug, PartialEq)]
pub struct ToneName {
	pub base_name: ToneBaseName,
	pub accidental: i32,
}

#[derive(Debug, PartialEq)]
pub enum ToneBaseName {
	C, D, E, F, G, A, B,
}
