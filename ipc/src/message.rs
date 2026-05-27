use serde::{
	Deserialize,
	Serialize,
};

#[derive(Deserialize, Serialize, Debug)]
pub struct RegisterSettings {
	pub items: Vec<RegisterSettingsItem>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RegisterSettingsItem {
	pub path: String,
	pub key: String,
	/// 初期値。player からオーバーライドされればその値、そうでなければ original と同じ値
	pub initial: f32,
	/// MML で設定された本来の初期値。オーバーライドされる可能性がある
	pub original: f32,
	// TODO 範囲ヒントを追加
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Set {
	pub path: String,
	pub key: String,
	pub value: f32,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum LogLevel {
	Error,
	Warn,
	Info,
	Debug,
}

/// player から controller にログを転送するためのメッセージ形式
#[derive(Deserialize, Serialize, Debug)]
pub struct Log {
	pub level: LogLevel,
	pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
	pub status: String,
	pub text: String,
}
