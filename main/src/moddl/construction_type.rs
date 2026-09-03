pub enum ConstructionType {
	Tempo,
	Instrument,
	Effect,
	GrooveCycle,
	Groove,
	Let,
	LetAll,
	Do,
	// Dos も必要か？　使い方が思い浮かばないが、ありそうなら追加する
	Waveform,
	TicksPerBar,
	TicksPerBeat,
	Mute,
	Solo,
	Export,
	Option,
}

impl ConstructionType {
	pub fn from_name(name: &str) -> Option<ConstructionType> {
		match name {
			"tempo" => Some(ConstructionType::Tempo),
			"instrument" => Some(ConstructionType::Instrument),
			"effect" => Some(ConstructionType::Effect),
			"grooveCycle" => Some(ConstructionType::GrooveCycle),
			"groove" => Some(ConstructionType::Groove),
			"let" => Some(ConstructionType::Let),
			"letAll" => Some(ConstructionType::LetAll),
			"do" => Some(ConstructionType::Do),
			"waveform" => Some(ConstructionType::Waveform),
			"ticksPerBar" => Some(ConstructionType::TicksPerBar),
			"ticksPerBeat" => Some(ConstructionType::TicksPerBeat),
			"mute" => Some(ConstructionType::Mute),
			"solo" => Some(ConstructionType::Solo),
			"export" => Some(ConstructionType::Export),
			"option" => Some(ConstructionType::Option),
			_ => None,
		}
	}
}
