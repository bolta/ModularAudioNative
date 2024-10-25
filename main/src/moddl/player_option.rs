use std::path::PathBuf;

pub struct PlayerOptions {
	pub moddl_path: PathBuf,
	pub output: PlayerOutput,
}

pub enum PlayerOutput {
	Audio,
	Wav { path: PathBuf },
	Stdout,
	Null,
}
