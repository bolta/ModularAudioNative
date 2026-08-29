use iceoryx2::{node::{Node, NodeBuilder}, service::ipc::Service};
use std::time::Duration;

/// Ctrl+C（SIGINT）や SIGTERM による終了要求を検知するためのウォッチャー。
///
/// iceoryx2 は Node を 1 つでも作成すると、共有メモリ上の IPC リソースを
/// 安全に解放できるよう、プロセス全体の SIGINT/SIGTERM ハンドリングを自前のものに
/// 差し替える。そのため OS 標準の「Ctrl+C を押すと即座にプロセスが終了する」という
/// 挙動は失われ、以降は `Node::wait` が終了要求を検知したときにアプリ側が
/// 明示的に終了処理を行う必要がある。
pub struct TerminationWatcher {
	node: Node<Service>,
}

impl TerminationWatcher {
	pub fn new() -> anyhow::Result<Self> {
		Ok(Self { node: NodeBuilder::new().create::<Service>() ? })
	}

	/// Ctrl+C 等による終了要求が来るまでブロックする。
	/// cycle は内部ポーリング間隔で、終了検知が遅れる最大時間になる。
	pub fn block_until_termination_requested(&self, cycle: Duration) {
		while self.node.wait(cycle).is_ok() { }
	}
}
