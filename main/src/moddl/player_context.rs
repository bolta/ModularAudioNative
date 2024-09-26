use super::{
	error::*, scope::*, value::*,
};
extern crate parser;
use parser::common::Location;

use std::{
	cell::RefCell,
	collections::{btree_map::BTreeMap, hash_map::HashMap, hash_set::HashSet},
	path::{Path, PathBuf},
	rc::Rc,
};

pub struct PlayerContext {
	pub moddl_path: PathBuf,
	pub tempo: f32,
	pub ticks_per_bar: i32,
	// トラックごとの instrument/effect
	// （書かれた順序を保持するため Vec で持つ）
	pub track_defs: Vec<(String, TrackDef, Location)>,
	// effect に接続されていない、「末端」であるトラック。master でミックスする対象
	pub terminal_tracks: HashSet<String>,
	pub grooves: HashMap<String, (String, Location)>, // トラックに対する Tick のタグ名
	pub groove_cycle: i32,
	// トラックごとの MML を蓄積
	pub mmls: BTreeMap<String, String>,
	pub mute_solo: MuteSolo,
	pub mute_solo_tracks: HashSet<String>,
	pub vars: Rc<RefCell<Scope>>,
	pub export: Option<Value>,
	pub seq_tags: HashSet<String>,
	// ソースファイルの先頭でだけオプションを許すためのフラグ
	pub allows_option_here: bool,
	// #21 パラメータ名を暗黙にラベルにする。互換動作
	pub use_default_labels: bool,
}
impl PlayerContext {
	pub fn init(moddl_path: &Path, root_scope: Rc<RefCell<Scope>>) -> Self {
		// ルートに直に書き込むと import したときにビルトインのエントリが衝突するので、1 階層切っておく。
		// ルートは 1 つのインスタンスを使い回す
		let vars = Scope::child_of(root_scope);

		Self {
			moddl_path: moddl_path.to_path_buf(),
			tempo: 120f32,
			ticks_per_bar: 384,
			track_defs: vec![],
			terminal_tracks: HashSet::new(),
			grooves: HashMap::new(),
			groove_cycle: 384,
			mmls: BTreeMap::new(),
			mute_solo: MuteSolo::Mute,
			mute_solo_tracks: HashSet::new(),
			vars,
			export: None,
			seq_tags: HashSet::new(),
			allows_option_here: true,
			use_default_labels: false,
		}
	}

	pub fn get_track_def(&self, track: &String) -> Option<(&TrackDef, &Location)> {
		self.track_defs.iter().find(|&elem| elem.0 == *track)
				.map(|elem| (&elem.1, &elem.2))
	}
	pub fn add_track_def(&mut self, track: &String, spec: TrackDef, loc: &Location) -> ModdlResult<()> {
		match self.get_track_def(track) {
			None => {
				self.track_defs.push((track.clone(), spec, loc.clone()));
				Ok(())
			}
			Some((_, ex_loc)) => {
				Err(error(ErrorType::TrackDefDuplicate {
					track: track.clone(),
					existing_def_loc: ex_loc.clone(),
				}, loc.clone()))
			}
		}
	}
}

#[derive(PartialEq)]
pub enum MuteSolo { Mute, Solo }

pub enum TrackDef {
	Instrument(NodeStructure),
	Effect(HashSet<String>, NodeStructure),
	Groove(NodeStructure),
}
