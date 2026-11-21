use crate::moddl::module_def::ModuleDef;

use super::{
	error::*, scope::*, value::*,
};
extern crate parser;
use graphviz_rust::dot_structures::EdgeTy;
use parser::common::Location;

use std::{
	cell::RefCell, collections::{btree_map::BTreeMap, hash_map::HashMap, hash_set::HashSet}, ops::Rem, path::{Path, PathBuf}, rc::Rc,
};

const SETTINGS_ASSOC_NAME: &str = "Settings";
const SETTINGS_KEY_TEMPO: &str = "tempo";
const SETTINGS_KEY_TICKS_PER_BAR: &str = "ticksPerBar";
const SETTINGS_KEY_GROOVE_CYCLE: &str = "grooveCycle";

// TODO 定義フェーズの結果なので、それらしい名前に変える
pub struct PlayerContext {
	pub moddl_path: PathBuf,
	// トラックごとの instrument/effect
	// （書かれた順序を保持するため Vec で持つ）
	pub track_defs: Vec<(String, TrackDef, Location)>,
	// effect に接続されていない、「末端」であるトラック。master でミックスする対象
	pub terminal_tracks: HashSet<String>,
	pub grooves: HashMap<String, (String, Location)>, // トラックに対する Tick のタグ名
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
		let _ = add_initial_settings(&vars); // 新しいスコープなので必ず成功する

		Self {
			moddl_path: moddl_path.to_path_buf(),
			track_defs: vec![],
			terminal_tracks: HashSet::new(),
			grooves: HashMap::new(),
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
	pub fn defined_track_names(&self) -> Vec<String> {
		self.track_defs.iter().map(|(name, _, _)| name.clone()).collect()
	}

	pub fn tempo(&self) -> f32 {
		self.settings_item_number(SETTINGS_KEY_TEMPO)
	}
	pub fn set_tempo(&self, value: &Value) -> ModdlResult<()> {
		self.set_settings_item_number(SETTINGS_KEY_TEMPO, value)
	}

	pub fn ticks_per_bar(&self) -> i32 {
		self.settings_item_number(SETTINGS_KEY_TICKS_PER_BAR) as i32
	}
	pub fn set_ticks_per_bar(&self, value: &Value) -> ModdlResult<()> {
		let value = positive_integer_only(value, SETTINGS_KEY_TICKS_PER_BAR) ?;
		self.set_settings_item_number(SETTINGS_KEY_TICKS_PER_BAR, &value)
	}

	pub fn groove_cycle(&self) -> i32 {
		self.settings_item_number(SETTINGS_KEY_GROOVE_CYCLE) as i32
	}
	pub fn set_groove_cycle(&self, value: &Value) -> ModdlResult<()> {
		let value = positive_integer_only(value, SETTINGS_KEY_GROOVE_CYCLE) ?;
		self.set_settings_item_number(SETTINGS_KEY_GROOVE_CYCLE, &value)
	}

	fn settings_item_number(&self, key: &str) -> f32 {
		// 初期化時に登録しているので必ずある
		let settings = self.vars.borrow().lookup(&SETTINGS_ASSOC_NAME.into()).unwrap().0;
		let settings_assoc = settings.as_assoc().unwrap();

		// 初期化時に登録しているので必ずある
		let setting_value = settings_assoc.get(&key.to_string()).unwrap();

		// 初期値も再設定できる値も数値のみ
		setting_value.0.as_number().unwrap()
	}
	fn set_settings_item_number(&self, key: &str, value: &Value) -> ModdlResult<()> {
		value.as_number() ?;

		// 初期化時に登録しているので必ずある
		let settings = self.vars.borrow().lookup(&SETTINGS_ASSOC_NAME.into()).unwrap().0;
		let settings_assoc = settings.as_assoc().unwrap();

		let (setting_value, setting_loc) = settings_assoc.get(&key.to_string()).unwrap().as_lazy().unwrap();
		setting_value.set(value.clone()).map_err(|(_prev_val, prev_loc)| {
			error(ErrorType::SettingDuplicate {
				name: key.into(),
				set_loc: prev_loc,
			}, setting_loc)
		})
	}

}

fn add_initial_settings(vars: &Rc<RefCell<Scope>>) -> ModdlResult<()> {
	let mut settings = HashMap::new();
	let mut add_number = |key: &str, value| {
		settings.insert(key.into(), (
			ValueBody::Lazy(Rc::new(LazyValue::new(ValueBody::Number(value)))),
			Location::dummy(),
		));
	};
	add_number(SETTINGS_KEY_TEMPO, 120f32);
	add_number(SETTINGS_KEY_TICKS_PER_BAR, 384f32);
	add_number(SETTINGS_KEY_GROOVE_CYCLE, 384f32);

	vars.borrow_mut().set(&"Settings".to_string(), (ValueBody::Assoc(settings), Location::dummy()))
}

fn positive_integer_only(value: &Value, name: &str) -> ModdlResult<Value> {
	let (value_num, loc) = value.as_number() ?;
	let int_num = value_num.floor();

	if int_num > 0f32 {
		Ok((ValueBody::Number(int_num), loc))
	} else {
		Err(error(ErrorType::BadSetting {
			name: name.into(),
			detail: ERROR_DETAIL_MUST_BE_POSITIVE.into(),
		}, loc))
	}
}

#[derive(PartialEq)]
pub enum MuteSolo { Mute, Solo }

pub enum TrackDef {
	Instrument(ModuleDef),
	Effect(HashSet<String>, ModuleDef),
	Groove(ModuleDef),
}
