fn main() {
	// main プロジェクト（moddl）のバイナリパスを取得
	let metadata = cargo_metadata::MetadataCommand::new()
			.exec()
			.expect("Failed to get cargo metadata");

	let target = metadata.packages
			.iter()
			.filter(|package| package.name == "moddl")
			.flat_map(|package| package.targets.iter())
			.find(|target| target.kind.iter().any(|k| k == "bin"))
			.expect("Failed to find moddl binary target");
	let bin_path = metadata.target_directory.join("debug").join(format!("{}.exe", target.name));
	println!("cargo:rustc-env=MODDL_BIN_PATH={}", bin_path);
	println!("cargo:rustc-env=MODDL_BIN_NAME={}", target.name);
}
