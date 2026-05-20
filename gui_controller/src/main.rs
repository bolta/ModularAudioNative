use std::{process::Command, rc::Rc, thread, time::Duration};

use dioxus::{logger::tracing, prelude::*};
use ipc::{Client, Response, Server, Set, channel_name_c2p, channel_name_p2c};

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const HEADER_SVG: Asset = asset!("/assets/header.svg");

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
	let _player = cmd.spawn() ?;
	// thread::sleep(Duration::from_secs(3));

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

	main_orig();
	Ok(())
}

fn main_orig() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        ControlPanel {}

    }
}

// component に渡す値は PartialEq でないとだめだというので、
// Client の実体のアドレスで判定する PartialEq を定義する
#[derive(Clone)]
struct ClientWrapper(Rc<Client>);
impl PartialEq for ClientWrapper {
	fn eq(&self, other: &Self) -> bool {
		&*self.0 as *const Client == &*other.0 as *const Client
	}
}

#[component]
fn RegisterControl(name: String, keey: String, init: f32, min: f32, max: f32, c2p_client: ClientWrapper) -> Element {
	let mut value = use_signal(|| init);

	let set_value = || {
		let c2p_client = (&c2p_client).clone();
		let target = (&name).clone();
		let key = (&keey).clone();
		move |e: Event<FormData>| if let Ok(v) = e.value().parse::<f32>() {
			*value.write() = v;
			tracing::info!("value has been set to {}", v);
			let msg = Set {
				target: target.clone(),
				key: key.clone(),
				value: v,
			};
			let mut b = bson::serialize_to_document(&msg).unwrap();
			// TODO うまく書ける方法を確立したい
			b.insert("type", "set");
			match (&c2p_client).clone().0.send_request(b.to_vec().unwrap().as_slice()) {
				Ok(_response) => { },
				Err(e) => { println!("@@@@@@@@@@@@@@ {}", e); }
			}
		}
	};

	rsx! {
		div {
			label { "{name}" }
			input {
				r#type: "range",
				min,
				max,
				value: value,
				oninput: set_value(),
			}
			input {
				id: "field1",
				value: "{value}",
				onchange: set_value(),
			}
		}
	}
}

#[component]
pub fn ControlPanel() -> Element {
	// TODO エラー処理
	let c2p_client = ClientWrapper(Rc::new(Client::new(channel_name_c2p(), Duration::from_millis(500)).unwrap()));

	// 順不同にならないよう Vec を使う
	let mut registers = use_signal(|| vec![
		("a.cutoff".to_string(), "value".to_string(), 1000f32, 30f32, 10000f32),
		("a.q".to_string(), "value".to_string(), 10f32, 1f32, 50f32),
		// ("c.baz".to_string(), -10f32),
	]);
	// signal は変更が追跡され、再描画のトリガーになったりするので、
	// 単なる内部状態としてはオーバースペックの感がある	
	// let mut count = use_signal(|| 0);

	rsx! {
		div {
			id: "control_panel",
			div {
				for (name, key, init, min, max) in registers.read().iter() {
					RegisterControl { name: name.clone(), keey: key.clone(), init: *init, min: *min, max: *max, c2p_client: c2p_client.clone() }
				}
			}
		}
	}
}
