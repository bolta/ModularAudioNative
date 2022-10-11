pub enum PlayerOutput {
	Audio,
	Wav { path: String },
	Stdout,
	Null,
}
