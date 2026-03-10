use std::process::Command;

fn main() {
	// build.rs で設定された環境変数からバイナリパスを取得
	let bin_path = env!("MODDL_BIN_PATH");
	let bin_name = env!("MODDL_BIN_NAME");
	
	println!("Executing: {} ({})", bin_name, bin_path);
	
	// main プロジェクトのバイナリを実行
	let mut cmd = Command::new(bin_path);
	
	// コマンドライン引数をそのまま渡す場合
	cmd.args(std::env::args().skip(1));
	
	let status = cmd.status()
		.expect("Failed to execute main binary");
	
	std::process::exit(status.code().unwrap_or(1));
}
