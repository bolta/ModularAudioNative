use serde::{
	Deserialize,
	Serialize,
};

#[derive(Deserialize, Serialize, Debug)]
pub struct Set {
	pub target: String,
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
