use ipc::{Client, Response, Server, Set, channel_name_c2p, channel_name_p2c};

use std::{process::Command, thread, time::Duration};

fn main() -> anyhow::Result<()> {
	// build.rs で設定された環境変数からバイナリパスを取得
	let bin_path = env!("MODDL_BIN_PATH");
	let bin_name = env!("MODDL_BIN_NAME");
	
	println!("Executing: {} ({})", bin_name, bin_path);
	
	
	// main プロジェクトのバイナリを実行
	let mut cmd = Command::new(bin_path);
	
	// コマンドライン引数をそのまま渡す場合
	// TODO チャンネル名を可変にするためコマンド引数で渡す
	let args = // vec![
	// 	"--ipc-channel-c2p", channel_name_c2p(),
	// 	"--ipc-channel-p2c", channel_name_p2c(),
	// ].into_iter().map(|arg| arg.to_string())
	/* .chain */(std::env::args().skip(1));
	cmd.args(args);
	let mut player = cmd.spawn() ?;

	// TODO 別の場所に移した方がよさそう
	thread::spawn(|| {
		// TODO ちゃんとエラー処理
		let p2c_server = Server::new(channel_name_p2c()).unwrap();
		loop {
			p2c_server.wait_for_request(|request_bytes| {
				match bson::Document::from_reader(request_bytes) {
					Ok(doc) => {
						println!("controller received {}", &doc);
						match doc.get_str("type") {
							Ok(tipe) => {
								match tipe {
									"ping" => {
										let response = Response {
											status: "ok".into(),
											text: "pong".into(),
										};
										Ok(bson::serialize_to_vec(&response).unwrap())
									}
									_ => {
										// TODO エラーにすべきかもしれない。ちゃんと処理
										let response = Response {
											status: "ok".into(),
											text: "hello".into(),
										};
										Ok(bson::serialize_to_vec(&response).unwrap())
									}
								}
							}
							Err(e) => todo!()
						}
					}
					Err(e) => todo!()
				}
			}).unwrap_or_else(|e| println!("{}", e));
		}
	});

	// cutoff を定期的に変更する仮実装
	// TODO 削除
	thread::spawn(|| {
		let c2p_client = Client::new(channel_name_c2p(), Duration::from_millis(500)).unwrap();
		let mut cutoff = 1000;
		loop {
			let msg = Set {
				path: "a.cutoff".to_string(),
				key: "value".to_string(),
				value: cutoff as f32,
			};
			let mut b = bson::serialize_to_document(&msg).unwrap();
			// TODO うまく書ける方法を確立したい
			b.insert("type", "set");
			match c2p_client.send_request(b.to_vec().unwrap().as_slice()) {
				Ok(_response) => { },
				Err(e) => { println!("@@@@@@@@@@@@@@ {}", e); }
			}

			cutoff = (cutoff + 500) % 10000;

			thread::sleep(Duration::from_millis(500));
		}
	});

	player.wait() ?;
	Ok(())
}
