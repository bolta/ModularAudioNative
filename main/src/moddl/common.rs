use std::{collections::HashSet, fs::File, io::Read, path::Path};

use parser::common::Location;

use super::error::{error, ModdlResult};

pub fn read_file(path: &Path) -> ModdlResult<String> {
	let mut file = File::open(path).map_err(|e| error(e.into(), Location::dummy())) ?;
	let mut moddl = String::new();
	file.read_to_string(&mut moddl).map_err(|e| error(e.into(), Location::dummy())) ?;

	Ok(moddl)
}

/// シーケンサのタグ名を生成する。また生成したタグ名を記録する
pub fn make_seq_tag(track: Option<&String>, tags: &mut HashSet<String>) -> String {
	let tag = match track {
		None => "#seq".to_string(),
		Some(track) => format!("#seq_{}", track),
	};
	tags.insert(tag.clone());

	tag
}
