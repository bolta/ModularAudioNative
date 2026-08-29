use std::{fmt::{Debug, Display}, thread, time::Duration};

// 直前に強制終了したプロセス（デバッガでの強制停止など）が iceoryx2 のサービスの
// 資源を中途半端な状態（ServiceInCorruptedState 等）で残すことが稀にある。
// iceoryx2 は「死んだノードが残した資源」を Node 生成のたびに自動で掃除する仕組みを
// 持っているため、少し待って Node（ひいてはサービス）を作り直すだけで解消することが多い。
const TRANSIENT_ERROR_RETRY_COUNT: u32 = 3;
const TRANSIENT_ERROR_RETRY_DELAY: Duration = Duration::from_millis(100);

/// Node/サービスの生成を試み、失敗したら短い間隔を空けて数回リトライする。
pub fn retry_on_transient_error<T>(mut f: impl FnMut() -> anyhow::Result<T>) -> anyhow::Result<T> {
	let mut attempt = 0;
	loop {
		match f() {
			Ok(v) => return Ok(v),
			Err(e) if attempt < TRANSIENT_ERROR_RETRY_COUNT => {
				attempt += 1;
				println!("🔁 retrying after transient IPC error (attempt {}/{}): {}", attempt, TRANSIENT_ERROR_RETRY_COUNT, e);
				thread::sleep(TRANSIENT_ERROR_RETRY_DELAY);
			}
			Err(e) => return Err(e),
		}
	}
}

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
