use std::{process::Command, rc::Rc, thread, time::Duration};

use dioxus::{logger::tracing, prelude::*};
use ipc::{Client, DomainHint, RegisterSettings, Response, Server, Set, channel_name_c2p, channel_name_p2c, decode, to_bson};

use crate::view_model::{RegisterSettingsItemStore, RegisterSettingsItemStoreStoreExt, RegisterSettingsItemsStore, RegisterSettingsStore, RegisterSettingsStoreStoreExt};

mod view_model;

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

	main_orig();
	Ok(())
}

fn main_orig() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
	let registers: Store<RegisterSettingsStore> = use_store(|| RegisterSettingsStore {
		items: Default::default(),
	});

	use_future(move || async move {
		let (tx, rx) = std::sync::mpsc::channel::<bson::Document>();
		thread::spawn(move || {
			// TODO ちゃんとエラー処理
			let p2c_server = Server::new(channel_name_p2c()).unwrap();
			loop {
				p2c_server.wait_for_request(|request_bytes| {
					// let b = bson::Document::from_reader(request_bytes);
					let bson = to_bson(request_bytes) ?;
					if let Some(settings) = decode::<RegisterSettings>(&bson) {
						// TODO エラー処理
						tx.send(bson.clone()) ?;
						let response = Response {
							status: "ok".into(),
							text: "received".into(),
						};
						Ok(bson::serialize_to_vec(&response).unwrap())
					}else {
						let response = Response {
							status: "ok".into(),
							text: "hoge".into(),
						};
						Ok(bson::serialize_to_vec(&response).unwrap())
					}
					// TODO 復活させる

					// match bson::Document::from_reader(request_bytes) {
					// 	Ok(doc) => {
					// 		println!("controller received {}", &doc);
					// 		println!("received a message");
					// 		// tx.send(doc);
					// 		match doc.get_str("type") {
					// 			Ok(tipe) => {
					// 				println!("************************************ {}", tipe);
					// 				match tipe {
					// 					"ping" => {
					// 						let response = Response {
					// 							status: "ok".into(),
					// 							text: "pong".into(),
					// 						};
					// 						Ok(bson::serialize_to_vec(&response).unwrap())
					// 					},
					// 					"registerSettings" => {
					// 						// TODO エラー処理
					// 						tx.send(doc.clone());
					// 						let response = Response {
					// 							status: "ok".into(),
					// 							text: "received".into(),
					// 						};
					// 						Ok(bson::serialize_to_vec(&response).unwrap())
					// 					},
					// 					_ => {
					// 						// TODO エラーにすべきかもしれない。ちゃんと処理
					// 						let response = Response {
					// 							status: "ok".into(),
					// 							text: "hello".into(),
					// 						};
					// 						Ok(bson::serialize_to_vec(&response).unwrap())
					// 					},
					// 				}
					// 				// TODO tx.send(doc) はここで一括で行う？
					// 			}
					// 			Err(e) => todo!()
					// 		}
					// 	}
					// 	Err(e) => todo!()
					// }
				}).unwrap_or_else(|e| println!("{}", e));
			}
		});

		loop {
			if let Ok(doc) = rx.try_recv() {
				// TODO 判定が送信側と重複している
				match doc.get_str("type") {
					Ok(tipe) => {
						match tipe {
							"RegisterSettings" => {
								let msg: RegisterSettings = bson::deserialize_from_document(doc).unwrap();
								dbg!(&msg);
								let store = RegisterSettingsStore::from(msg);
								let mut items_ = registers.items();
								let mut items = items_.write();
								items.clear();
								items.extend(store.items);

							},
							_ => { },
						}
					}
					Err(e) => todo!()
				}
			}
			tokio::time::sleep(Duration::from_millis(100)).await;
		}
	});

	rsx! {
		document::Link { rel: "icon", href: FAVICON }
		document::Link { rel: "stylesheet", href: MAIN_CSS }
		ControlPanel { registers }
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
				path: target.clone(),
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
				step: (max - min) / 1000f32,
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
fn Item(item: Store<RegisterSettingsItemStore>, name: String, path: String, c2p_client: ClientWrapper) -> Element {
	let is_settings = matches!(&*item.read(), RegisterSettingsItemStore::Settings { .. });
	if is_settings {
		let (key, initial, min, max) = {
			let item = item.read();
			match &*item {
				RegisterSettingsItemStore::Settings { key, initial, domain, .. } => {
					let (min, max) = domain.as_ref().map(|domain| match domain {
						// TODO 端点を含まない対応
						DomainHint::Range { min, max, .. } => (*min, *max),
						// TODO Enum 対応
						DomainHint::Enum { .. } => (-5000f32, 5000f32),
					// TODO 定義域がない場合の対応
					}).unwrap_or((-5000f32, 5000f32));
					(key.clone(), *initial, min, max)
				},
				RegisterSettingsItemStore::Group(_) => unreachable!(),
			}
		};

		rsx! {
			RegisterControl {
				name: path,
				keey: key,
				init: initial,
				min, max,
				c2p_client,
			}
		}
	} else if let Some(group) = item.group() {
			rsx! {
				fieldset {
					legend { "{name}" }
					ItemList {
						items: group,
						path,
						c2p_client,
					}
				}
			}
	} else {
		rsx! {}
	}
}

#[component]
fn ItemList(items: Store<RegisterSettingsItemsStore>, path: String, c2p_client: ClientWrapper) -> Element {
	rsx! {
		for (name, item) in items.iter() {
			Item {
				item,
				name: name.clone(),
				path: if path.len() > 0 { format!("{}.{}", &path, &name) } else { name.clone() },
				c2p_client: c2p_client.clone(),
			}
		}
	}
}

#[component]
pub fn ControlPanel(registers: Store<RegisterSettingsStore>) -> Element {
	// TODO エラー処理
	let c2p_client = ClientWrapper(Rc::new(Client::new(channel_name_c2p(), Duration::from_millis(500)).unwrap()));

	rsx! {
		div {
			id: "control_panel",
			div {
				ItemList {
					items: registers.items(),
					path: "",
					c2p_client,
				}
			}
		}
	}
}
