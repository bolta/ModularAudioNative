pub struct PlayerOptions {
	pub moddl_path: String,
	pub output: PlayerOutput,
}

pub enum PlayerOutput {
	Audio,
	Wav { path: String },
	Stdout,
	Null,
}
