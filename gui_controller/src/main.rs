use std::{path::PathBuf, process::Command, rc::Rc, thread, time::Duration};

use dioxus::{logger::tracing, prelude::*};
use ipc::{Client, DomainHint, RegisterSettings, Response, Server, Set, channel_name_c2p, channel_name_p2c, decode, to_bson};

use crate::view_model::{RegisterSettingsItemStore, RegisterSettingsItemStoreStoreExt, RegisterSettingsItemsStore, RegisterSettingsStore, RegisterSettingsStoreStoreExt};

mod view_model;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const HEADER_SVG: Asset = asset!("/assets/header.svg");

// IPC スレッドが UI スレッドでの処理完了を待つ上限。
// UI スレッドが固まっていても IPC スレッドが永久にブロックしないようにするための保険。
const UI_REPLY_TIMEOUT: Duration = Duration::from_millis(500);

fn get_player_path() -> anyhow::Result<PathBuf> {
	// dx serve のように gui_controller.exe が moddl と別ディレクトリにバンドルされる
	// 開発時は、環境変数 MODDL_EXE_PATH で moddl の実行ファイルの場所を明示できる
	// （VSCode の launch.json/tasks.json の env で開発者ごとに設定する想定）。
	// ビルド時に絶対パスを埋め込むと開発機固有の情報が配布物に残ってしまうため、
	// あくまで実行時にのみ参照する。
	if let Ok(path) = std::env::var("MODDL_EXE_PATH") {
		return Ok(PathBuf::from(path));
	}

	// 指定がなければ、配布用の zip 一式と同様に gui_controller.exe 自身と同じ
	// ディレクトリを探す。portaudio_x64.dll や builtins/root.moddl も moddl と
	// 同じディレクトリから探されるため、その一式が揃っている前提。
	let bin_name = env!("MODDL_BIN_NAME");
	let exe_name = format!("{}{}", bin_name, std::env::consts::EXE_SUFFIX);
	let current_exe = std::env::current_exe()?;
	current_exe.parent()
			.map(|dir| dir.join(exe_name))
			.ok_or_else(|| anyhow::Error::msg("cannot locate player executable"))
}

fn main() -> anyhow::Result<()> {
	let player_path = get_player_path() ?;
	println!("Executing {}", player_path.to_str().unwrap_or("(path not displayable)"));

	// main プロジェクトのバイナリを実行
	let mut cmd = Command::new(player_path);
	
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
		// IPC スレッドから UI スレッドへメッセージを転送するチャネル。
		// IPC スレッドが処理結果を反映したレスポンスを返せるよう、メッセージには
		// UI スレッドでの処理結果を送り返すための reply チャネルを同梱する。
		// UI スレッド側は async に recv() で待つのでポーリング不要（IPC スレッドからの
		// send は同期呼び出しなので、送信側を async 化する必要はない）。
		let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(bson::Document, std::sync::mpsc::Sender<Response>)>();
		thread::spawn(move || {
			// TODO ちゃんとエラー処理
			let p2c_server = Server::new(channel_name_p2c()).unwrap();
			loop {
				p2c_server.wait_for_request(|request_bytes| {
					let bson = to_bson(request_bytes) ?;
					let (reply_tx, reply_rx) = std::sync::mpsc::channel::<Response>();
					tx.send((bson, reply_tx)) ?;
					// UI スレッドでの処理完了（＝reply_tx への送信）を待ち、
					// その結果をそのままレスポンスとして返す。
					let response = reply_rx.recv_timeout(UI_REPLY_TIMEOUT).unwrap_or_else(|_| Response {
						status: "error".into(),
						text: "timed out waiting for UI thread".into(),
					});
					Ok(bson::serialize_to_vec(&response).unwrap())
				}).unwrap_or_else(|e| println!("{}", e));
			}
		});

		while let Some((bson, reply_tx)) = rx.recv().await {
			let response = if let Some(settings) = decode::<RegisterSettings>(&bson) {
				dbg!(&settings);
				let store = RegisterSettingsStore::from(settings);
				let mut items_ = registers.items();
				let mut items = items_.write();
				items.clear();
				items.extend(store.items);

				Response { status: "ok".into(), text: "applied".into() }
			// 種別が増えたら分岐を追加
			// } else if let Some(<message>) = decode::<<MessageType>>(&bson) {
			} else {
				Response { status: "error".into(), text: format!("unsupported message: {}", &bson) }
			};

			// IPC スレッドが待ちきれずタイムアウトした後は受け手がいなくなるので、
			// 送信失敗は無視してよい
			let _ = reply_tx.send(response);
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
fn RegisterControl(path: String, keey: String, init: f32, min: f32, max: f32, label: String, c2p_client: ClientWrapper) -> Element {
	let mut value = use_signal(|| init);

	let set_value = || {
		let c2p_client = c2p_client.clone();
		let target = path.clone();
		let key = keey.clone();
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
			label { "{label}" }
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
					let (min, max) = match domain {
						Some(DomainHint::Range { min, includes_min, max, includes_max }) => {
							let (min, max) = if min <= max { (*min, *max) } else { (*max, *min) };
							let delta = 0.1f32.min((max - min) * 0.001f32);
							let min = if *includes_min { min } else { min + delta };
							let max = if *includes_max { max } else { max - delta };

							(min, max)
						},

						// TODO Enum 対応
						Some(DomainHint::Enum { .. }) => (-100f32, 100f32),

						// TODO 定義域がない場合にちゃんと対応（いっそスライダーを出さないとか？）
						None => (-100f32, 100f32),
					};
					(key.clone(), *initial, min, max)
				},
				RegisterSettingsItemStore::Group(_) => unreachable!(),
			}
		};

		rsx! {
			RegisterControl {
				path,
				keey: key,
				init: initial,
				min, max,
				label: name,
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
