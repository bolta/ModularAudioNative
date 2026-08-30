use std::path::PathBuf;

#[derive(Debug)]
pub struct PlayerOptions {
	pub moddl_path: PathBuf,
	pub dump_ast: bool,
	pub no_play: bool,
	pub output: PlayerOutput,
}

#[derive(Clone, Debug)]
pub enum PlayerOutput {
	Audio,
	Wav { path: PathBuf },
	Stdout,
	Null,
}
