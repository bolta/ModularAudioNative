use std::collections::BTreeMap;

use serde::{
	Deserialize,
	Serialize, de::DeserializeOwned,
};

use crate::message;

pub trait Message: Serialize + DeserializeOwned {
	/// JSON から元の型を復元するためのタグ
	const TYPE_TAG: &str;
}

/// TAG を型と同名で定義する
// TODO 手続きマクロで自動導出を実装する方がよい
macro_rules! message_type {
	($type: ty) => {
		impl Message for $type {
			const TYPE_TAG: &str = stringify!($type);
		}
	};
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RegisterSettings {
	pub items: BTreeMap<String, RegisterSettingsItem>,
}
message_type!(RegisterSettings);

// このあたりの定義は player 側にもほぼ同じものが一式ある。
// 内部処理用とメッセージング用という用途の違いがあるので、現時点では同一のデータ構造であっても使い回さない方が無難と考えたためだが、
// DomainHint みたいに、そもそも別プロセスから使う前提のものはこっちだけに置いてもいいのかもしれない

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum RegisterSettingsItem {
	Settings {
		key: String,
		initial: f32,
		original: f32,
		domain: Option<DomainHint>,
	},
	Group(BTreeMap<String, RegisterSettingsItem>)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum DomainHint {
	Range {
		min: f32,
		includes_min: bool,
		max: f32,
		includes_max: bool,
	},
	Enum {
		items: Vec<EnumItem>,
	},
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnumItem { pub value: f32, pub name: String }


#[derive(Deserialize, Serialize, Debug)]
pub struct Set {
	pub path: String,
	pub key: String,
	pub value: f32,
}
message_type!(Set);

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum LogLevel {
	Error,
	Warn,
	Info,
	Debug,
}

/// player から controller にログを転送するためのメッセージ形式
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Log {
	pub level: LogLevel,
	pub message: String,
}
message_type!(Log);

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Response {
	pub status: String,
	pub text: String,
}
message_type!(Response);

