use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
	match run() {
		Ok(()) => ExitCode::SUCCESS,
		Err(e) => {
			eprintln!("xtask: error: {e}");
			ExitCode::FAILURE
		}
	}
}

struct Args {
	profile: String,
	out_dir: PathBuf,
	// --out-dir が明示されていない（= dist/<profile> という既定値の）場合だけ true。
	// 素性の分からないユーザー指定ディレクトリを無条件に削除するのは危険なため、
	// まるごと削除してよいのはビルドツールが自分で作る既定の出力先だけに限る。
	clean_out_dir: bool,
}

fn run() -> Result<(), String> {
	let root = workspace_root();
	let args = parse_args(&root)?;

	build_main(&root, &args.profile)?;
	build_gui(&root, &args.profile)?;

	// 古い成果物が混ざらないよう、既定の出力先（dist/<profile>）の場合だけ
	// まるごと削除してから作り直す（他の profile の出力先や、--out-dir で明示された
	// 既存ディレクトリの中身には触れない）
	if args.clean_out_dir && args.out_dir.exists() {
		fs::remove_dir_all(&args.out_dir)
			.map_err(|e| format!("failed to remove existing output dir {}: {e}", args.out_dir.display()))?;
	}

	fs::create_dir_all(&args.out_dir)
		.map_err(|e| format!("failed to create output dir {}: {e}", args.out_dir.display()))?;
	fs::create_dir_all(args.out_dir.join("builtins"))
		.map_err(|e| format!("failed to create builtins dir: {e}"))?;

	copy_file(
		&root.join("target").join(&args.profile).join("moddl.exe"),
		&args.out_dir.join("moddl.exe"),
	)?;

	copy_gui_app(&root, &args.profile, &args.out_dir)?;

	copy_file(
		&root.join("main/res/portaudio_x64.dll"),
		&args.out_dir.join("portaudio_x64.dll"),
	)?;
	copy_file(
		&root.join("main/res/builtins/root.moddl"),
		&args.out_dir.join("builtins/root.moddl"),
	)?;

	println!("xtask: build artifacts placed in {}", args.out_dir.display());
	Ok(())
}

fn workspace_root() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.expect("xtask crate must live directly under the workspace root")
		.to_path_buf()
}

fn parse_args(root: &Path) -> Result<Args, String> {
	let mut profile = "release".to_string();
	let mut out_dir: Option<PathBuf> = None;

	let mut iter = env::args().skip(1).peekable();
	// Allow (and ignore) a leading task name, e.g. `cargo xtask build`.
	if let Some(first) = iter.peek() {
		if first == "build" {
			iter.next();
		}
	}

	while let Some(arg) = iter.next() {
		match arg.as_str() {
			"--profile" => {
				profile = iter.next().ok_or("--profile requires a value (debug|release)")?;
			}
			"--out-dir" => {
				let value = iter.next().ok_or("--out-dir requires a path")?;
				out_dir = Some(PathBuf::from(value));
			}
			other => return Err(format!("unknown argument: {other}")),
		}
	}

	if profile != "debug" && profile != "release" {
		return Err(format!("--profile must be 'debug' or 'release', got '{profile}'"));
	}

	// デバッグビルドとリリースビルドの成果物を取り違えないよう、既定の出力先は
	// profile ごとのサブディレクトリに分ける。--out-dir を明示した場合は、
	// profile のサブディレクトリを挟まずそのディレクトリに直接出力する。
	let clean_out_dir = out_dir.is_none();
	let out_dir = out_dir.unwrap_or_else(|| root.join("dist").join(&profile));

	Ok(Args { profile, out_dir, clean_out_dir })
}

fn build_main(root: &Path, profile: &str) -> Result<(), String> {
	let mut cmd = Command::new("cargo");
	cmd.current_dir(root).arg("build").arg("--package").arg("moddl");
	if profile == "release" {
		cmd.arg("--release");
	}
	run_command(cmd, "cargo build (moddl)")
}

fn build_gui(root: &Path, profile: &str) -> Result<(), String> {
	let mut cmd = Command::new("dx");
	cmd.current_dir(root.join("gui_controller")).arg("build");
	if profile == "release" {
		cmd.arg("--release");
	}
	run_command(cmd, "dx build (gui_controller)")
}

fn run_command(mut cmd: Command, description: &str) -> Result<(), String> {
	println!("xtask: running {description}: {cmd:?}");
	let status = cmd
		.status()
		.map_err(|e| format!("failed to launch {description}: {e}"))?;
	if !status.success() {
		return Err(format!("{description} failed with {status}"));
	}
	Ok(())
}

fn copy_gui_app(root: &Path, profile: &str, out_dir: &Path) -> Result<(), String> {
	let app_dir = root
		.join("target/dx/gui_controller")
		.join(profile)
		.join("windows/app");
	let exe_name = "gui_controller.exe";

	copy_file(&app_dir.join(exe_name), &out_dir.join(exe_name))?;

	// dioxus desktop loads bundled assets from an `assets/` folder next to the exe.
	let assets_dir = app_dir.join("assets");
	if assets_dir.is_dir() {
		copy_dir_recursive(&assets_dir, &out_dir.join("assets"))?;
	}
	Ok(())
}

fn copy_file(from: &Path, to: &Path) -> Result<(), String> {
	fs::copy(from, to)
		.map(|_| ())
		.map_err(|e| format!("failed to copy {} -> {}: {e}", from.display(), to.display()))
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<(), String> {
	fs::create_dir_all(to).map_err(|e| format!("failed to create dir {}: {e}", to.display()))?;
	let entries =
		fs::read_dir(from).map_err(|e| format!("failed to read dir {}: {e}", from.display()))?;
	for entry in entries {
		let entry =
			entry.map_err(|e| format!("failed to read entry in {}: {e}", from.display()))?;
		let file_type = entry
			.file_type()
			.map_err(|e| format!("failed to stat {}: {e}", entry.path().display()))?;
		let dest = to.join(entry.file_name());
		if file_type.is_dir() {
			copy_dir_recursive(&entry.path(), &dest)?;
		} else {
			copy_file(&entry.path(), &dest)?;
		}
	}
	Ok(())
}
