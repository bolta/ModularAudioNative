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
// TODO 生成したタグ名を tags に収集しているのは、skip 時の tick 供給先のセットを作るため。
// 現状は skip が機能していないので、収集結果は使われていない。
// #48 のリファクタリングに伴って収集が面倒な場面が生じたのでとりあえず tags を Option にしたが、
// 本当は skip を復活させるなら Option はだめ、復活させないならこの処理自体廃止、があるべき姿なので、
// 現状はどっちつかず
pub fn make_seq_tag(track: Option<&String>, tags: Option<&mut HashSet<String>>) -> String {
	let tag = match track {
		None => "#seq".to_string(),
		Some(track) => format!("#seq_{}", track),
	};
	tags.map(|tags| tags.insert(tag.clone()));

	tag
}
