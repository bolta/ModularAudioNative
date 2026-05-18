use std::{fmt::{Debug, Display}};

pub fn request_event_name(channel_name: &str) -> String {
	channel_name.to_string() + "#request"
}
pub fn response_event_name(channel_name: &str) -> String {
	channel_name.to_string() + "#response"
}

pub fn channel_name_c2p() -> &'static str { "moddl_ipc_c2p" }
pub fn channel_name_p2c() -> &'static str { "moddl_ipc_p2c" }

#[derive(Debug)]
pub enum IpcError {
	Timeout,
	Interrupt,
}
impl Display for IpcError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			IpcError::Timeout => write!(f, "timed out"),
			IpcError::Interrupt => write!(f, "interrupted"),
		}
	}
}
