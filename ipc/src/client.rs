use anyhow::anyhow;
use iceoryx2::{node::{Node, NodeBuilder}, port::{client::Client as Iox2Client, listener::Listener, notifier::Notifier}, prelude::ZeroCopySend, service::ipc::Service};
use std::{fmt::Debug, time::{Duration, Instant}};

use crate::common::{IpcError, request_event_name, response_event_name, retry_on_transient_error};

pub struct Client {
	node: Node<Service>,
	client: Iox2Client<Service, [u8], (), [u8], ()>,
	request_notifier: Notifier<Service>,
	response_listener: Listener<Service>,
	request_timeout: Duration,
}

impl Client {
	pub fn new(channel_name: &str, request_timeout: Duration) -> anyhow::Result<Self> {
		retry_on_transient_error(|| Self::new_impl(channel_name, request_timeout))
	}

	// TODO エラーはもう少し丁寧に扱うかも
	fn new_impl(channel_name: &str, request_timeout: Duration) -> anyhow::Result<Self>
	{
		let node = NodeBuilder::new().create::<Service>() ?;
		let client = {
			let factory = node
					.service_builder(& channel_name.try_into() ?)
					.request_response::<[u8], [u8]>()
					.open_or_create() ?;
			factory.client_builder()
					.initial_max_slice_len(16)
					.allocation_strategy(iceoryx2::prelude::AllocationStrategy::PowerOfTwo)
					.create() ?
		};
		let request_notifier = {
			let factory = node
					.service_builder(& request_event_name(channel_name).as_str().try_into() ?)
					.event()
					.open_or_create() ?;
			factory.notifier_builder().create() ?
		};
		let response_listener = {
			let factory = node
					.service_builder(& response_event_name(channel_name).as_str().try_into() ?)
					.event()
					.open_or_create() ?;
			factory.listener_builder().create() ?
		};

		Ok(Self { node, client, request_notifier, response_listener, request_timeout })
	}

	// レスポンスが来るまでブロックするので専用スレッドで動かす必要がある
	pub fn send_request(&self, value: &[u8]) -> anyhow::Result<Vec<u8>> {
		let request = self.client.loan_slice_uninit(value.len()) ?;
		let request = request.write_from_slice(value);
		let pending_response = request.send() ?;
		self.request_notifier.notify() ?;
		let request_time = Instant::now();

		loop {
			if let Err(e) = self.node.wait(Duration::ZERO) { return Err(e.into()); }

			if request_time.elapsed() >= self.request_timeout { return Err(anyhow!(IpcError::Timeout)); }

			match self.response_listener.timed_wait_one(Duration::from_millis(100)) {
				// エラー（ListenerWaitError）。中断
				Err(e) => { return Err(e.into()); },
				// レスポンス通知が来ないままタイムアウトした。待ち続ける
				Ok(None) => { continue; },
				// 通知が来た。処理する
				Ok(Some(..)) => { },
			}
			
			if let Some(res_value) = pending_response.receive() ? {
				// clone せずにコールバックで扱わせた方がより軽いかもだが、そこまではいいか
				// return Ok(res_value.payload().clone());
				return Ok(res_value.payload().to_vec());
			} else {
				// 来るはずはないと思うが…
				// TODO もしここに来た場合、このまま while ループを続けて意味ある？　return Err すべき？
				println!("👻 unreachable?");
			}
		}
	}
}
