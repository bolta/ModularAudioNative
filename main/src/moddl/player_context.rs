use super::{
	builtin::*, console::*, error::*, evaluator::*, executor::process_statements, import_cache::ImportCache, io::Io, path::*, player_option::*, scope::*, value::*
};
use crate::{
	calc::*,
	common::stack::*,
	core::{
		common::*,
		context::*,
		event::*,
		machine::*,
		node::*,
		node_factory::*,
		node_host::*,
	},
	mml::default::{
		feature::Feature,
		sequence_generator::*,
	},
	node::{
		audio::*,
		cond::*,
		prim::*,
		stereo::*,
		system::*,
		util::*,
		var::*,
	},
	seq::{
		sequencer::*,
		tick::*,
	},
	vis::{
		visualizer::*,
	},
	wave::{
		waveform_host::*,
		wav_reader::*, waveform::Waveform,
	}
};
extern crate parser;
use graphviz_rust::attributes::start;
use nom::Err;
use parser::{
	common::{Location, Span}, mml::default_mml_parser, moddl::{ast::*, parser::{compilation_unit, expr}}
};

use std::{
	borrow::Borrow,
	cell::RefCell,
	collections::btree_map::BTreeMap,
	collections::hash_map::HashMap,
	collections::hash_set::HashSet,
	fs::File,
	io::Read,
	path::Path,
	rc::Rc,
	sync::{
		Arc,
		mpsc,
	},
	thread,
};

pub struct PlayerContext {
	pub moddl_path: String,
	pub sample_rate: i32,
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
	pub seq_tags: HashSet<String>,
	// ソースファイルの先頭でだけオプションを許すためのフラグ
	pub allows_option_here: bool,
	// #21 パラメータ名を暗黙にラベルにする。互換動作
	pub use_default_labels: bool,
}
impl PlayerContext {
	pub fn init(moddl_path: &str, sample_rate: i32) -> Self {
		// ルートに直に書き込むと import したときにビルトインのエントリが衝突するので、1 階層切っておく
		// TODO ルートは singleton にできるはず…
		let root_vars = Scope::root(builtin_vars(sample_rate));
		let vars = Scope::child_of(root_vars);

		Self {
			moddl_path: moddl_path.to_string(),
			sample_rate,
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
