use iceoryx2::{node::{Node, NodeBuilder}, port::{listener::Listener, notifier::Notifier, server::Server as Iox2Server}, service::ipc::Service};
use std::time::Duration;

use crate::common::{request_event_name, response_event_name, retry_on_transient_error};

pub struct Server {
	node: Node<Service>,
	server: Iox2Server<Service, [u8], (), [u8], ()>,
	request_listener: Listener<Service>,
	response_notifier: Notifier<Service>,
}

impl Server {
	pub fn new(channel_name: &str) -> anyhow::Result<Self> {
		retry_on_transient_error(|| Self::new_impl(channel_name))
	}

	// TODO エラーはもう少し丁寧に扱うかも
	fn new_impl(channel_name: &str) -> anyhow::Result<Self> {
		let node = NodeBuilder::new().create::<Service>() ?;
		let server = {
			let factory = node
					.service_builder(& channel_name.try_into() ?)
					.request_response::<[u8], [u8]>()
					.open_or_create() ?;
			factory.server_builder()
					.initial_max_slice_len(16)
					.allocation_strategy(iceoryx2::prelude::AllocationStrategy::PowerOfTwo)
					.create() ?
		};
		let request_listener = {
			let factory = node
					.service_builder(& request_event_name(channel_name).as_str().try_into() ?)
					.event()
					.open_or_create() ?;
			factory.listener_builder().create() ?
		};
		let response_notifier = {
			let factory = node
					.service_builder(& response_event_name(channel_name).as_str().try_into() ?)
					.event()
					.open_or_create() ?;
			factory.notifier_builder().create() ?
		};

		Ok(Self { node, server, request_listener, response_notifier })
	}

	// リクエストが来るまでブロックするので専用スレッドで動かす必要がある
	pub fn wait_for_request<MakeResponse: Fn (&[u8]) -> anyhow::Result<Vec<u8>>>(&self, make_response: MakeResponse) -> anyhow::Result<()> {
		// TODO タイムアウトを設ける
		loop {
			// Ctrl+C で抜ける
			if let Err(e) = self.node.wait(Duration::ZERO) { return Err(e.into()); }

			// timed_wait_one の待ち時間はサーバとしての動作に関する限り何でもよいが、
			// Ctrl+C での終了がこの切れ目まで待たされるので、適度に小さい方がよい。
			match self.request_listener.timed_wait_one(Duration::from_millis(100)) {
				// エラー（ListenerWaitError）。中断
				Err(e) => { return Err(e.into()); },
				// リクエスト通知が来ないままタイムアウトした。待ち続ける
				Ok(None) => { continue; },
				// 通知が来た。処理する
				Ok(Some(..)) => { },
			}

			if let Some(request) = self.server.receive() ? {
				let response_payload = make_response(request.payload()) ?;
				let response = request.loan_slice_uninit(response_payload.len()) ?;
				let response = response.write_from_slice(&response_payload);
				response.send() ?;
				self.response_notifier.notify() ?;
				// 1 件処理したら帰る（ループは呼び出し側で行う）が、継続処理でもいいかも
				return Ok(());

			} else {
				// リクエストは必ず来ているはずなのでここには来ないはず。
				// TODO 万一来たら return Err すべきか？
				println!("👹 unreachable??");
			}
		}
	}

	pub fn poll_for_request<MakeResponse: Fn (&[u8]) -> anyhow::Result<Vec<u8>>>(&self, make_response: MakeResponse) -> anyhow::Result<()> {
		// TODO タイムアウトを設ける
		// loop {
		// 	// Ctrl+C で抜ける
		// 	if let Err(e) = self.node.wait(Duration::ZERO) { return Err(e.into()); }

		// 	// timed_wait_one の待ち時間はサーバとしての動作に関する限り何でもよいが、
		// 	// Ctrl+C での終了がこの切れ目まで待たされるので、適度に小さい方がよい。
		// 	match self.request_listener.timed_wait_one(Duration::from_millis(100)) {
		// 		// エラー（ListenerWaitError）。中断
		// 		Err(e) => { return Err(e.into()); },
		// 		// リクエスト通知が来ないままタイムアウトした。待ち続ける
		// 		Ok(None) => { continue; },
		// 		// 通知が来た。処理する
		// 		Ok(Some(..)) => { },
		// 	}

		// 毎サンプルポーリングする request_listener 

		while let Some(request) = self.server.receive() ? {
			let response_payload = make_response(request.payload()) ?;
			let response = request.loan_slice_uninit(response_payload.len()) ?;
			let response = response.write_from_slice(&response_payload);
			response.send() ?;
			self.response_notifier.notify() ?;
			// 1 件処理したら帰る（ループは呼び出し側で行う）が、継続処理でもいいかも
		}

		Ok(())
		// }
	}
}
